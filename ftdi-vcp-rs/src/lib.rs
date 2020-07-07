use ftdi_vcp_sys::{
    get_device_info_list, FT_Close, FT_GetBitMode, FT_GetComPortNumber, FT_GetLatencyTimer,
    FT_Open, FT_OpenEx, FT_Purge, FT_Read, FT_ResetDevice, FT_SetBitMode, FT_SetLatencyTimer,
    FT_Write, DWORD, FT_HANDLE, FT_OPEN_BY_DESCRIPTION, FT_STATUS, LONG, LPDWORD, LPVOID, PVOID,
    UCHAR,
};
use std::convert::TryInto;
use std::ffi::CString;
use std::io::{Read, Write};
use std::mem::MaybeUninit;

pub mod mpsse;

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

    /// The provided string contained at least one NULL byte
    StringContainsNullByte,

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

#[derive(Copy, Clone)]
pub enum Interface {
    A,
    B,
    C,
    D,
}

impl VCP {
    pub fn new_from_name(name: &str) -> Result<VCP, Error> {
        let c_str = CString::new(name).or(Err(Error::StringContainsNullByte))?;
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

    pub fn new_from_vid_pid(
        vid: u16,
        pid: u16,
        interface: Option<Interface>,
    ) -> Result<VCP, Error> {
        let target_id = (vid as u32) << 16 | pid as u32;

        let mut discovered_idx = None;
        for (idx, entry) in get_device_info_list()
            .map_err(|e| Error::from(e as FT_STATUS))?
            .iter()
            .enumerate()
        {
            // println!("Device {}: {:08x}/{:08x}", idx, entry.ID, entry.LocId);
            if entry.ID == target_id
                && match interface {
                    Some(Interface::A) => entry.LocId & 0xf == 0x1,
                    Some(Interface::B) => entry.LocId & 0xf == 0x2,
                    Some(Interface::C) => entry.LocId & 0xf == 0x3,
                    Some(Interface::D) => entry.LocId & 0xf == 0x4,
                    None => true,
                }
            {
                discovered_idx = Some(idx);
                break;
            }
        }

        if discovered_idx.is_none() {
            return Err(Error::DeviceNotFound);
        }
        let discovered_idx = discovered_idx.unwrap();

        let mut handle = MaybeUninit::<FT_HANDLE>::uninit();
        let result = Error::from(unsafe { FT_Open(discovered_idx as _, handle.as_mut_ptr()) });
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
    pub fn set_bitmode(&mut self, outputs: u8, bitmode: BitMode) -> Result<(), Error> {
        let result = Error::from(unsafe { FT_SetBitMode(self.handle, outputs, bitmode.to_u8()) });
        if result != Error::NoError {
            Err(result)
        } else {
            self.bit_mode = bitmode;
            Ok(())
        }
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        let result = Error::from(unsafe { FT_ResetDevice(self.handle) });
        if result != Error::NoError {
            Err(result)
        } else {
            Ok(())
        }
    }

    pub fn purge(&mut self) -> Result<(), Error> {
        let result = Error::from(unsafe {
            FT_Purge(
                self.handle,
                ftdi_vcp_sys::FT_PURGE_RX | ftdi_vcp_sys::FT_PURGE_TX,
            )
        });
        if result != Error::NoError {
            Err(result)
        } else {
            Ok(())
        }
    }

    pub fn latency_timer(&mut self) -> Result<u8, Error> {
        let mut latency = MaybeUninit::<UCHAR>::uninit();
        let result = Error::from(unsafe { FT_GetLatencyTimer(self.handle, latency.as_mut_ptr()) });
        if result != Error::NoError {
            Err(result)
        } else {
            Ok(unsafe { latency.assume_init() })
        }
    }

    pub fn set_latency_timer(&mut self, latency: u8) -> Result<(), Error> {
        let result = Error::from(unsafe { FT_SetLatencyTimer(self.handle, latency) });
        if result != Error::NoError {
            Err(result)
        } else {
            Ok(())
        }
    }

    // pub fn write(&mut self, out_buffer: &[u8]) -> Result<usize, Error> {
    //     let mut bytes_written = MaybeUninit::<DWORD>::uninit();
    //     let result = Error::from(unsafe {
    //         FT_Write(
    //             self.handle,
    //             out_buffer.as_ptr() as LPVOID,
    //             out_buffer
    //                 .len()
    //                 .try_into()
    //                 .expect("couldn't convert buffer length to DWORD"),
    //             bytes_written.as_mut_ptr() as LPDWORD,
    //         )
    //     });
    //     if result != Error::NoError {
    //         Err(result)
    //     } else {
    //         Ok(unsafe { bytes_written.assume_init() }
    //             .try_into()
    //             .expect("invalid number of bytes written"))
    //     }
    // }

    pub fn set_gpio(&mut self, value: u8, direction: u8) -> Result<(), Error> {
        match self.bit_mode {
            BitMode::MPSSE => self
                .write_all(&[mpsse::Command::MC_SETB_LOW.to_u8(), value, direction])
                .or_else(|_| Err(Error::IoError)),
            _ => unimplemented!(),
        }
    }

    pub fn readb_low(&mut self) -> Result<u8, Error> {
        self.write_all(&[mpsse::Command::MC_READB_LOW.to_u8()])
            .or_else(|_| Err(Error::IoError))?;
        let mut result = [0; 1];
        self.read(&mut result).or_else(|_| Err(Error::IoError))?;
        Ok(result[0])
    }

    pub fn readb_high(&mut self) -> Result<u8, Error> {
        self.write_all(&[mpsse::Command::MC_READB_HIGH.to_u8()])
            .or_else(|_| Err(Error::IoError))?;
        let mut result = [0; 1];
        self.read(&mut result).or_else(|_| Err(Error::IoError))?;
        Ok(result[0])
    }

    /// The purpose of this function is unclear.  It appears to send some number
    /// of bits out the line.
    pub fn xfer_spi_bits(&mut self, data: u8, bits: usize) -> Result<u8, Error> {
        if bits < 1 {
            return Ok(0);
        }

        let buffer = &[
            /* Input and output, update data on negative edge read on positive, bits. */
            0x20 /*MC_DATA_IN*/ | 0x10 /*MC_DATA_OUT*/ | 0x01 /*MC_DATA_OCN*/ | 0x02, /*MC_DATA_BITS*/
            bits as u8 - 1,
            data,
        ];
        self.write_all(buffer).or_else(|_| Err(Error::IoError))?;

        let mut return_val = [0; 1];
        self.read_exact(&mut return_val)
            .or_else(|_| Err(Error::IoError))?;
        Ok(return_val[0])
    }

    pub fn xfer_spi(&mut self, data: &mut [u8]) -> Result<(), Error> {
        if data.is_empty() {
            return Ok(());
        }

        /* Input and output, update data on negative edge read on positive. */
        let buffer = &[
            0x20 /*MC_DATA_IN*/ | 0x10 /*MC_DATA_OUT*/ | 0x01, /*MC_DATA_OCN*/
            (data.len() - 1) as u8,
            ((data.len() - 1) / 256) as u8,
        ];
        self.write_all(buffer).or_else(|_| Err(Error::IoError))?;
        self.write_all(data).or_else(|_| Err(Error::IoError))?;

        self.read_exact(data).or_else(|_| Err(Error::IoError))?;
        Ok(())
    }

    pub fn send_spi(&mut self, data: &[u8]) -> Result<(), Error> {
        if data.is_empty() {
            return Ok(());
        }

        /* Input and output, update data on negative edge read on positive. */
        let buffer = &[
            0x10 /*MC_DATA_OUT*/ | 0x01, /*MC_DATA_OCN*/
            (data.len() - 1) as u8,
            ((data.len() - 1) / 256) as u8,
        ];
        self.write_all(buffer).or_else(|_| Err(Error::IoError))?;
        self.write_all(data).or_else(|_| Err(Error::IoError))?;
        Ok(())
    }

    pub fn send_dummy_bytes(&mut self, n: u8) -> Result<(), Error> {
        // add 8 x count dummy bits (aka n bytes)
        self.write_all(&[mpsse::Command::MC_CLK_N8.to_u8(), n - 1, 0x00])
            .or_else(|_| Err(Error::IoError))
    }

    pub fn send_dummy_bit(&mut self) -> Result<(), Error> {
        // add 1  dummy bit
        self.write_all(&[mpsse::Command::MC_CLK_N.to_u8(), 0x00])
            .or_else(|_| Err(Error::IoError))
    }
}

impl Write for VCP {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut bytes_written = MaybeUninit::<DWORD>::uninit();
        let result = Error::from(unsafe {
            FT_Write(
                self.handle,
                buf.as_ptr() as LPVOID,
                buf.len()
                    .try_into()
                    .expect("couldn't convert buffer length to DWORD"),
                bytes_written.as_mut_ptr() as LPDWORD,
            )
        });
        if result != Error::NoError {
            Err(std::io::Error::new(std::io::ErrorKind::Other, result))
        } else {
            let bytes_written = unsafe { bytes_written.assume_init() };
            // println!(
            //     "Wrote {} bytes (wanted to write {})",
            //     bytes_written,
            //     buf.len()
            // );
            Ok(bytes_written
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
        let mut bytes_read = MaybeUninit::<DWORD>::uninit();
        let result = Error::from(unsafe {
            FT_Read(
                self.handle,
                buf.as_mut_ptr() as LPVOID,
                buf.len()
                    .try_into()
                    .expect("couldn't convert buffer length to DWORD"),
                bytes_read.as_mut_ptr() as LPDWORD,
            )
        });
        if result != Error::NoError {
            Err(std::io::Error::new(std::io::ErrorKind::Other, result))
        } else {
            let bytes_read = unsafe { bytes_read.assume_init() };
            // println!("Read {} bytes (wanted to read {})", bytes_read, buf.len());
            Ok(bytes_read
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
