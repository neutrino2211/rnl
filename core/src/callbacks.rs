//! Callback management for RNL
//!
//! This module manages JavaScript callbacks that can be invoked from native code.
//! Callbacks are stored by a unique ID and can be invoked later via rnl_invoke_callback.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use once_cell::sync::OnceCell;
use parking_lot::Mutex;

/// Global callback registry
static CALLBACK_REGISTRY: OnceCell<Mutex<CallbackRegistry>> = OnceCell::new();

fn get_registry() -> &'static Mutex<CallbackRegistry> {
    CALLBACK_REGISTRY.get_or_init(|| Mutex::new(CallbackRegistry::new()))
}

/// Stores information about a pending callback
pub struct PendingCallback {
    /// Widget handle this callback is associated with
    pub widget_handle: i64,
    /// Callback name (e.g., "onClick")
    pub callback_name: String,
}

/// Registry for managing callbacks
pub struct CallbackRegistry {
    /// Map of callback ID to pending callback info
    pending: HashMap<u64, PendingCallback>,
    /// Next callback ID
    next_id: AtomicU64,
}

impl CallbackRegistry {
    fn new() -> Self {
        Self {
            pending: HashMap::new(),
            next_id: AtomicU64::new(1),
        }
    }
}

/// Register a callback and return its ID
pub fn register_callback(widget_handle: i64, callback_name: &str) -> u64 {
    let mut registry = get_registry().lock();
    let id = registry.next_id.fetch_add(1, Ordering::SeqCst);
    registry.pending.insert(
        id,
        PendingCallback {
            widget_handle,
            callback_name: callback_name.to_string(),
        },
    );
    log::debug!(
        "Registered callback {} for widget {} event {}",
        id,
        widget_handle,
        callback_name
    );
    id
}

/// Get callback info by ID
pub fn get_callback(id: u64) -> Option<(i64, String)> {
    let registry = get_registry().lock();
    registry.pending.get(&id).map(|p| (p.widget_handle, p.callback_name.clone()))
}

/// Remove a callback by ID
pub fn remove_callback(id: u64) {
    let mut registry = get_registry().lock();
    registry.pending.remove(&id);
}

/// Remove all callbacks for a widget
pub fn remove_callbacks_for_widget(widget_handle: i64) {
    let mut registry = get_registry().lock();
    registry.pending.retain(|_, v| v.widget_handle != widget_handle);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let id = register_callback(42, "onClick");
        let info = get_callback(id);
        assert!(info.is_some());
        let (handle, name) = info.unwrap();
        assert_eq!(handle, 42);
        assert_eq!(name, "onClick");
    }

    #[test]
    fn test_remove() {
        let id = register_callback(99, "onPress");
        remove_callback(id);
        assert!(get_callback(id).is_none());
    }
}
