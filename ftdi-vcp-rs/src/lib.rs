use ftdi_vcp_sys::{
    FT_Close, FT_GetComPortNumber, FT_OpenEx, FT_HANDLE, FT_OPEN_BY_DESCRIPTION, FT_STATUS, LONG,
    PVOID,
};
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::convert::TryInto;

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
    UnknownError(FT_STATUS),
}

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

pub struct VCP {
    handle: FT_HANDLE,
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
            Err(result)
        } else {
            Ok(VCP {
                handle: unsafe { handle.assume_init() },
            })
        }
    }

    pub fn com_port(&self) -> Result<usize, Error> {
        let mut com_port_number = MaybeUninit::<LONG>::uninit();
        let result = Error::from(unsafe {
            FT_GetComPortNumber(self.handle, com_port_number.as_mut_ptr())
        });
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
}

impl Drop for VCP {
    fn drop(&mut self) {
        let result = Error::from(unsafe { FT_Close(self.handle) });
        if result != Error::NoError {
            panic!("unable to close device: {:?}", result);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
