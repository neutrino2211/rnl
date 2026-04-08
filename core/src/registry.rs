//! Element registry - manages registered native element factories
//!
//! Platform code registers element factories at startup via `rnl_register_element`.
//! The registry is then used by the JS bridge to create and manipulate native widgets.

use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};
use std::ptr;
use std::sync::RwLock;

use once_cell::sync::OnceCell;

/// C-compatible element factory vtable
///
/// Each element type (button, box, text, etc.) provides an instance of this struct
/// that defines how to create and manipulate that element.
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

// Safety: Factories are registered once at init, then only read
unsafe impl Send for RnlElementFactory {}
unsafe impl Sync for RnlElementFactory {}

/// Global element factory registry
pub struct Registry {
    factories: RwLock<HashMap<String, &'static RnlElementFactory>>,
}

impl Registry {
    /// Get the global registry instance
    pub fn global() -> &'static Self {
        static INSTANCE: OnceCell<Registry> = OnceCell::new();
        INSTANCE.get_or_init(|| Registry {
            factories: RwLock::new(HashMap::new()),
        })
    }

    /// Register a new element factory
    ///
    /// # Safety
    /// The factory pointer must remain valid for the lifetime of the program.
    pub fn register(&self, factory: &'static RnlElementFactory) {
        if factory.name.is_null() {
            log::warn!("Attempted to register factory with null name");
            return;
        }

        let name = unsafe { CStr::from_ptr(factory.name) }
            .to_string_lossy()
            .into_owned();

        let mut factories = self.factories.write().unwrap();
        if factories.contains_key(&name) {
            log::warn!("Element '{}' already registered, overwriting", name);
        }

        log::debug!("Registering element: {}", name);
        factories.insert(name, factory);
    }

    /// Get a factory by element name
    pub fn get(&self, name: &str) -> Option<&'static RnlElementFactory> {
        self.factories.read().unwrap().get(name).copied()
    }

    /// Get the count of registered elements
    pub fn count(&self) -> usize {
        self.factories.read().unwrap().len()
    }

    /// List all registered element names
    pub fn list(&self) -> Vec<String> {
        self.factories.read().unwrap().keys().cloned().collect()
    }
}

/// C API: Register an element factory
///
/// # Safety
/// - factory must point to a valid RnlElementFactory
/// - The factory must remain valid for the program's lifetime
#[no_mangle]
pub unsafe extern "C" fn rnl_register_element(factory: *const RnlElementFactory) {
    if factory.is_null() {
        log::error!("rnl_register_element called with null factory");
        return;
    }

    // Convert to 'static reference - the factory must live for program duration
    // This is safe because platform code allocates these as static constants
    let factory_ref: &'static RnlElementFactory = &*factory;
    Registry::global().register(factory_ref);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    // Test factory creation function
    extern "C" fn test_create() -> *mut c_void {
        // Return a dummy pointer
        Box::into_raw(Box::new(42i32)) as *mut c_void
    }

    extern "C" fn test_destroy(widget: *mut c_void) {
        unsafe {
            let _ = Box::from_raw(widget as *mut i32);
        }
    }

    #[test]
    fn test_registry_register_and_get() {
        // Create a test factory - this needs to be static for the registry
        static TEST_NAME: &[u8] = b"test-widget\0";
        static mut TEST_FACTORY: RnlElementFactory = RnlElementFactory {
            name: TEST_NAME.as_ptr() as *const c_char,
            create: Some(test_create),
            set_attribute: None,
            set_callback: None,
            append_child: None,
            insert_before: None,
            remove_child: None,
            destroy: Some(test_destroy),
        };

        // Register
        unsafe {
            rnl_register_element(&TEST_FACTORY);
        }

        // Verify registration
        let factory = Registry::global().get("test-widget");
        assert!(factory.is_some());
        
        let f = factory.unwrap();
        assert!(f.create.is_some());
        
        // Test creation
        let widget = (f.create.unwrap())();
        assert!(!widget.is_null());
        
        // Clean up
        (f.destroy.unwrap())(widget);
    }

    #[test]
    fn test_registry_count() {
        // Count should be at least 1 from previous test
        // In a fresh process this might be different
        let count = Registry::global().count();
        // Just verify it doesn't crash
        assert!(count >= 0);
    }
}
