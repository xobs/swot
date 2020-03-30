use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    fs::copy(
        "lib/vcp-2.12.28/amd64/ftd2xx.lib",
        out_dir.join(format!("ftd2xx.lib")),
    )
    .unwrap();
    println!("cargo:rustc-link-search=native={}", out_dir.into_os_string().into_string().expect("invalid path string"));
    // println!("cargo:rustc-link-search=native={}", vcp_dir);
    // println!("cargo:rustc-link-lib=static={}/ftd2xx", vcp_dir);
    // cc::Build::new()
    //     .file("foo.c")
    //     .file("bar.c")
    //     .compile("foo");
}
