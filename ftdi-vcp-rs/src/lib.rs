use ftdi_vcp_sys::{
    FT_Close, FT_GetBitMode, FT_GetComPortNumber, FT_OpenEx, FT_SetBitMode, FT_Write, DWORD,
    FT_HANDLE, FT_OPEN_BY_DESCRIPTION, FT_STATUS, LONG, LPDWORD, LPVOID, PVOID, UCHAR,
};
use std::convert::TryInto;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::io::{Read, Write};

#[derive(Debug)]
pub enum BitMode {
    Reset,
    AsyncBitbang,
    MPSSE,
    SyncBitbang,
    MCUHost,
    FastSerial,
    CBUSBitbang,
    SyncFIFO,
    Unknown(u8),
}

impl From<UCHAR> for BitMode {
    fn from(src: UCHAR) -> Self {
        use BitMode::*;
        match src {
            0x00 => Reset,
            0x01 => AsyncBitbang,
            0x02 => MPSSE,
            0x04 => SyncBitbang,
            0x08 => MCUHost,
            0x10 => FastSerial,
            0x20 => CBUSBitbang,
            0x40 => SyncFIFO,
            x => Unknown(x),
        }
    }
}

impl BitMode {
    pub fn to_u8(&self) -> UCHAR {
        use BitMode::*;
        match *self {
            Reset => 0x00,
            AsyncBitbang => 0x01,
            MPSSE => 0x02,
            SyncBitbang => 0x04,
            MCUHost => 0x08,
            FastSerial => 0x10,
            CBUSBitbang => 0x20,
            SyncFIFO => 0x40,
            Unknown(x) => x,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Error {
    NoError,
    InvalidHandle,
    DeviceNotFound,
    DeviceNotOpened,
    IoError,
    InsufficientResources,
    InvalidParameter,
    InvalidBaudRate,
    DeviceNotOpenedForErase,
    DeviceNotOpenedForWrite,
    FailedToWriteDevice,
    EepromReadFailed,
    EepromWriteFailed,
    EepromEraseFailed,
    EepromNotPresent,
    EepromNotProgrammed,
    InvalidArgs,
    NotSupported,
    OtherError,
    DeviceListNotReady,
    NoComPortAssigned,
    // UnknownBitMode(u8),
    UnknownError(FT_STATUS),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VCPError: {:?}", self)
    }
}
impl std::error::Error for Error {}

impl From<FT_STATUS> for Error {
    fn from(src: FT_STATUS) -> Self {
        match src {
            0 => Error::NoError,
            1 => Error::InvalidHandle,
            2 => Error::DeviceNotFound,
            3 => Error::DeviceNotOpened,
            4 => Error::IoError,
            5 => Error::InsufficientResources,
            6 => Error::InvalidParameter,
            7 => Error::InvalidBaudRate,
            8 => Error::DeviceNotOpenedForErase,
            9 => Error::DeviceNotOpenedForWrite,
            10 => Error::FailedToWriteDevice,
            11 => Error::EepromReadFailed,
            12 => Error::EepromWriteFailed,
            13 => Error::EepromEraseFailed,
            14 => Error::EepromNotPresent,
            15 => Error::EepromNotProgrammed,
            16 => Error::InvalidArgs,
            17 => Error::NotSupported,
            18 => Error::OtherError,
            19 => Error::DeviceListNotReady,
            x => Error::UnknownError(x),
        }
    }
}

#[derive(Debug)]
pub struct VCP {
    handle: FT_HANDLE,
    bit_mode: BitMode,
}

impl VCP {
    pub fn new_from_name(name: &str) -> Result<VCP, Error> {
        let c_str = CString::new(name).expect("string already contained null bytes");
        let mut handle = MaybeUninit::<FT_HANDLE>::uninit();
        let result = Error::from(unsafe {
            FT_OpenEx(
                c_str.as_ptr() as PVOID,
                FT_OPEN_BY_DESCRIPTION,
                handle.as_mut_ptr(),
            )
        });
        if result != Error::NoError {
            return Err(result);
        }
        let handle = unsafe { handle.assume_init() };

        let mut bit_mode = MaybeUninit::<UCHAR>::uninit();
        let result = Error::from(unsafe { FT_GetBitMode(handle, bit_mode.as_mut_ptr()) });
        if result != Error::NoError {
            unsafe { FT_Close(handle) };
            return Err(result);
        }
        let bit_mode = BitMode::from(unsafe { bit_mode.assume_init() });
        Ok(VCP { handle, bit_mode })
    }

    pub fn com_port(&self) -> Result<usize, Error> {
        let mut com_port_number = MaybeUninit::<LONG>::uninit();
        let result =
            Error::from(unsafe { FT_GetComPortNumber(self.handle, com_port_number.as_mut_ptr()) });
        if result != Error::NoError {
            Err(result)
        } else {
            let com_port_number = unsafe { com_port_number.assume_init() };
            if let Ok(e) = com_port_number.try_into() {
                Ok(e)
            } else {
                Err(Error::NoComPortAssigned)
            }
        }
    }

    /// Set the given signals to "OUTPUT".  All other signals will be "INPUT".
    pub fn set_bit_mode(&mut self, outputs: u8) -> Result<(), Error> {
        let result = Error::from(unsafe {
            FT_SetBitMode(self.handle, outputs, BitMode::SyncBitbang.to_u8())
        });
        if result != Error::NoError {
            Err(result)
        } else {
            Ok(())
        }
    }

    pub fn write(&mut self, out_buffer: &[u8]) -> Result<usize, Error> {
        let mut bytes_written = MaybeUninit::<DWORD>::uninit();
        let result = Error::from(unsafe {
            FT_Write(
                self.handle,
                out_buffer.as_ptr() as LPVOID,
                out_buffer
                    .len()
                    .try_into()
                    .expect("couldn't convert buffer length to DWORD"),
                bytes_written.as_mut_ptr() as LPDWORD,
            )
        });
        if result != Error::NoError {
            Err(result)
        } else {
            Ok(unsafe { bytes_written.assume_init() }
                .try_into()
                .expect("invalid number of bytes written"))
        }
    }
}

impl Write for VCP {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut bytes_written = MaybeUninit::<DWORD>::uninit();
        let result = Error::from(unsafe {
            FT_Write(
                self.handle,
                buf.as_ptr() as LPVOID,
                buf
                    .len()
                    .try_into()
                    .expect("couldn't convert buffer length to DWORD"),
                bytes_written.as_mut_ptr() as LPDWORD,
            )
        });
        if result != Error::NoError {
            Err(std::io::Error::new(std::io::ErrorKind::Other, result))
        } else {
            Ok(unsafe { bytes_written.assume_init() }
                .try_into()
                .expect("invalid number of bytes written"))
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Read for VCP {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut bytes_written = MaybeUninit::<DWORD>::uninit();
        let result = Error::from(unsafe {
            FT_Write(
                self.handle,
                buf.as_ptr() as LPVOID,
                1,
                bytes_written.as_mut_ptr() as LPDWORD,
            )
        });
        if result != Error::NoError {
            Err(std::io::Error::new(std::io::ErrorKind::Other, result))
        } else {
            Ok(unsafe { bytes_written.assume_init() }
                .try_into()
                .expect("invalid number of bytes written"))
        }
    }
}

impl Drop for VCP {
    fn drop(&mut self) {
        let result = Error::from(unsafe { FT_Close(self.handle) });
        if result != Error::NoError {
            panic!("unable to close device: {:?}", result);
        }
    }
}
