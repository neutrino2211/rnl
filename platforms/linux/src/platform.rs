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
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr};

use crate::elements;

/// Thread-local application state (GTK objects must stay on main thread)
thread_local! {
    static APP_STATE: RefCell<AppState> = RefCell::new(AppState::default());
}

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

/// Called once at startup to initialize the platform
/// Platform should register all its elements here
#[no_mangle]
pub extern "C" fn rnl_platform_init() {
    log::info!("GTK4 platform initializing...");

    // Initialize GTK4 - this is required before creating any widgets
    gtk4::init().expect("Failed to initialize GTK4");
    
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
) -> c_int {
    let title_str = if title.is_null() {
        "RNL App".to_string()
    } else {
        unsafe { CStr::from_ptr(title) }
            .to_string_lossy()
            .into_owned()
    };

    log::info!("Creating window: {} ({}x{})", title_str, width, height);

    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        
        // Create application if needed
        if state.app.is_none() {
            let app = Application::builder()
                .application_id("dev.rnl.app")
                .build();
            state.app = Some(app);
        }

        let app = state.app.as_ref().unwrap();

        // Create window and root container
        let title_for_closure = title_str.clone();
        let activate_called = std::rc::Rc::new(std::cell::Cell::new(false));
        let activate_called_clone = activate_called.clone();
        
        app.connect_activate(move |app| {
            if activate_called_clone.get() {
                return; // Already activated
            }
            activate_called_clone.set(true);
            
            let window = ApplicationWindow::builder()
                .application(app)
                .title(&title_for_closure)
                .default_width(width)
                .default_height(height)
                .build();

            let root = GtkBox::new(Orientation::Vertical, 0);
            window.set_child(Some(&root));

            // Store in thread-local (we need to do this from within the callback)
            APP_STATE.with(|s| {
                let mut s = s.borrow_mut();
                s.main_window = Some(window.clone());
                s.root_container = Some(root);
            });

            window.present();
        });

        0 // Success
    })
}

/// Get the root container widget for adding children
#[no_mangle]
pub extern "C" fn rnl_platform_get_root_container() -> *mut c_void {
    APP_STATE.with(|state| {
        let state = state.borrow();
        if let Some(root) = &state.root_container {
            // Return a raw pointer to the GtkBox
            // We use Box::into_raw on a boxed clone so it stays valid
            Box::into_raw(Box::new(root.clone())) as *mut c_void
        } else {
            std::ptr::null_mut()
        }
    })
}

/// Run the main event loop
#[no_mangle]
pub extern "C" fn rnl_platform_run() -> c_int {
    log::info!("Starting GTK4 main loop...");

    APP_STATE.with(|state| {
        let state = state.borrow();
        if let Some(app) = &state.app {
            // Run with empty args - window is created in activate handler
            app.run_with_args::<&str>(&[]);
            0
        } else {
            log::error!("No application created");
            1
        }
    })
}

/// Quit the application
#[no_mangle]
pub extern "C" fn rnl_platform_quit() {
    log::info!("Quitting GTK4 application...");

    APP_STATE.with(|state| {
        let state = state.borrow();
        if let Some(app) = &state.app {
            app.quit();
        }
    });
}

/// Schedule a callback to run on the main thread
#[no_mangle]
pub extern "C" fn rnl_platform_schedule_main(callback: extern "C" fn(*mut c_void), user_data: *mut c_void) {
    // Safety: GTK's idle_add runs on main thread
    // user_data is passed through unchanged
    let data = user_data as usize; // Convert to usize for Send
    
    glib::idle_add_once(move || {
        callback(data as *mut c_void);
    });
}

/// Get the platform name for logging/debugging
#[no_mangle]
pub extern "C" fn rnl_platform_name() -> *const c_char {
    static NAME: &[u8] = b"GTK4/Linux\0";
    NAME.as_ptr() as *const c_char
}
