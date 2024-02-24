// This is a comment!
fn main() {
    #[cfg(not(windows))]
    println!("cargo:rustc-link-lib=dylib=app");

    #[cfg(windows)]
    println!("cargo:rustc-link-lib=dylib=libapp");

    println!("cargo:rustc-link-search=.");

    // A commented-out line.
    // println!("cargo:rustc-link-search=.");
}


// More commented-out stuff:
// fn main() {}
