//! Build script for RNL Linux platform
//!
//! Links against librnl (the Rust core) when available.

fn main() {
    // Tell cargo to look for librnl in the core target directory
    // The actual linking happens when we build the final binary
    
    // During development, we might build the platform independently
    // but for the final binary, we need librnl.a from the core crate
    
    let out_dir = std::env::var("OUT_DIR").unwrap_or_default();
    println!("cargo:warning=Building rnl-linux platform (out: {})", out_dir);
    
    // The linker will need these directories to find librnl
    // Try common locations
    let core_paths = [
        "../../core/target/debug",
        "../../core/target/release",
        "../core/target/debug",
        "../core/target/release",
    ];
    
    for path in &core_paths {
        if std::path::Path::new(path).exists() {
            println!("cargo:rustc-link-search=native={}", path);
        }
    }
    
    // Link against the core library
    // Note: The actual binary linking will be done by rnl-cli
    // which combines the platform code with librnl.a
}
