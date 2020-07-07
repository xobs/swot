/*
#define FT_LIST_NUMBER_ONLY			0x80000000
#define FT_LIST_BY_INDEX			0x40000000
#define FT_LIST_ALL					0x20000000

#define FT_LIST_MASK (FT_LIST_NUMBER_ONLY|FT_LIST_BY_INDEX|FT_LIST_ALL)
*/
// use clap::{App, Arg};
// use std::fs::File;
// use std::io::Seek;
// use std::io::{Read, Write};
// use std::thread::sleep;
// use std::time::Duration;

// mod flash;
mod mpsse_glue;

// fn parse_size(input: &str) -> Result<usize, &'static str> {
//     let multiple_index = input
//         .chars()
//         .position(|c| !(c.is_numeric() || c == '.'))
//         .unwrap_or(input.len());

//     let (value, multiple) = &input.split_at(multiple_index);
//     let value = value.parse::<usize>().map_err(|_| "unable to parse")?;
//     let multiple = match multiple.trim().to_lowercase().as_str() {
//         "m" | "mib" => 1024 * 1024,
//         "k" | "kib" => 1024,
//         "b" | "" | "bytes" => 1,
//         "g" | "gib" => 1024 * 1024 * 1024,
//         x => {
//             println!("Unrecognized suffix {}", x);
//             return Err("unrecognized suffix");
//         }
//     };
//     Ok(value * multiple)
// }

// #[test]
// fn parse_size_sanity() {
//     assert_eq!(parse_size("0").unwrap(), 0);
//     assert_eq!(parse_size("1024").unwrap(), 1024);
//     assert_eq!(parse_size("1k").unwrap(), 1024);
//     assert_eq!(parse_size("1K").unwrap(), 1024);
//     assert_eq!(parse_size("1 K").unwrap(), 1024);
//     assert_eq!(parse_size("1 k").unwrap(), 1024);
//     assert_eq!(parse_size("1 kiB").unwrap(), 1024);
//     assert_eq!(parse_size("1 M").unwrap(), 1024 * 1024);
//     assert_eq!(parse_size("2 M").unwrap(), 1024 * 1024 * 2);
// }

use std::os::raw::{c_char, c_int};
use std::ffi::CString;
extern "C" {
    fn iceprog_main(argc: c_int, argv: *const *const c_char) -> c_int;
}

fn main() -> Result<(), ftdi_vcp_rs::Error> {
    // create a vector of zero terminated strings
    let args = std::env::args()
        .map(|arg| CString::new(arg).unwrap())
        .collect::<Vec<CString>>();
    // convert the strings to raw pointers
    let c_args = args
        .iter()
        .map(|arg| arg.as_ptr())
        .collect::<Vec<*const c_char>>();

    unsafe {
        iceprog_main(c_args.len() as c_int, c_args.as_ptr())
    };
    Ok(())
}
