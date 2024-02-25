fn main() {
    #[cfg(not(windows))]
    println!("cargo:rustc-link-lib=dylib=app");
        /*  /        /  /   // Comment           */
    #[cfg(windows)]
    println!("cargo:rustc-link-lib=dylib=libapp");

    println!("cargo:rustc-link-search=.");
}
