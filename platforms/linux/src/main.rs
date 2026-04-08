//! RNL Linux Platform - GTK4 Implementation
//!
//! This is the main entry point for the Linux platform.
//! It initializes GTK4 and calls into the RNL core to run the application.

mod platform;
mod elements;

use std::ffi::{c_char, c_int, CString};

// Import from platform module
pub use platform::*;
pub use elements::*;

// Import from core
use rnl;

/// Main entry point for the Linux application
///
/// This calls into the RNL core which will:
/// 1. Initialize the QuickJS runtime
/// 2. Call `rnl_platform_init()` to register elements
/// 3. Load and execute the JS bundle
/// 4. Call `rnl_platform_run()` to start the event loop
fn main() {
    // Initialize logging
    env_logger::init();
    log::info!("RNL Linux platform starting...");

    // Get command line arguments
    let args: Vec<String> = std::env::args().collect();
    let argc = args.len() as c_int;

    // Convert args to C strings
    let c_args: Vec<CString> = args
        .iter()
        .map(|arg| CString::new(arg.as_str()).unwrap())
        .collect();

    let c_arg_ptrs: Vec<*mut c_char> = c_args
        .iter()
        .map(|arg| arg.as_ptr() as *mut c_char)
        .collect();

    // Look for bundle path argument or use default
    let bundle_path = if args.len() > 1 && !args[1].starts_with('-') {
        CString::new(args[1].as_str()).unwrap()
    } else {
        CString::new("target/bundle.js").unwrap()
    };

    // Call into the core using the exported C function
    let exit_code = unsafe {
        rnl::rnl_main(
            bundle_path.as_ptr(),
            argc,
            c_arg_ptrs.as_ptr() as *mut *mut c_char,
        )
    };

    std::process::exit(exit_code);
}
