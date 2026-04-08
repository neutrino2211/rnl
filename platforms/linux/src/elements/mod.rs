//! GTK4 Element implementations
//!
//! Each element type implements the RnlElementFactory C ABI from core

mod box_element;
mod text_element;
mod button_element;

pub use box_element::*;
pub use text_element::*;
pub use button_element::*;

// Re-export from core
pub use rnl::registry::RnlElementFactory;

/// Register all GTK4 elements with the RNL core
pub fn register_all_elements() {
    log::info!("Registering GTK4 elements...");

    // Register each element using core's function
    unsafe {
        rnl::registry::rnl_register_element(&box_element::BOX_FACTORY);
        rnl::registry::rnl_register_element(&text_element::TEXT_FACTORY);
        rnl::registry::rnl_register_element(&button_element::BUTTON_FACTORY);
    }

    log::info!("GTK4 elements registered: box, text, button");
}
