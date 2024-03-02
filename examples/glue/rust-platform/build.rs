fn main() {
    #[cfg(not(windows))]
                     /*///Test comment*&//// */
    println!("cargo:rustc-link-lib=dylib=app");

    #[cfg(windows)]
    println!("cargo:rustc-link-lib=dylib=libapp");

    println!("cargo:rustc-link-search=.");

    println!("cargo:rustc-link-search=.");
}
