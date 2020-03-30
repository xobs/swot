
/*
#define FT_LIST_NUMBER_ONLY			0x80000000
#define FT_LIST_BY_INDEX			0x40000000
#define FT_LIST_ALL					0x20000000

#define FT_LIST_MASK (FT_LIST_NUMBER_ONLY|FT_LIST_BY_INDEX|FT_LIST_ALL)
*/
use ftdi_vcp_rs::VCP;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut vcp = VCP::new_from_name("iCEBreaker V1.0e A").expect("couldn't open vcp");
    println!("Opened VCP: {:?}", vcp);
    vcp.set_bit_mode(0x80).expect("couldn't set bit mode");
    for i in 0..10 {
        if i & 1 != 0 {
            vcp.write(&[0x80]).expect("couldn't set all 1");
        } else {
            vcp.write(&[0x00]).expect("couldn't set all 1");
        }
        sleep(Duration::from_millis(500));
    }
    println!("VCP COM{}:", vcp.com_port().expect("couldn't get com port"));
}
