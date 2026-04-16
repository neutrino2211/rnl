//! JS ↔ Native bridge
//!
//! Handles communication between JavaScript and native platform code.
//! Manages widget handles and maps them to native pointers.

use std::collections::HashMap;
use std::ffi::{c_char, c_void, CString};
use std::ptr;
use std::sync::atomic::{AtomicI64, Ordering};

use once_cell::sync::OnceCell;
use parking_lot::Mutex;

use crate::registry::Registry;

/// Global bridge instance
static BRIDGE: OnceCell<Mutex<NativeBridge>> = OnceCell::new();

fn get_bridge() -> &'static Mutex<NativeBridge> {
    BRIDGE.get_or_init(|| Mutex::new(NativeBridge::new()))
}

/// Information about a widget instance
struct WidgetHandle {
    /// Element type name (e.g., "button", "text")
    type_name: String,
    /// Native widget pointer
    ptr: *mut c_void,
    /// Text content for text nodes
    text: Option<String>,
}

// Safety: Widget pointers are managed carefully
unsafe impl Send for WidgetHandle {}
unsafe impl Sync for WidgetHandle {}

/// Bridge between JS and native code
pub struct NativeBridge {
    /// Map of handle ID to widget info
    widgets: HashMap<i64, WidgetHandle>,
    /// Next available handle ID
    next_id: AtomicI64,
    /// Root container handle (set by platform)
    root_handle: Option<i64>,
    /// Stored JS callbacks (handle_id, callback_name) -> callback
    /// Note: In a full impl, we'd need proper QuickJS lifetime management
    callbacks: HashMap<(i64, String), usize>, // Placeholder - would store callback refs
}

impl NativeBridge {
    fn new() -> Self {
        Self {
            widgets: HashMap::new(),
            next_id: AtomicI64::new(1), // Start at 1, 0 = invalid
            root_handle: None,
            callbacks: HashMap::new(),
        }
    }

