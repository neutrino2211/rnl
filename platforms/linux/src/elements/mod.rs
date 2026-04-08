//! GTK4 Element implementations
//!
//! Each element type implements the RnlElementFactory C ABI from core/include/rnl.h

mod box_element;
mod text_element;
mod button_element;

pub use box_element::*;
pub use text_element::*;
pub use button_element::*;

use std::ffi::{c_char, c_void};

/// C-compatible element factory vtable (mirrors core/src/registry.rs)
#[repr(C)]
pub struct RnlElementFactory {
    /// Unique element name (e.g., "button", "box", "text-field")
    pub name: *const c_char,
    /// Create a new instance of this element
    pub create: Option<extern "C" fn() -> *mut c_void>,
    /// Set an attribute/prop on the widget
    pub set_attribute: Option<extern "C" fn(*mut c_void, *const c_char, *const c_char)>,
    /// Set a callback attribute (e.g., onClick)
    pub set_callback: Option<extern "C" fn(*mut c_void, *const c_char, *mut c_void)>,
    /// Append a child widget
    pub append_child: Option<extern "C" fn(*mut c_void, *mut c_void)>,
    /// Insert child before a reference widget
    pub insert_before: Option<extern "C" fn(*mut c_void, *mut c_void, *mut c_void)>,
    /// Remove a child widget
    pub remove_child: Option<extern "C" fn(*mut c_void, *mut c_void)>,
    /// Destroy the widget and free resources
    pub destroy: Option<extern "C" fn(*mut c_void)>,
}

// Import the core's registration function
extern "C" {
    fn rnl_register_element(factory: *const RnlElementFactory);
}

/// Register all GTK4 elements with the RNL core
pub fn register_all_elements() {
    log::info!("Registering GTK4 elements...");

    // Register each element
    unsafe {
        rnl_register_element(&box_element::BOX_FACTORY);
        rnl_register_element(&text_element::TEXT_FACTORY);
        rnl_register_element(&button_element::BUTTON_FACTORY);
    }

    log::info!("GTK4 elements registered: box, text, button");
}
