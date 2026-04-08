//! RNL Core - Rust runtime for React Native Libre
//!
//! This crate provides:
//! - QuickJS JavaScript runtime
//! - Element registry for native components
//! - JS bridge for native ↔ JavaScript communication
//! - C ABI for platform integration

pub mod runtime;
pub mod registry;
pub mod bridge;
pub mod callbacks;
pub mod ffi;

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;

use once_cell::sync::OnceCell;
use parking_lot::Mutex;

use runtime::JsRuntime;
use registry::Registry;
use bridge::NativeBridge;

/// Global runtime instance
static RUNTIME: OnceCell<Mutex<JsRuntime>> = OnceCell::new();

/// Initialize the global runtime
fn init_runtime() -> &'static Mutex<JsRuntime> {
    RUNTIME.get_or_init(|| {
        Mutex::new(JsRuntime::new().expect("Failed to initialize JS runtime"))
    })
}

/// Main entry point - called by platform's main()
///
/// # Safety
/// - bundle_path must be a valid C string or NULL
/// - argv must be an array of argc valid C strings
#[no_mangle]
pub unsafe extern "C" fn rnl_main(
    bundle_path: *const c_char,
    argc: c_int,
    argv: *mut *mut c_char,
) -> c_int {
    // Logger may already be initialized by the platform
    let _ = env_logger::try_init();
    log::info!("RNL Core starting...");

    // Initialize runtime
    let runtime = init_runtime();

    // Call platform init to register elements
    extern "C" {
        fn rnl_platform_init();
        fn rnl_platform_create_window(title: *const c_char, width: i32, height: i32) -> c_int;
        fn rnl_platform_set_bundle(bundle: *const c_char);
    }
    rnl_platform_init();
    log::info!("Platform initialized, {} elements registered", 
               Registry::global().count());

    // Create the main window
    let title = std::ffi::CString::new("RNL App").unwrap();
    rnl_platform_create_window(title.as_ptr(), 800, 600);

    // Load JS bundle
    let bundle_content = if bundle_path.is_null() {
        // Embedded bundle would be included via include_str! in production
        // For now, look for bundle.js in current directory
        std::fs::read_to_string("target/bundle.js")
            .unwrap_or_else(|_| {
                log::warn!("No bundle found, using minimal bootstrap");
                r#"console.log("RNL Runtime ready - no bundle loaded");"#.to_string()
            })
    } else {
        let path = CStr::from_ptr(bundle_path).to_string_lossy();
        std::fs::read_to_string(path.as_ref())
            .unwrap_or_else(|e| {
                log::error!("Failed to load bundle from {}: {}", path, e);
                String::new()
            })
    };

    // Pass bundle to platform - it will execute after window is ready
    let bundle_c = std::ffi::CString::new(bundle_content).unwrap();
    rnl_platform_set_bundle(bundle_c.as_ptr());

    log::info!("Bundle set, starting platform event loop");

    // Start platform event loop
    extern "C" {
        fn rnl_platform_run() -> c_int;
    }
    rnl_platform_run()
}

/// Execute a JS bundle (called by platform after window is ready)
///
/// # Safety
/// - bundle must be a valid C string
#[no_mangle]
pub unsafe extern "C" fn rnl_execute_bundle(bundle: *const c_char) -> c_int {
    if bundle.is_null() {
        log::error!("rnl_execute_bundle: null bundle");
        return 1;
    }
    
    let bundle_content = CStr::from_ptr(bundle).to_string_lossy();
    log::info!("Executing bundle ({} bytes)", bundle_content.len());
    
    let runtime = init_runtime();
    let mut rt = runtime.lock();
    
    if let Err(e) = rt.eval(&bundle_content, "<bundle>") {
        log::error!("JS evaluation failed: {}", e);
        return 1;
    }
    
    log::info!("Bundle executed successfully");
    0
}

/// Log a message from native code (routed to JS console)
///
/// # Safety
/// - level and message must be valid C strings
#[no_mangle]
pub unsafe extern "C" fn rnl_log(level: *const c_char, message: *const c_char) {
    if level.is_null() || message.is_null() {
        return;
    }

    let level_str = CStr::from_ptr(level).to_string_lossy();
    let msg = CStr::from_ptr(message).to_string_lossy();

    match level_str.as_ref() {
        "debug" => log::debug!("[native] {}", msg),
        "info" => log::info!("[native] {}", msg),
        "warn" => log::warn!("[native] {}", msg),
        "error" => log::error!("[native] {}", msg),
        _ => log::info!("[native] {}", msg),
    }
}

/// Report an error from native code (will throw in JS)
///
/// # Safety
/// - message must be a valid C string
#[no_mangle]
pub unsafe extern "C" fn rnl_error(message: *const c_char) {
    if message.is_null() {
        return;
    }

    let msg = CStr::from_ptr(message).to_string_lossy();
    log::error!("[native error] {}", msg);

    // In a full implementation, this would throw in the JS context
    // For now, just log the error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_initialization() {
        let rt = init_runtime();
        let mut runtime = rt.lock();
        
        // Should be able to evaluate simple JS
        let result = runtime.eval("1 + 1", "<test>");
        assert!(result.is_ok());
    }
}
