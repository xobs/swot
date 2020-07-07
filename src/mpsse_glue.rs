#[no_mangle]
pub extern "C" fn mpsse_error(status: usize) {}

#[no_mangle]
pub extern "C" fn mpsse_send_spi(data: *const u8, n: usize) {}

#[no_mangle]
pub extern "C" fn mpsse_xfer_spi(data: *mut u8, n: usize) {}

#[no_mangle]
pub extern "C" fn mpsse_set_gpio(gpio: u8, direction: u8) {}

#[no_mangle]
pub extern "C" fn mpsse_readb_low() -> usize {
    0
}

#[no_mangle]
pub extern "C" fn mpsse_send_dummy_bytes(n: u8) {}

#[no_mangle]
pub extern "C" fn mpsse_send_dummy_bit() {}

#[no_mangle]
pub extern "C" fn mpsse_init(ifnum: usize, devstr: *const u8, slow_clock: bool) {}

#[no_mangle]
pub extern "C" fn mpsse_close() {}

// uint8_t mpsse_recv_byte(void);
// void mpsse_send_byte(uint8_t data);
// uint8_t mpsse_xfer_spi_bits(uint8_t data, int n);
// int mpsse_readb_high(void);
