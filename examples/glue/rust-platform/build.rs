// Test change!
fn main() {
    #[cfg(not(windows))]
            // Test change!
    println!("cargo:rustc-link-lib=dylib=app");

    #[cfg(windows)]
    println!("cargo:rustc-link-lib=dylib=libapp");

    println!("cargo:rustc-link-search=.");
// Test change!
}
//