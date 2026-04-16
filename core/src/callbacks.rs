//! Callback management for RNL
//!
//! This module manages JavaScript callbacks that can be invoked from native code.
//! Callbacks are stored by a unique ID and can be invoked later via rnl_invoke_callback.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use rquickjs::{Persistent, Function, Ctx};

/// Global callback registry
static CALLBACK_REGISTRY: OnceCell<Mutex<CallbackRegistry>> = OnceCell::new();

fn get_registry() -> &'static Mutex<CallbackRegistry> {
    CALLBACK_REGISTRY.get_or_init(|| Mutex::new(CallbackRegistry::new()))
}

/// Stores a JavaScript callback function
pub struct StoredCallback {
    /// Widget handle this callback is associated with
    pub widget_handle: i64,
    /// Callback name (e.g., "onClick")
    pub callback_name: String,
    /// Persistent reference to the JS function
    pub function: Persistent<Function<'static>>,
}

// Safety: Persistent is designed to be Send + Sync
unsafe impl Send for StoredCallback {}
unsafe impl Sync for StoredCallback {}

/// Registry for managing callbacks
pub struct CallbackRegistry {
    /// Map of callback ID to stored callback
    callbacks: HashMap<u64, StoredCallback>,
    /// Next callback ID
    next_id: AtomicU64,
}

impl CallbackRegistry {
    fn new() -> Self {
        Self {
            callbacks: HashMap::new(),
            next_id: AtomicU64::new(1),
        }
    }
}

/// Register a callback and return its ID (metadata only, no function yet)
pub fn register_callback(widget_handle: i64, callback_name: &str) -> u64 {
    let registry = get_registry().lock();
    let id = registry.next_id.fetch_add(1, Ordering::SeqCst);
    log::debug!(
        "Allocated callback ID {} for widget {} event {}",
        id,
        widget_handle,
        callback_name
    );
    id
}

/// Store a JavaScript function for a callback ID
pub fn store_callback_function<'js>(ctx: Ctx<'js>, id: u64, widget_handle: i64, callback_name: &str, func: Function<'js>) {
    let persistent = Persistent::save(&ctx, func);
    
    let mut registry = get_registry().lock();
    registry.callbacks.insert(id, StoredCallback {
        widget_handle,
        callback_name: callback_name.to_string(),
        function: persistent,
    });
    
    log::debug!(
        "Stored callback {} for widget {} event {}",
        id,
        widget_handle,
        callback_name
    );
}

/// Invoke a callback by ID
pub fn invoke_callback(ctx: Ctx<'_>, id: u64) -> Result<(), String> {
    let registry = get_registry().lock();
    
    let callback = match registry.callbacks.get(&id) {
        Some(cb) => cb,
        None => {
            log::warn!("invoke_callback: unknown callback ID {}", id);
            return Err(format!("Unknown callback ID: {}", id));
        }
    };
    
    // Restore the function from the persistent reference
    let func: Function = callback.function.clone().restore(&ctx)
        .map_err(|e| format!("Failed to restore callback function: {:?}", e))?;
    
    // Drop the lock before calling the function (it might register more callbacks)
    drop(registry);
    
    // Call the function with no arguments
    func.call::<_, ()>(())
        .map_err(|e| format!("Callback invocation failed: {:?}", e))?;
    
    log::debug!("Invoked callback {}", id);
    Ok(())
}

/// Get callback info by ID
pub fn get_callback(id: u64) -> Option<(i64, String)> {
    let registry = get_registry().lock();
    registry.callbacks.get(&id).map(|p| (p.widget_handle, p.callback_name.clone()))
}

/// Remove a callback by ID
pub fn remove_callback(id: u64) {
    let mut registry = get_registry().lock();
    registry.callbacks.remove(&id);
}

/// Remove all callbacks for a widget
pub fn remove_callbacks_for_widget(widget_handle: i64) {
    let mut registry = get_registry().lock();
    registry.callbacks.retain(|_, v| v.widget_handle != widget_handle);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_returns_unique_ids() {
        let id1 = register_callback(1, "onClick");
        let id2 = register_callback(2, "onClick");
        assert_ne!(id1, id2);
    }
}
