#![allow(non_camel_case_types, non_snake_case)]

extern crate winapi;
// use winapi::um::winnt::{PVOID, ULONG, DWORD};
pub use winapi::shared::minwindef::{DWORD, LPDWORD, LPLONG, LPVOID, UCHAR, PUCHAR};
pub use winapi::shared::ntdef::{LONG, PVOID, ULONG};

#[allow(non_camel_case_types)]
pub type FT_HANDLE = PVOID;
#[allow(non_camel_case_types)]
pub type FT_STATUS = ULONG;

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone)]
pub struct FT_DEVICE_LIST_INFO_NODE {
    Flags: ULONG,
    Type: ULONG,
    ID: ULONG,
    LocId: DWORD,
    SerialNumber: [u8; 16],
    Description: [u8; 64],
    ftHandle: FT_HANDLE,
}

#[repr(C)]
pub enum FtStatus {
    FT_OK = 0,
    FT_INVALID_HANDLE = 1,
    FT_DEVICE_NOT_FOUND = 2,
    FT_DEVICE_NOT_OPENED = 3,
    FT_IO_ERROR = 4,
    FT_INSUFFICIENT_RESOURCES = 5,
    FT_INVALID_PARAMETER = 6,
    FT_INVALID_BAUD_RATE = 7,

    FT_DEVICE_NOT_OPENED_FOR_ERASE = 8,
    FT_DEVICE_NOT_OPENED_FOR_WRITE = 9,
    FT_FAILED_TO_WRITE_DEVICE = 10,
    FT_EEPROM_READ_FAILED = 11,
    FT_EEPROM_WRITE_FAILED = 12,
    FT_EEPROM_ERASE_FAILED = 13,
    FT_EEPROM_NOT_PRESENT = 14,
    FT_EEPROM_NOT_PROGRAMMED = 15,
    FT_INVALID_ARGS = 16,
    FT_NOT_SUPPORTED = 17,
    FT_OTHER_ERROR = 18,
    FT_DEVICE_LIST_NOT_READY = 19,
}

// // Device information flags
// enum FT_FLAGS {
//     FT_FLAGS_OPENED = 1,
//     FT_FLAGS_HISPEED = 2,
// }

pub const FT_OPEN_BY_SERIAL_NUMBER: DWORD = 1;
pub const FT_OPEN_BY_DESCRIPTION: DWORD = 2;
pub const FT_OPEN_BY_LOCATION: DWORD = 4;

pub const FT_LIST_NUMBER_ONLY: DWORD = 0x80000000;
pub const FT_LIST_BY_INDEX: DWORD = 0x40000000;
pub const FT_LIST_ALL: DWORD = 0x20000000;

pub const FT_LIST_MASK: DWORD = (FT_LIST_NUMBER_ONLY | FT_LIST_BY_INDEX | FT_LIST_ALL);

#[link(name = "ftd2xx")]
#[allow(non_snake_case)]
#[allow(dead_code)]
extern "stdcall" {
    pub fn FT_ListDevices(pArg1: PVOID, pArg2: PVOID, Flags: DWORD) -> FT_STATUS;
    pub fn FT_CreateDeviceInfoList(lpdwNumDevs: LPDWORD) -> FT_STATUS;
    pub fn FT_GetDeviceInfoList(
        pDest: *mut FT_DEVICE_LIST_INFO_NODE,
        lpdwNumDevs: LPDWORD,
    ) -> FT_STATUS;
    pub fn FT_GetDeviceInfoDetail(
        dwIndex: DWORD,
        lpdwFlags: LPDWORD,
        lpdwType: LPDWORD,
        lpdwID: LPDWORD,
        lpdwLocId: LPDWORD,
        lpSerialNumber: LPVOID,
        lpDescription: LPVOID,
        pftHandle: *mut FT_HANDLE,
    ) -> FT_STATUS;
    pub fn FT_GetComPortNumber(ftHandle: FT_HANDLE, lpdwComPortNumber: LPLONG) -> FT_STATUS;
    pub fn FT_Open(deviceNumber: u32, pHandle: *mut FT_HANDLE) -> FT_STATUS;
    pub fn FT_OpenEx(pArg1: PVOID, Flags: DWORD, pHandle: *mut FT_HANDLE) -> FT_STATUS;
    pub fn FT_Close(ftHandle: FT_HANDLE) -> FT_STATUS;
    pub fn FT_Write(
        ftHandle: FT_HANDLE,
        lpBuffer: LPVOID,
        dwBytesToWrite: DWORD,
        lpBytesWritten: LPDWORD,
    ) -> FT_STATUS;
    pub fn FT_Read(
        ftHandle: FT_HANDLE,
        lpBuffer: LPVOID,
        dwBytesToRead: DWORD,
        lpBytesReturned: LPDWORD,
    ) -> FT_STATUS;
    pub fn FT_SetBitMode(ftHandle: FT_HANDLE, ucMask: UCHAR, ucEnable: UCHAR) -> FT_STATUS;
    pub fn FT_GetBitMode(ftHandle: FT_HANDLE, pucMode: PUCHAR) -> FT_STATUS;
}

