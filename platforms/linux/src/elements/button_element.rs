//! Button element - GtkButton wrapper
//!
//! A clickable button widget. Maps to GtkButton with onClick callback support.

use gtk4::prelude::*;
use gtk4::{Button, Widget};
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};

use once_cell::sync::Lazy;
use parking_lot::Mutex;

use super::RnlElementFactory;

/// Static name for the element (null-terminated C string)
static BUTTON_NAME: &[u8] = b"button\0";

/// The element factory for "button"
pub static BUTTON_FACTORY: RnlElementFactory = RnlElementFactory {
    name: BUTTON_NAME.as_ptr() as *const c_char,
    create: Some(button_create),
    set_attribute: Some(button_set_attribute),
    set_callback: Some(button_set_callback),
    append_child: Some(button_append_child),
    insert_before: None,
    remove_child: Some(button_remove_child),
    destroy: Some(button_destroy),
};

/// Store callback pointers for buttons
/// Key: button widget pointer, Value: callback pointer
static BUTTON_CALLBACKS: Lazy<Mutex<HashMap<usize, usize>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Create a new GtkButton
extern "C" fn button_create() -> *mut c_void {
    log::debug!("Creating GtkButton");

    let button = Button::builder()
        .label("")
        .build();

    let ptr = button.as_ptr() as *mut c_void;
    
    // Connect the clicked signal
    let button_ptr = ptr as usize;
    button.connect_clicked(move |_btn| {
        log::debug!("Button clicked (ptr: {})", button_ptr);
        
        // Get the callback for this button
        let callback_ptr = {
            let callbacks = BUTTON_CALLBACKS.lock();
            callbacks.get(&button_ptr).copied()
        };

        if let Some(cb_ptr) = callback_ptr {
            if cb_ptr != 0 {
                // Invoke the callback through the core
                extern "C" {
                    fn rnl_invoke_callback(callback: *mut c_void, event_json: *const c_char);
                }

                let event = b"{\"type\":\"click\"}\0";
                unsafe {
                    rnl_invoke_callback(
                        cb_ptr as *mut c_void,
                        event.as_ptr() as *const c_char,
                    );
                }
            }
        }
    });

    std::mem::forget(button);

    log::debug!("Created GtkButton at {:?}", ptr);
    ptr
}

