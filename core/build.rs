//! Build script - generates rnl.h header using cbindgen

use std::env;
use std::path::PathBuf;

fn main() {
    // Only generate headers on explicit request or release builds
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = PathBuf::from(&crate_dir).join("include");

    std::fs::create_dir_all(&out_dir).expect("Failed to create include directory");

    let config = cbindgen::Config::from_file("cbindgen.toml")
        .expect("Failed to read cbindgen.toml");

    if let Err(e) = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .map(|bindings| bindings.write_to_file(out_dir.join("rnl.h")))
    {
        // Don't fail build if cbindgen fails - the header might already exist
        eprintln!("Warning: cbindgen failed: {}", e);
    }

    // Tell Cargo to rerun this if source changes
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=cbindgen.toml");
}
