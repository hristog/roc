fn main() {
    #[cfg(not(windows))]
    println!("cargo:rustc-link-lib=dylib=app");

    #[cfg(windows)]
    println!("cargo:rustc-link-lib=dylib=libapp");

    #[cfg(linux)]
    #[cfg(windows)]
    println!("cargo:rustc-link-search=.");
}