/// Set an attribute on the button
extern "C" fn button_set_attribute(widget: *mut c_void, name: *const c_char, value: *const c_char) {
    if widget.is_null() || name.is_null() || value.is_null() {
        return;
    }

    let button: Button = unsafe {
        let ptr = widget as *mut gtk4::ffi::GtkButton;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    let name_str = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    let value_str = unsafe { CStr::from_ptr(value) }.to_string_lossy();

    log::debug!("button.set_attribute({}, {})", name_str, value_str);

    match name_str.as_ref() {
        "label" | "children" | "text" => {
            button.set_label(&value_str);
        }
        "enabled" | "sensitive" => {
            button.set_sensitive(value_str == "true");
        }
        "disabled" => {
            // Inverse of enabled
            button.set_sensitive(value_str != "true");
        }
        "has-frame" | "hasFrame" => {
            button.set_has_frame(value_str == "true");
        }
        "icon-name" | "iconName" => {
            button.set_icon_name(&value_str);
        }
        // Style attributes
        "margin" => {
            if let Ok(margin) = value_str.parse::<i32>() {
                button.set_margin_start(margin);
                button.set_margin_end(margin);
                button.set_margin_top(margin);
                button.set_margin_bottom(margin);
            }
        }
        "halign" => {
            let align = parse_alignment(&value_str);
            button.set_halign(align);
        }
        "valign" => {
            let align = parse_alignment(&value_str);
            button.set_valign(align);
        }
        "hexpand" => {
            button.set_hexpand(value_str == "true");
        }
        "vexpand" => {
            button.set_vexpand(value_str == "true");
        }
        "css-classes" | "cssClasses" => {
            // Parse comma-separated CSS classes
            let classes: Vec<&str> = value_str.split(',').map(|s| s.trim()).collect();
            button.set_css_classes(&classes);
        }
        _ => {
            log::debug!("Unknown button attribute: {}", name_str);
        }
    }

    std::mem::forget(button);
}

fn parse_alignment(value: &str) -> gtk4::Align {
    match value {
        "start" | "flex-start" => gtk4::Align::Start,
        "end" | "flex-end" => gtk4::Align::End,
        "center" => gtk4::Align::Center,
        "fill" | "stretch" => gtk4::Align::Fill,
        "baseline" => gtk4::Align::Baseline,
        _ => gtk4::Align::Fill,
    }
}

/// Set a callback on the button
extern "C" fn button_set_callback(
    widget: *mut c_void,
    name: *const c_char,
    callback: *mut c_void,
) {
    if widget.is_null() || name.is_null() {
        return;
    }

    let name_str = unsafe { CStr::from_ptr(name) }.to_string_lossy();

    log::debug!("button.set_callback({}, {:?})", name_str, callback);

    match name_str.as_ref() {
        "onClick" | "onclick" | "onPress" | "onpress" => {
            // Store the callback pointer
            let mut callbacks = BUTTON_CALLBACKS.lock();
            callbacks.insert(widget as usize, callback as usize);
            log::debug!("Stored onClick callback for button {:?}", widget);
        }
        _ => {
            log::debug!("Unknown button callback: {}", name_str);
        }
    }
}

/// Append a child to the button - buttons can have a custom child widget
extern "C" fn button_append_child(parent: *mut c_void, child: *mut c_void) {
    if parent.is_null() || child.is_null() {
        return;
    }

    let button: Button = unsafe {
        let ptr = parent as *mut gtk4::ffi::GtkButton;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    let child_widget: Widget = unsafe {
        let ptr = child as *mut gtk4::ffi::GtkWidget;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    log::debug!("button.append_child - setting custom child");
    button.set_child(Some(&child_widget));

    std::mem::forget(button);
    std::mem::forget(child_widget);
}

/// Remove a child from the button
extern "C" fn button_remove_child(parent: *mut c_void, _child: *mut c_void) {
    if parent.is_null() {
        return;
    }

    let button: Button = unsafe {
        let ptr = parent as *mut gtk4::ffi::GtkButton;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    log::debug!("button.remove_child");
    button.set_child(None::<&Widget>);

    std::mem::forget(button);
}

/// Destroy the button widget
extern "C" fn button_destroy(widget: *mut c_void) {
    if widget.is_null() {
        return;
    }

    log::debug!("button.destroy");

    // Remove the callback
    {
        let mut callbacks = BUTTON_CALLBACKS.lock();
        callbacks.remove(&(widget as usize));
    }

    let button: Button = unsafe {
        let ptr = widget as *mut gtk4::ffi::GtkButton;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    drop(button);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_name() {
        let name = unsafe { CStr::from_ptr(BUTTON_FACTORY.name) };
        assert_eq!(name.to_str().unwrap(), "button");
    }

    #[test]
    fn test_factory_functions_exist() {
        assert!(BUTTON_FACTORY.create.is_some());
        assert!(BUTTON_FACTORY.set_attribute.is_some());
        assert!(BUTTON_FACTORY.set_callback.is_some());
        assert!(BUTTON_FACTORY.append_child.is_some());
        assert!(BUTTON_FACTORY.destroy.is_some());
    }

    #[test]
    fn test_callback_storage() {
        // Test that we can store and retrieve callbacks
        let widget_ptr = 12345usize;
        let callback_ptr = 67890usize;

        {
            let mut callbacks = BUTTON_CALLBACKS.lock();
            callbacks.insert(widget_ptr, callback_ptr);
        }

        {
            let callbacks = BUTTON_CALLBACKS.lock();
            assert_eq!(callbacks.get(&widget_ptr), Some(&callback_ptr));
        }

        {
            let mut callbacks = BUTTON_CALLBACKS.lock();
            callbacks.remove(&widget_ptr);
        }

        {
            let callbacks = BUTTON_CALLBACKS.lock();
            assert!(callbacks.get(&widget_ptr).is_none());
        }
    }
}
