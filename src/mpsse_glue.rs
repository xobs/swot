use ftdi_vcp_rs::{mpsse::Command::*, BitMode, Interface, VCP};
use std::cell::RefCell;
use std::ffi::CStr;
use std::io::Write;
use std::os::raw::{c_char, c_int};
use std::slice;

thread_local! {
    pub static COM_PORT: RefCell<Option<VCP>> = RefCell::new(None);
}

#[no_mangle]
pub extern "C" fn mpsse_error(status: c_int) {
    panic!("mpsse_error({})", status);
}

#[no_mangle]
pub extern "C" fn mpsse_send_spi(data: *const u8, n: c_int) {
    // println!("mpsse_send_spi({:?}, {}", data, n);
    let bytes = unsafe { slice::from_raw_parts(data, n as usize) };
    let mut bytes = bytes.to_vec();
    COM_PORT.with(|p| {
        p.borrow_mut()
            .as_mut()
            .map(|x| x.xfer_spi(&mut bytes).unwrap())
    });
}

#[no_mangle]
pub extern "C" fn mpsse_xfer_spi(data: *mut u8, n: c_int) {
    // println!("mpsse_xfer_spi({:?}, {})", data, n);
    let mut bytes = unsafe { slice::from_raw_parts_mut(data, n as usize) };

    COM_PORT.with(|p| {
        p.borrow_mut()
            .as_mut()
            .map(|x| x.xfer_spi(&mut bytes).unwrap())
    });
}

#[no_mangle]
pub extern "C" fn mpsse_set_gpio(gpio: u8, direction: u8) {
    // println!("mpsse_set_gpio({}, {})", gpio, direction);
    COM_PORT.with(|p| {
        p.borrow_mut()
            .as_mut()
            .map(|x| x.set_gpio(gpio, direction).unwrap())
    });
}

#[no_mangle]
pub extern "C" fn mpsse_readb_low() -> c_int {
    // println!("mpsse_readb_low()");
    COM_PORT
        .with(|p| p.borrow_mut().as_mut().map(|x| x.readb_low().unwrap()))
        .unwrap() as _
}

#[no_mangle]
pub extern "C" fn mpsse_send_dummy_bytes(n: u8) {
    // println!("mpsse_send_dummy_bytes({})", n);
    COM_PORT.with(|p| {
        p.borrow_mut()
            .as_mut()
            .map(|x| x.send_dummy_bytes(n).unwrap())
    });
}

#[no_mangle]
pub extern "C" fn mpsse_send_dummy_bit() {
    // println!("mpsse_send_dummy_bit()");
    COM_PORT.with(|p| p.borrow_mut().as_mut().map(|x| x.send_dummy_bit().unwrap()));
}

#[no_mangle]
pub extern "C" fn mpsse_init(ifnum: c_int, devstr: *const c_char, slow_clock: bool) {
    // println!("mpsse_init({}, {:?}, {})", ifnum, devstr, slow_clock);
    let device_name = if devstr.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(devstr).to_string_lossy() })
    };

    let ftdi_ifnum = match ifnum {
        0 => Some(Interface::A),
        1 => Some(Interface::B),
        2 => Some(Interface::C),
        3 => Some(Interface::D),
        _ => None,
    };

    let mut vcp = if let Some(name) = device_name {
        VCP::new_from_name(&name).expect(&format!(
            "Can't find iCE FTDI USB device (device string {}).",
            name
        ))
    } else {
        VCP::new_from_vid_pid(0x0403, 0x6010, ftdi_ifnum)
            .or_else(|_| VCP::new_from_vid_pid(0x0403, 0x6014, ftdi_ifnum))
            .expect(
                "Can't find iCE FTDI USB device (vendor_id 0x0403, device_id 0x6010 or 0x6014).\n",
            )
    };

    vcp.reset().expect("couldn't reset vcp");
    vcp.purge().expect("couldn't purge vcp");

    // let previous_latency = vcp.latency_timer().expect("couldn't  get previous latency");
    vcp.set_latency_timer(1).expect("couldn't set new latency");

    vcp.set_bitmode(0xff, BitMode::MPSSE)
        .expect("couldn't set bitmode");

    // enable clock divide by 5
    vcp.write(&[MC_TCK_D5.to_u8()])
        .or_else(|_| Err(ftdi_vcp_rs::Error::IoError))
        .expect("couldn't enable divide-by-5");

    if slow_clock {
        // set 50 kHz clock
        vcp.write(&[MC_SET_CLK_DIV.to_u8(), 119, 0x00])
            .or_else(|_| Err(ftdi_vcp_rs::Error::IoError))
            .expect("couldn't set slow clock");
    } else {
        // set 6 MHz clock
        vcp.write(&[MC_SET_CLK_DIV.to_u8(), 0x00, 0x00])
            .or_else(|_| Err(ftdi_vcp_rs::Error::IoError))
            .expect("couldn't set fast clock");
    }

    COM_PORT.with(|e| *e.borrow_mut() = Some(vcp));
}

#[no_mangle]
pub extern "C" fn mpsse_close() {}
