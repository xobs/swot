use cc;

fn main() {
    println!("cargo:rerun-if-changed=src/mpsse.h");
    println!("cargo:rerun-if-changed=src/unistd.h");
    println!("cargo:rerun-if-changed=src/getopt.h");
    println!("cargo:rerun-if-changed=src/iceprog.c");
    cc::Build::new()
        .file("src/iceprog.c")
        .include("src")
        // .warnings_into_errors(true)
        .debug(true)
        .compile("iceprog");
}
