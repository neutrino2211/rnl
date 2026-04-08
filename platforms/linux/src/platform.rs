//! Platform implementation for GTK4/Linux
//!
//! Implements the C ABI functions that the RNL core calls:
//! - rnl_platform_init()
//! - rnl_platform_create_window()
//! - rnl_platform_get_root_container()
//! - rnl_platform_run()
//! - rnl_platform_quit()
//! - rnl_platform_schedule_main()

use gtk4::prelude::*;
use gtk4::{glib, Application, ApplicationWindow, Box as GtkBox, Orientation};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::ffi::{c_char, c_int, c_void, CStr};

use crate::elements;

/// Global application state
static APP_STATE: OnceCell<Mutex<AppState>> = OnceCell::new();

struct AppState {
    app: Option<Application>,
    main_window: Option<ApplicationWindow>,
    root_container: Option<GtkBox>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            app: None,
            main_window: None,
            root_container: None,
        }
    }
}

fn get_app_state() -> &'static Mutex<AppState> {
    APP_STATE.get_or_init(|| Mutex::new(AppState::default()))
}

/// Called once at startup to initialize the platform
/// Platform should register all its elements here
#[no_mangle]
pub extern "C" fn rnl_platform_init() {
    log::info!("GTK4 platform initializing...");

    // GTK4 init is done lazily when the Application is created
    // Register all elements with the core
    elements::register_all_elements();

    log::info!("GTK4 platform initialized, elements registered");
}

/// Create the root application window
#[no_mangle]
pub extern "C" fn rnl_platform_create_window(
    title: *const c_char,
    width: i32,
    height: i32,
) -> *mut c_void {
    let title_str = if title.is_null() {
        "RNL App".to_string()
    } else {
        unsafe { CStr::from_ptr(title) }
            .to_string_lossy()
            .into_owned()
    };

    log::info!(
        "Creating window: {} ({}x{})",
        title_str,
        width,
        height
    );

    // Create the GTK application
    let app = Application::builder()
        .application_id("com.rnl.app")
        .build();

    // Store app in state
    {
        let mut state = get_app_state().lock();
        state.app = Some(app.clone());
    }

    // Set up the activate handler
    let title_for_closure = title_str.clone();
    app.connect_activate(move |app| {
        // Create window
        let window = ApplicationWindow::builder()
            .application(app)
            .title(&title_for_closure)
            .default_width(width)
            .default_height(height)
            .build();

        // Create root container (vertical box)
        let root_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(0)
            .hexpand(true)
            .vexpand(true)
            .build();

        window.set_child(Some(&root_box));

        // Store window and root in state
        {
            let mut state = get_app_state().lock();
            state.main_window = Some(window.clone());
            state.root_container = Some(root_box.clone());
        }

        // Notify the core about the root container
        extern "C" {
            fn rnl_set_root_container(root: *mut c_void);
        }

        let root_ptr = root_box.as_ptr() as *mut c_void;
        unsafe {
            rnl_set_root_container(root_ptr);
        }

        // Show the window
        window.present();

        log::info!("Window created and presented");
    });

    // Return the app pointer
    // Note: In GTK4, we don't directly return a window pointer
    // The window is created in the activate handler
    app.as_ptr() as *mut c_void
}

/// Get the content/root container of a window
#[no_mangle]
pub extern "C" fn rnl_platform_get_root_container(_window: *mut c_void) -> *mut c_void {
    let state = get_app_state().lock();
    match &state.root_container {
        Some(root) => root.as_ptr() as *mut c_void,
        None => {
            log::warn!("get_root_container called but no root container exists");
            std::ptr::null_mut()
        }
    }
}

/// Start the platform event loop (blocks until app exits)
#[no_mangle]
pub extern "C" fn rnl_platform_run() -> c_int {
    log::info!("Starting GTK4 event loop...");

    let app = {
        let state = get_app_state().lock();
        state.app.clone()
    };

    match app {
        Some(app) => {
            // Run the GTK application
            // Note: run() expects command-line args, but we've already parsed them
            let exit_code = app.run_with_args::<String>(&[]);
            exit_code.into()
        }
        None => {
            log::error!("rnl_platform_run called but no app exists");
            1
        }
    }
}

/// Request the event loop to stop
#[no_mangle]
pub extern "C" fn rnl_platform_quit() {
    log::info!("Platform quit requested");

    let state = get_app_state().lock();
    if let Some(app) = &state.app {
        app.quit();
    }
}

/// Schedule a callback to run on the main/UI thread
#[no_mangle]
pub extern "C" fn rnl_platform_schedule_main(
    callback: Option<extern "C" fn(*mut c_void)>,
    data: *mut c_void,
) {
    if let Some(cb) = callback {
        // Use glib::idle_add_once to schedule on the main thread
        // We need to box the data pointer to safely pass it to the closure
        let data_ptr = data as usize; // Convert to usize for Send

        glib::idle_add_once(move || {
            let data = data_ptr as *mut c_void;
            cb(data);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_initialization() {
        // Just verify we can create the app state
        let state = get_app_state();
        let guard = state.lock();
        assert!(guard.app.is_none());
    }
}