pub fn create_device_info_list() -> Result<usize, usize> {
    let device_count: DWORD = 0;
    let result = unsafe { FT_CreateDeviceInfoList(&device_count as *const _ as LPDWORD) };
    if result == 0 {
        Ok(device_count as usize)
    } else {
        Err(result as usize)
    }
}

pub fn get_device_info_list() -> Result<Vec<FT_DEVICE_LIST_INFO_NODE>, usize> {
    let mut dev_info = vec![];

    let device_count = create_device_info_list()?;
    dev_info.resize(
        device_count as usize,
        FT_DEVICE_LIST_INFO_NODE {
            Flags: 0,
            Type: 0,
            ID: 0,
            LocId: 0,
            SerialNumber: [0; 16],
            Description: [0; 64],
            ftHandle: 0 as FT_HANDLE,
        },
    );
    let result = unsafe {
        FT_GetDeviceInfoList(dev_info.as_mut_ptr(), &device_count as *const _ as LPDWORD)
    };
    if result != 0 {
        return Err(result as usize);
    }
    Ok(dev_info)
}

#[cfg(test)]
mod tests {
    use crate::*;
    use std::ffi::CStr;
    #[test]
    fn list_devices() {
        println!("Listing devices...");
        let device_count = create_device_info_list().expect("couldn't list devices");
        // let result = unsafe { FT_ListDevices(&device_count as *const i32 as PVOID, 0 as PVOID, FT_LIST_NUMBER_ONLY) };
        println!("There are {} devices", device_count);

        let dev_info = get_device_info_list().expect("couldn't get device info list");
        for item in dev_info {
            println!("Flags: {:08x}", item.Flags);
            println!("Type: {:08x}", item.Type);
            println!("ID: {:08x}", item.ID);
            println!("LocId: {:08x}", item.LocId);
            println!("SerialNumber: {:?}", item.SerialNumber);
            // let description = CStr::from_bytes_with_nul(&item.Description).expect("couldn't parse item string");
            let p = &item.Description as *const u8 as *const i8;
            let description = unsafe { CStr::from_ptr(p) };
            println!("Description: {}", description.to_string_lossy()); //String::from_utf8_lossy(&item.Description));
            print!("             ");
            for ch in item.Description.iter() {
                print!(" {:02x}", ch);
            }
            println!("");
            println!("ftHandle: {:08x}", item.ftHandle as usize);
        }
    }

    #[test]
    fn open_device() {
        let desc = b"iCEBreaker V1.0e B\0";
        let mut handle = 0 as FT_HANDLE;
        let result = unsafe {
            FT_OpenEx(
                desc.as_ptr() as PVOID,
                FT_OPEN_BY_DESCRIPTION,
                (&mut handle) as *mut FT_HANDLE,
            )
        };
        println!("Result: {}", result);
        if result != 0 {
            panic!("couldn't open handle: result not 0");
        }

        let mut com_port_number: LONG = 0;
        let result = unsafe { FT_GetComPortNumber(handle, &mut com_port_number) };
        if result != 0 {
            panic!("couldn't get com port number");
        }
        println!("Device opened on COM{}:", com_port_number);

        let result = unsafe { FT_Close(handle) };
        println!("Result: {}", result);
        if result != 0 {
            panic!("couldn't close handle: result not 0");
        }
    }
}
