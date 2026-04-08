//! Text element - GtkLabel wrapper
//!
//! A widget that displays text. Maps to GtkLabel.

use gtk4::prelude::*;
use gtk4::{Label, Widget};
use std::ffi::{c_char, c_void, CStr};

use super::RnlElementFactory;

/// Static name for the element (null-terminated C string)
static TEXT_NAME: &[u8] = b"text\0";

/// The element factory for "text"
pub static TEXT_FACTORY: RnlElementFactory = RnlElementFactory {
    name: TEXT_NAME.as_ptr() as *const c_char,
    create: Some(text_create),
    set_attribute: Some(text_set_attribute),
    set_callback: Some(text_set_callback),
    append_child: Some(text_append_child),
    insert_before: None, // Text elements don't have children in the DOM sense
    remove_child: Some(text_remove_child),
    destroy: Some(text_destroy),
};

/// Create a new GtkLabel
extern "C" fn text_create() -> *mut c_void {
    log::debug!("Creating GtkLabel");

    let label = Label::builder()
        .label("")
        .xalign(0.0) // Left align by default
        .build();

    let ptr = label.as_ptr() as *mut c_void;
    std::mem::forget(label);

    log::debug!("Created GtkLabel at {:?}", ptr);
    ptr
}

/// Set an attribute on the text/label
extern "C" fn text_set_attribute(widget: *mut c_void, name: *const c_char, value: *const c_char) {
    if widget.is_null() || name.is_null() || value.is_null() {
        return;
    }

    let label: Label = unsafe {
        let ptr = widget as *mut gtk4::ffi::GtkLabel;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    let name_str = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    let value_str = unsafe { CStr::from_ptr(value) }.to_string_lossy();

    log::debug!("text.set_attribute({}, {})", name_str, value_str);

    match name_str.as_ref() {
        // Main content attribute - this is what React text children become
        "children" | "text" | "label" => {
            label.set_text(&value_str);
        }
        "selectable" => {
            label.set_selectable(value_str == "true");
        }
        "wrap" => {
            label.set_wrap(value_str == "true");
        }
        "lines" | "max-lines" => {
            if let Ok(lines) = value_str.parse::<i32>() {
                label.set_lines(lines);
            }
        }
        "ellipsize" => {
            let mode = match value_str.as_ref() {
                "start" => gtk4::pango::EllipsizeMode::Start,
                "middle" => gtk4::pango::EllipsizeMode::Middle,
                "end" => gtk4::pango::EllipsizeMode::End,
                _ => gtk4::pango::EllipsizeMode::None,
            };
            label.set_ellipsize(mode);
        }
        "justify" | "text-align" => {
            let justify = match value_str.as_ref() {
                "left" | "start" => gtk4::Justification::Left,
                "right" | "end" => gtk4::Justification::Right,
                "center" => gtk4::Justification::Center,
                "fill" => gtk4::Justification::Fill,
                _ => gtk4::Justification::Left,
            };
            label.set_justify(justify);
        }
        "xalign" => {
            if let Ok(align) = value_str.parse::<f32>() {
                label.set_xalign(align);
            }
        }
        "yalign" => {
            if let Ok(align) = value_str.parse::<f32>() {
                label.set_yalign(align);
            }
        }
        "use-markup" | "markup" => {
            label.set_use_markup(value_str == "true");
        }
        // Style attributes
        "margin" => {
            if let Ok(margin) = value_str.parse::<i32>() {
                label.set_margin_start(margin);
                label.set_margin_end(margin);
                label.set_margin_top(margin);
                label.set_margin_bottom(margin);
            }
        }
        "halign" => {
            let align = parse_alignment(&value_str);
            label.set_halign(align);
        }
        "valign" => {
            let align = parse_alignment(&value_str);
            label.set_valign(align);
        }
        "hexpand" => {
            label.set_hexpand(value_str == "true");
        }
        "vexpand" => {
            label.set_vexpand(value_str == "true");
        }
        _ => {
            log::debug!("Unknown text attribute: {}", name_str);
        }
    }

    std::mem::forget(label);
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

/// Set a callback on the text (labels don't typically have callbacks)
extern "C" fn text_set_callback(
    _widget: *mut c_void,
    name: *const c_char,
    _callback: *mut c_void,
) {
    if name.is_null() {
        return;
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    log::debug!("text.set_callback({}) - labels don't have callbacks", name_str);
}

/// Append a child to the text - for GtkLabel, this means setting the text
extern "C" fn text_append_child(_parent: *mut c_void, _child: *mut c_void) {
    // Text elements in React Native work differently
    // The text content comes via setAttribute("children", text)
    log::debug!("text.append_child - text content comes via attributes");
}

/// Remove a child from the text
extern "C" fn text_remove_child(_parent: *mut c_void, _child: *mut c_void) {
    log::debug!("text.remove_child - no-op for labels");
}

/// Destroy the text widget
extern "C" fn text_destroy(widget: *mut c_void) {
    if widget.is_null() {
        return;
    }

    log::debug!("text.destroy");

    let label: Label = unsafe {
        let ptr = widget as *mut gtk4::ffi::GtkLabel;
        gtk4::glib::translate::from_glib_none(ptr)
    };

    drop(label);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_name() {
        let name = unsafe { CStr::from_ptr(TEXT_FACTORY.name) };
        assert_eq!(name.to_str().unwrap(), "text");
    }

    #[test]
    fn test_factory_functions_exist() {
        assert!(TEXT_FACTORY.create.is_some());
        assert!(TEXT_FACTORY.set_attribute.is_some());
        assert!(TEXT_FACTORY.destroy.is_some());
    }
}
