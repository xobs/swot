
fn main() {
    let vcp_dir = "lib/vcp-2.12.28/amd64";
    println!("cargo:rustc-link-search=native={}", vcp_dir);
    // println!("cargo:rustc-link-lib=static={}/ftd2xx", vcp_dir);
    // cc::Build::new()
    //     .file("foo.c")
    //     .file("bar.c")
    //     .compile("foo");
}
