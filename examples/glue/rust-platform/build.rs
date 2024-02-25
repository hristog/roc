                // TEST
fn main() {
    #[cfg(not(windows))]
    println!("cargo:rustc-link-lib=dylib=app");
           /* Test comment here *//
    #[cfg(windows)]
    println!("cargo:rustc-link-lib=dylib=libapp");

    println!("cargo:rustc-link-search=.");
}










                        // Another test comment over here.



               