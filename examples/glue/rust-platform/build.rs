#[cfg(not(windows))]
#[cfg(not(linux))]
fn main() {
    #[cfg(not(windows))]
    println!("cargo:rustc-link-lib=dylib=app");

    #[cfg(windows)]
    println!("cargo:rustc-link-lib=dylib=libapp");
    #[cfg(not(windows))]
    println!("cargo:rustc-link-search=.");
}
