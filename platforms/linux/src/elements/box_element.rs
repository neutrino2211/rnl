//! Box element - GtkBox wrapper
//!
//! A container that arranges children in a single row or column.
//! Maps to GtkBox with orientation and spacing properties.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, Widget};
use std::ffi::{c_char, c_void, CStr};

use super::RnlElementFactory;

/// Static name for the element (null-terminated C string)
static BOX_NAME: &[u8] = b"box\0";

/// The element factory for "box"
pub static BOX_FACTORY: RnlElementFactory = RnlElementFactory {
    name: BOX_NAME.as_ptr() as *const c_char,
    create: Some(box_create),
    set_attribute: Some(box_set_attribute),
    set_callback: Some(box_set_callback),
    append_child: Some(box_append_child),
    insert_before: Some(box_insert_before),
    remove_child: Some(box_remove_child),
    destroy: Some(box_destroy),
};

/// Create a new GtkBox
extern "C" fn box_create() -> *mut c_void {
    log::debug!("Creating GtkBox");

    let gtk_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .hexpand(true)
        .vexpand(false)
        .build();

    // Prevent GTK from garbage collecting this
    // We'll manually drop it in destroy
    let ptr = gtk_box.as_ptr() as *mut c_void;
    std::mem::forget(gtk_box);

    log::debug!("Created GtkBox at {:?}", ptr);
    ptr
}

/// Set an attribute on the box
extern "C" fn box_set_attribute(widget: *mut c_void, name: *const c_char, value: *const c_char) {
    if widget.is_null() || name.is_null() || value.is_null() {
        return;
    }

    let gtk_box: GtkBox = unsafe {
        let ptr = widget as *mut gtk4::ffi::GtkBox;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    let name_str = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    let value_str = unsafe { CStr::from_ptr(value) }.to_string_lossy();

    log::debug!("box.set_attribute({}, {})", name_str, value_str);

    match name_str.as_ref() {
        "orientation" => {
            let orientation = match value_str.as_ref() {
                "horizontal" | "row" => Orientation::Horizontal,
                _ => Orientation::Vertical,
            };
            gtk_box.set_orientation(orientation);
        }
        "spacing" => {
            if let Ok(spacing) = value_str.parse::<i32>() {
                gtk_box.set_spacing(spacing);
            }
        }
        "homogeneous" => {
            gtk_box.set_homogeneous(value_str == "true");
        }
        "hexpand" => {
            gtk_box.set_hexpand(value_str == "true");
        }
        "vexpand" => {
            gtk_box.set_vexpand(value_str == "true");
        }
        "margin" => {
            if let Ok(margin) = value_str.parse::<i32>() {
                gtk_box.set_margin_start(margin);
                gtk_box.set_margin_end(margin);
                gtk_box.set_margin_top(margin);
                gtk_box.set_margin_bottom(margin);
            }
        }
        "marginStart" | "margin-start" => {
            if let Ok(margin) = value_str.parse::<i32>() {
                gtk_box.set_margin_start(margin);
            }
        }
        "marginEnd" | "margin-end" => {
            if let Ok(margin) = value_str.parse::<i32>() {
                gtk_box.set_margin_end(margin);
            }
        }
        "marginTop" | "margin-top" => {
            if let Ok(margin) = value_str.parse::<i32>() {
                gtk_box.set_margin_top(margin);
            }
        }
        "marginBottom" | "margin-bottom" => {
            if let Ok(margin) = value_str.parse::<i32>() {
                gtk_box.set_margin_bottom(margin);
            }
        }
        "halign" => {
            let align = parse_alignment(&value_str);
            gtk_box.set_halign(align);
        }
        "valign" => {
            let align = parse_alignment(&value_str);
            gtk_box.set_valign(align);
        }
        _ => {
            log::debug!("Unknown box attribute: {}", name_str);
        }
    }

    // Don't drop - we don't own this reference
    std::mem::forget(gtk_box);
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

/// Set a callback on the box (boxes don't typically have callbacks)
extern "C" fn box_set_callback(
    _widget: *mut c_void,
    name: *const c_char,
    _callback: *mut c_void,
) {
    if name.is_null() {
        return;
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    log::debug!("box.set_callback({}) - boxes don't have callbacks", name_str);
}

/// Append a child to the box
extern "C" fn box_append_child(parent: *mut c_void, child: *mut c_void) {
    if parent.is_null() || child.is_null() {
        log::warn!("box_append_child: null pointer");
        return;
    }

    let gtk_box: GtkBox = unsafe {
        let ptr = parent as *mut gtk4::ffi::GtkBox;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    let child_widget: Widget = unsafe {
        let ptr = child as *mut gtk4::ffi::GtkWidget;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    log::debug!("box.append_child");
    gtk_box.append(&child_widget);

    // Don't drop - we don't own these references
    std::mem::forget(gtk_box);
    std::mem::forget(child_widget);
}

/// Insert child before a reference widget
extern "C" fn box_insert_before(parent: *mut c_void, child: *mut c_void, before: *mut c_void) {
    if parent.is_null() || child.is_null() {
        return;
    }

    let gtk_box: GtkBox = unsafe {
        let ptr = parent as *mut gtk4::ffi::GtkBox;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    let child_widget: Widget = unsafe {
        let ptr = child as *mut gtk4::ffi::GtkWidget;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    if before.is_null() {
        // Insert at the end
        gtk_box.append(&child_widget);
    } else {
        let before_widget: Widget = unsafe {
            let ptr = before as *mut gtk4::ffi::GtkWidget;
            gtk4::glib::translate::from_glib_none(ptr)
        };

        gtk_box.insert_child_after(&child_widget, Some(&before_widget));
        std::mem::forget(before_widget);
    }

    std::mem::forget(gtk_box);
    std::mem::forget(child_widget);
}

/// Remove a child from the box
extern "C" fn box_remove_child(parent: *mut c_void, child: *mut c_void) {
    if parent.is_null() || child.is_null() {
        return;
    }

    let gtk_box: GtkBox = unsafe {
        let ptr = parent as *mut gtk4::ffi::GtkBox;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    let child_widget: Widget = unsafe {
        let ptr = child as *mut gtk4::ffi::GtkWidget;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    log::debug!("box.remove_child");
    gtk_box.remove(&child_widget);

    std::mem::forget(gtk_box);
    std::mem::forget(child_widget);
}

/// Destroy the box widget
extern "C" fn box_destroy(widget: *mut c_void) {
    if widget.is_null() {
        return;
    }

    log::debug!("box.destroy");

    // For GTK4, widgets are ref-counted
    // We need to unref the widget we created
    let gtk_box: GtkBox = unsafe {
        let ptr = widget as *mut gtk4::ffi::GtkBox;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    // Dropping the GtkBox will decrease its ref count
    drop(gtk_box);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: GTK4 tests require gtk::init() which needs a display
    // These tests are more for documentation purposes

    #[test]
    fn test_factory_name() {
        let name = unsafe { CStr::from_ptr(BOX_FACTORY.name) };
        assert_eq!(name.to_str().unwrap(), "box");
    }

    #[test]
    fn test_factory_functions_exist() {
        assert!(BOX_FACTORY.create.is_some());
        assert!(BOX_FACTORY.set_attribute.is_some());
        assert!(BOX_FACTORY.append_child.is_some());
        assert!(BOX_FACTORY.remove_child.is_some());
        assert!(BOX_FACTORY.destroy.is_some());
    }
}
