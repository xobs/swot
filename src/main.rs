
/*
#define FT_LIST_NUMBER_ONLY			0x80000000
#define FT_LIST_BY_INDEX			0x40000000
#define FT_LIST_ALL					0x20000000

#define FT_LIST_MASK (FT_LIST_NUMBER_ONLY|FT_LIST_BY_INDEX|FT_LIST_ALL)
*/
use ftdi_vcp_rs::VCP;

fn main() {
    let mut vcp = VCP::new_from_name("iCEBreaker V1.0e A").expect("couldn't open vcp");
    println!("VCP COM{}:", vcp.com_port().expect("couldn't get com port"));
}