    fn create_node(&mut self, type_name: &str) -> Result<i64, String> {
        let factory = Registry::global()
            .get(type_name)
            .ok_or_else(|| format!("Unknown element type: {}", type_name))?;

        let create_fn = factory.create.ok_or_else(|| {
            format!("Element '{}' has no create function", type_name)
        })?;

        let ptr = create_fn();
        if ptr.is_null() {
            return Err(format!("Failed to create element '{}'", type_name));
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.widgets.insert(
            id,
            WidgetHandle {
                type_name: type_name.to_string(),
                ptr,
                text: None,
            },
        );

        log::debug!("Created node {} of type '{}' (ptr: {:?})", id, type_name, ptr);
        Ok(id)
    }

    fn create_text(&mut self, text: &str) -> i64 {
        // Text nodes are special - they're stored internally and only
        // materialized when appended to a text-capable parent
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.widgets.insert(
            id,
            WidgetHandle {
                type_name: "__text__".to_string(),
                ptr: ptr::null_mut(),
                text: Some(text.to_string()),
            },
        );

        log::debug!("Created text node {} with content '{}'", id, text);
        id
    }

    fn set_attribute(&mut self, handle: i64, name: &str, value: &str) {
        let widget = match self.widgets.get(&handle) {
            Some(w) => w,
            None => {
                log::warn!("setAttribute: unknown handle {}", handle);
                return;
            }
        };

        // Skip text nodes
        if widget.type_name == "__text__" {
            return;
        }

        let factory = match Registry::global().get(&widget.type_name) {
            Some(f) => f,
            None => {
                log::warn!("setAttribute: unknown type '{}'", widget.type_name);
                return;
            }
        };

        if let Some(set_attr) = factory.set_attribute {
            let name_c = CString::new(name).unwrap();
            let value_c = CString::new(value).unwrap();
            set_attr(widget.ptr, name_c.as_ptr(), value_c.as_ptr());
        }
    }

    fn set_callback(&mut self, handle: i64, name: &str, callback_id: u64) {
        let widget = match self.widgets.get(&handle) {
            Some(w) => w,
            None => {
                log::warn!("setCallback: unknown handle {}", handle);
                return;
            }
        };

        if widget.type_name == "__text__" || widget.type_name == "__root__" {
            return;
        }

        let factory = match Registry::global().get(&widget.type_name) {
            Some(f) => f,
            None => {
                log::warn!("setCallback: unknown type '{}'", widget.type_name);
                return;
            }
        };

        log::debug!("setCallback({}, {}, callback_id={})", handle, name, callback_id);

        // Store the callback ID for this (handle, name) pair
        self.callbacks.insert((handle, name.to_string()), callback_id as usize);

        // Pass the callback ID to the native element
        // The callback ID acts as a "pointer" that the native code can use
        // to invoke rnl_invoke_callback later
        if let Some(set_cb) = factory.set_callback {
            let name_c = CString::new(name).unwrap();
            // Pass callback_id as the callback pointer
            set_cb(widget.ptr, name_c.as_ptr(), callback_id as *mut c_void);
        }
    }

    fn append_child(&mut self, parent_handle: i64, child_handle: i64) {
        // Get child info first
        let (child_type, child_ptr, child_text) = {
            let child = match self.widgets.get(&child_handle) {
                Some(w) => w,
                None => {
                    log::warn!("appendChild: unknown child handle {}", child_handle);
                    return;
                }
            };
            (child.type_name.clone(), child.ptr, child.text.clone())
        };

        // Handle text nodes specially - they might need to set label on parent
        if child_type == "__text__" {
            if let Some(text) = child_text {
                // Try to set as "children" or "label" attribute on parent
                self.set_attribute(parent_handle, "children", &text);
            }
            return;
        }

        // Get parent info
        let (parent_type, parent_ptr) = {
            let parent = match self.widgets.get(&parent_handle) {
                Some(w) => w,
                None => {
                    log::warn!("appendChild: unknown parent handle {}", parent_handle);
                    return;
                }
            };
            (parent.type_name.clone(), parent.ptr)
        };

        // Special case: __root__ is a GtkBox set by the platform
        // We need to use the "box" factory's append_child for it
        let effective_type = if parent_type == "__root__" {
            "box".to_string()
        } else {
            parent_type
        };

        let factory = match Registry::global().get(&effective_type) {
            Some(f) => f,
            None => {
                log::warn!("appendChild: unknown parent type '{}'", effective_type);
                return;
            }
        };

        if let Some(append) = factory.append_child {
            append(parent_ptr, child_ptr);
        }
    }

    fn insert_before(&mut self, parent_handle: i64, child_handle: i64, before_handle: i64) {
        let parent = match self.widgets.get(&parent_handle) {
            Some(w) => w,
            None => {
                log::warn!("insertBefore: unknown parent handle {}", parent_handle);
                return;
            }
        };

        let child = match self.widgets.get(&child_handle) {
            Some(w) => w,
            None => {
                log::warn!("insertBefore: unknown child handle {}", child_handle);
                return;
            }
        };

        let before = match self.widgets.get(&before_handle) {
            Some(w) => w,
            None => {
                log::warn!("insertBefore: unknown before handle {}", before_handle);
                return;
            }
        };

        let factory = match Registry::global().get(&parent.type_name) {
            Some(f) => f,
            None => return,
        };

        if let Some(insert) = factory.insert_before {
            insert(parent.ptr, child.ptr, before.ptr);
        }
    }

    fn remove_child(&mut self, parent_handle: i64, child_handle: i64) {
        let parent = match self.widgets.get(&parent_handle) {
            Some(w) => w,
            None => {
                log::warn!("removeChild: unknown parent handle {}", parent_handle);
                return;
            }
        };

        let child = match self.widgets.get(&child_handle) {
            Some(w) => w,
            None => {
                log::warn!("removeChild: unknown child handle {}", child_handle);
                return;
            }
        };

        let factory = match Registry::global().get(&parent.type_name) {
            Some(f) => f,
            None => return,
        };

        if let Some(remove) = factory.remove_child {
            remove(parent.ptr, child.ptr);
        }
    }

    fn set_text(&mut self, handle: i64, text: &str) {
        if let Some(widget) = self.widgets.get_mut(&handle) {
            if widget.type_name == "__text__" {
                widget.text = Some(text.to_string());
            }
        }
    }

    fn get_root_handle(&self) -> i64 {
        self.root_handle.unwrap_or(0)
    }

    fn set_root(&mut self, ptr: *mut c_void) {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.widgets.insert(
            id,
            WidgetHandle {
                type_name: "__root__".to_string(),
                ptr,
                text: None,
            },
        );
        self.root_handle = Some(id);
        log::debug!("Set root container handle to {}", id);
    }
}

// Public functions called from JS bindings in runtime.rs

pub fn create_node(type_name: &str) -> i64 {
    match get_bridge().lock().create_node(type_name) {
        Ok(id) => id,
        Err(e) => {
            log::error!("create_node failed: {}", e);
            0 // Invalid handle
        }
    }
}

pub fn create_text(text: &str) -> i64 {
    get_bridge().lock().create_text(text)
}

pub fn set_attribute(handle: i64, name: &str, value: &str) {
    get_bridge().lock().set_attribute(handle, name, value)
}

pub fn set_callback(handle: i64, name: &str) {
    // Legacy stub - use set_callback_with_id instead
    log::warn!("set_callback called without callback_id");
}

pub fn set_callback_with_id(handle: i64, name: &str, callback_id: u64) {
    get_bridge().lock().set_callback(handle, name, callback_id)
}

pub fn append_child(parent: i64, child: i64) {
    get_bridge().lock().append_child(parent, child)
}

pub fn insert_before(parent: i64, child: i64, before: i64) {
    get_bridge().lock().insert_before(parent, child, before)
}

pub fn remove_child(parent: i64, child: i64) {
    get_bridge().lock().remove_child(parent, child)
}

pub fn set_text(handle: i64, text: &str) {
    get_bridge().lock().set_text(handle, text)
}

pub fn get_root_handle() -> i64 {
    get_bridge().lock().get_root_handle()
}

/// C API: Invoke a JS callback from native code
///
/// # Safety
/// - callback must be a valid callback ID (cast to pointer)
/// - event_json must be a valid C string (currently unused)
#[no_mangle]
pub unsafe extern "C" fn rnl_invoke_callback(callback: *mut c_void, event_json: *const c_char) {
    if callback.is_null() {
        return;
    }

    // The callback pointer is actually the callback ID
    let callback_id = callback as u64;

    let event = if event_json.is_null() {
        "{}".to_string()
    } else {
        std::ffi::CStr::from_ptr(event_json)
            .to_string_lossy()
            .into_owned()
    };

    log::debug!("rnl_invoke_callback({}) with event: {}", callback_id, event);
    
    // Get the runtime and invoke the callback
    // We need to call through to lib.rs which owns the runtime
    extern "C" {
        fn rnl_invoke_callback_impl(callback_id: u64) -> std::ffi::c_int;
    }
    
    let result = rnl_invoke_callback_impl(callback_id);
    if result != 0 {
        log::error!("Callback {} invocation failed", callback_id);
    }
}

/// C API: Set the root container handle (called by platform)
///
/// # Safety
/// - root must be a valid platform widget pointer
#[no_mangle]
pub unsafe extern "C" fn rnl_set_root_container(root: *mut c_void) {
    get_bridge().lock().set_root(root);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_text() {
        let id = create_text("Hello, World!");
        assert!(id > 0);
    }

    #[test]
    fn test_get_root_handle_default() {
        // Default root handle is 0 (none set)
        // After set_root is called, it would be different
        let handle = get_root_handle();
        // Just verify it doesn't crash
        assert!(handle >= 0);
    }
}
