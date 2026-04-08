//! Add command implementation

use crate::cli::{AddOpts, AddTarget};
use crate::config::Config;
use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn run(opts: AddOpts) -> Result<()> {
    let project_dir = std::env::current_dir()?;
    
    // Load config
    let mut config = Config::load(&project_dir)?;

    match opts.target {
        AddTarget::Element { name } => add_element(&project_dir, &mut config, &name),
        AddTarget::Platform { name } => add_platform(&project_dir, &mut config, &name),
    }
}

fn add_element(project_dir: &Path, config: &mut Config, name: &str) -> Result<()> {
    println!(
        "{} {} element: {}",
        "Adding".green().bold(),
        "custom".cyan(),
        name.white().bold()
    );

    // Check if element already exists
    if config.elements.custom.contains(&name.to_string()) {
        bail!("Element '{}' already exists in project", name);
    }

    // Create element files for each enabled platform
    for platform in config.enabled_platforms() {
        match platform {
            "linux" => create_linux_element(project_dir, name)?,
            "macos" => create_macos_element(project_dir, name)?,
            "windows" => create_windows_element(project_dir, name)?,
            _ => continue,
        }
    }

    // Update config
    config.elements.custom.push(name.to_string());
    config.save(project_dir)?;

    println!();
    println!("{}", "✓ Element created!".green().bold());
    println!();
    println!("  {}", "Files created:".cyan());
    for platform in config.enabled_platforms() {
        let path = match platform {
            "linux" => format!("platforms/linux/src/elements/{}.cpp", name),
            "macos" => format!("platforms/macos/Sources/Elements/{}.swift", name),
            "windows" => format!("platforms/windows/src/{}.cs", name),
            _ => continue,
        };
        println!("    {}", path);
    }
    println!();
    println!(
        "  {} Remember to register the element in rnl_platform_init()",
        "Note:".yellow()
    );

    Ok(())
}

fn add_platform(project_dir: &Path, config: &mut Config, name: &str) -> Result<()> {
    println!(
        "{} {} platform: {}",
        "Adding".green().bold(),
        "support for".cyan(),
        name.white().bold()
    );

    match name {
        "linux" => {
            if config.platforms.linux.is_some() {
                bail!("Linux platform already enabled");
            }
            create_linux_platform(project_dir)?;
            config.platforms.linux = Some(crate::config::LinuxConfig {
                enabled: true,
                toolkit: "gtk4".to_string(),
                lang: "cpp".to_string(),
                min_version: Some("22.04".to_string()),
            });
        }
        "macos" => {
            if config.platforms.macos.is_some() {
                bail!("macOS platform already enabled");
            }
            create_macos_platform(project_dir)?;
            config.platforms.macos = Some(crate::config::MacOSConfig {
                enabled: true,
                toolkit: "appkit".to_string(),
                lang: "swift".to_string(),
                min_version: Some("12.0".to_string()),
            });
        }
        "windows" => {
            if config.platforms.windows.is_some() {
                bail!("Windows platform already enabled");
            }
            create_windows_platform(project_dir)?;
            config.platforms.windows = Some(crate::config::WindowsConfig {
                enabled: true,
                toolkit: "winui3".to_string(),
                lang: "csharp".to_string(),
                min_version: Some("10.0.17763.0".to_string()),
            });
        }
        _ => bail!("Unknown platform: {}. Valid options: linux, macos, windows", name),
    }

    config.save(project_dir)?;

    println!();
    println!("{}", "✓ Platform added!".green().bold());

    Ok(())
}

fn create_linux_element(project_dir: &Path, name: &str) -> Result<()> {
    let element_dir = project_dir.join("platforms/linux/src/elements");
    fs::create_dir_all(&element_dir)?;

    let snake_name = to_snake_case(name);
    let pascal_name = to_pascal_case(name);

    let content = format!(
        r#"// {}.cpp - Custom element implementation for Linux/GTK4
#include <rnl.h>
#include <gtk/gtk.h>
#include <cstring>

static rnl_widget_t {snake}_create() {{
    // TODO: Create your GTK widget here
    GtkWidget* widget = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    return widget;
}}

static void {snake}_set_attribute(rnl_widget_t widget, const char* name, const char* value) {{
    // TODO: Handle attribute updates
    // GtkWidget* w = GTK_WIDGET(widget);
}}

static void {snake}_set_callback(rnl_widget_t widget, const char* name, rnl_js_value_t callback) {{
    // TODO: Connect signals to JS callbacks
    // if (strcmp(name, "onSomething") == 0) {{
    //     g_signal_connect(widget, "signal-name", G_CALLBACK(...), callback);
    // }}
}}

static void {snake}_append_child(rnl_widget_t parent, rnl_widget_t child) {{
    // TODO: Add child widget if this element is a container
}}

static void {snake}_remove_child(rnl_widget_t parent, rnl_widget_t child) {{
    // TODO: Remove child widget if this element is a container
}}

static void {snake}_destroy(rnl_widget_t widget) {{
    // GTK handles ref counting, but clean up any custom data here
}}

const rnl_element_factory_t {snake}_factory = {{
    .name = "{name}",
    .create = {snake}_create,
    .set_attribute = {snake}_set_attribute,
    .set_callback = {snake}_set_callback,
    .append_child = {snake}_append_child,
    .insert_before = nullptr,
    .remove_child = {snake}_remove_child,
    .destroy = {snake}_destroy,
}};

void register_{snake}() {{
    rnl_register_element(&{snake}_factory);
}}
"#,
        name = name,
        snake = snake_name
    );

    fs::write(element_dir.join(format!("{}.cpp", name)), content)?;
    println!("  {} platforms/linux/src/elements/{}.cpp", "created".green(), name);

    Ok(())
}

fn create_macos_element(project_dir: &Path, name: &str) -> Result<()> {
    let element_dir = project_dir.join("platforms/macos/Sources/Elements");
    fs::create_dir_all(&element_dir)?;

    let pascal_name = to_pascal_case(name);

    let content = format!(
        r#"// {pascal}.swift - Custom element implementation for macOS
import AppKit

var {lower}Factory = rnl_element_factory_t(
    name: strdup("{name}"),
    
    create: {{
        // TODO: Create your AppKit view here
        let view = NSView()
        return Unmanaged.passRetained(view).toOpaque()
    }},
    
    set_attribute: {{ widget, name, value in
        guard let view = Unmanaged<NSView>.fromOpaque(widget!).takeUnretainedValue() as NSView?,
              let nameStr = name.map({{ String(cString: $0) }}),
              let valueStr = value.map({{ String(cString: $0) }}) else {{ return }}
        
        // TODO: Handle attribute updates
        switch nameStr {{
        default:
            break
        }}
    }},
    
    set_callback: {{ widget, name, callback in
        // TODO: Connect actions to JS callbacks
    }},
    
    append_child: {{ parent, child in
        // TODO: Add subview if this element is a container
    }},
    
    insert_before: nil,
    
    remove_child: {{ parent, child in
        // TODO: Remove subview if this element is a container
    }},
    
    destroy: {{ widget in
        Unmanaged<NSView>.fromOpaque(widget!).release()
    }}
)

func register{pascal}() {{
    withUnsafePointer(to: &{lower}Factory) {{ ptr in
        rnl_register_element(ptr)
    }}
}}
"#,
        name = name,
        pascal = pascal_name,
        lower = name.to_lowercase()
    );

    fs::write(element_dir.join(format!("{}.swift", pascal_name)), content)?;
    println!(
        "  {} platforms/macos/Sources/Elements/{}.swift",
        "created".green(),
        pascal_name
    );

    Ok(())
}

fn create_windows_element(project_dir: &Path, name: &str) -> Result<()> {
    let element_dir = project_dir.join("platforms/windows/src");
    fs::create_dir_all(&element_dir)?;

    let pascal_name = to_pascal_case(name);

    let content = format!(
        r#"// {pascal}.cs - Custom element implementation for Windows/WinUI3
using System;
using System.Runtime.InteropServices;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

public static class {pascal}Element
{{
    private static readonly rnl_element_factory_t Factory = new()
    {{
        name = Marshal.StringToHGlobalAnsi("{name}"),
        create = Create,
        set_attribute = SetAttribute,
        set_callback = SetCallback,
        append_child = AppendChild,
        insert_before = IntPtr.Zero,
        remove_child = RemoveChild,
        destroy = Destroy,
    }};

    [UnmanagedCallersOnly]
    private static IntPtr Create()
    {{
        // TODO: Create your WinUI control here
        var panel = new StackPanel();
        return GCHandle.ToIntPtr(GCHandle.Alloc(panel));
    }}

    [UnmanagedCallersOnly]
    private static void SetAttribute(IntPtr widget, IntPtr name, IntPtr value)
    {{
        var panel = (StackPanel)GCHandle.FromIntPtr(widget).Target!;
        var nameStr = Marshal.PtrToStringUTF8(name);
        var valueStr = Marshal.PtrToStringUTF8(value);

        // TODO: Handle attribute updates
        switch (nameStr)
        {{
            default:
                break;
        }}
    }}

    [UnmanagedCallersOnly]
    private static void SetCallback(IntPtr widget, IntPtr name, IntPtr callback)
    {{
        // TODO: Connect events to JS callbacks
    }}

    [UnmanagedCallersOnly]
    private static void AppendChild(IntPtr parent, IntPtr child)
    {{
        // TODO: Add child if this element is a container
    }}

    [UnmanagedCallersOnly]
    private static void RemoveChild(IntPtr parent, IntPtr child)
    {{
        // TODO: Remove child if this element is a container
    }}

    [UnmanagedCallersOnly]
    private static void Destroy(IntPtr widget)
    {{
        GCHandle.FromIntPtr(widget).Free();
    }}

    public static void Register()
    {{
        unsafe
        {{
            fixed (rnl_element_factory_t* ptr = &Factory)
            {{
                NativeMethods.rnl_register_element((IntPtr)ptr);
            }}
        }}
    }}
}}
"#,
        name = name,
        pascal = pascal_name
    );

    fs::write(element_dir.join(format!("{}.cs", pascal_name)), content)?;
    println!(
        "  {} platforms/windows/src/{}.cs",
        "created".green(),
        pascal_name
    );

    Ok(())
}

fn create_linux_platform(project_dir: &Path) -> Result<()> {
    let platform_dir = project_dir.join("platforms/linux/src/elements");
    fs::create_dir_all(&platform_dir)?;

    // Create platform.cpp
    let platform_cpp = r#"// platform.cpp - Linux platform initialization
#include <rnl.h>
#include <adwaita.h>
#include <gtk/gtk.h>

static AdwApplication* app = nullptr;
static GtkWidget* main_window = nullptr;
static GtkWidget* root_container = nullptr;

// Forward declarations for element registration
extern void register_box();
extern void register_button();
extern void register_text();

extern "C" void rnl_platform_init() {
    // Register built-in elements
    register_box();
    register_button();
    register_text();
}

static void on_activate(GtkApplication* application, gpointer user_data) {
    main_window = adw_application_window_new(application);
    gtk_window_set_title(GTK_WINDOW(main_window), "RNL App");
    gtk_window_set_default_size(GTK_WINDOW(main_window), 800, 600);
    
    root_container = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    adw_application_window_set_content(ADW_APPLICATION_WINDOW(main_window), root_container);
    
    gtk_window_present(GTK_WINDOW(main_window));
}

extern "C" rnl_widget_t rnl_platform_create_window(
    const char* title,
    int32_t width,
    int32_t height
) {
    app = adw_application_new("com.rnl.app", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(on_activate), nullptr);
    return main_window;
}

extern "C" rnl_widget_t rnl_platform_get_root_container(rnl_widget_t window) {
    return root_container;
}

extern "C" int32_t rnl_platform_run() {
    return g_application_run(G_APPLICATION(app), 0, nullptr);
}

extern "C" void rnl_platform_quit() {
    g_application_quit(G_APPLICATION(app));
}

extern "C" void rnl_platform_schedule_main(void (*callback)(void*), void* data) {
    g_idle_add_once((GSourceOnceFunc)callback, data);
}
"#;

    fs::write(
        project_dir.join("platforms/linux/src/platform.cpp"),
        platform_cpp,
    )?;
    println!("  {} platforms/linux/src/platform.cpp", "created".green());

    // Create main.cpp
    let main_cpp = r#"// main.cpp - Linux entry point
#include <rnl.h>

int main(int argc, char** argv) {
    return rnl_main(nullptr, argc, argv);
}
"#;

    fs::write(project_dir.join("platforms/linux/src/main.cpp"), main_cpp)?;
    println!("  {} platforms/linux/src/main.cpp", "created".green());

    Ok(())
}

fn create_macos_platform(project_dir: &Path) -> Result<()> {
    let platform_dir = project_dir.join("platforms/macos/Sources/Elements");
    fs::create_dir_all(&platform_dir)?;

    // Create Platform.swift
    let platform_swift = r#"// Platform.swift - macOS platform initialization
import AppKit

var mainWindow: NSWindow?
var rootView: NSStackView?

@_cdecl("rnl_platform_init")
func platformInit() {
    NSApplication.shared.setActivationPolicy(.regular)
    // Register built-in elements
    registerBox()
    registerButton()
    registerText()
}

@_cdecl("rnl_platform_create_window")
func createWindow(title: UnsafePointer<CChar>, width: Int32, height: Int32) -> UnsafeMutableRawPointer? {
    let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: CGFloat(width), height: CGFloat(height)),
        styleMask: [.titled, .closable, .resizable, .miniaturizable],
        backing: .buffered,
        defer: false
    )
    window.title = String(cString: title)
    window.center()
    
    let stack = NSStackView()
    stack.orientation = .vertical
    stack.alignment = .leading
    stack.translatesAutoresizingMaskIntoConstraints = false
    window.contentView = stack
    
    rootView = stack
    mainWindow = window
    
    return Unmanaged.passRetained(window).toOpaque()
}

@_cdecl("rnl_platform_get_root_container")
func getRootContainer(window: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer? {
    guard let root = rootView else { return nil }
    return Unmanaged.passUnretained(root).toOpaque()
}

@_cdecl("rnl_platform_run")
func platformRun() -> Int32 {
    mainWindow?.makeKeyAndOrderFront(nil)
    NSApplication.shared.activate(ignoringOtherApps: true)
    NSApplication.shared.run()
    return 0
}

@_cdecl("rnl_platform_quit")
func platformQuit() {
    NSApplication.shared.terminate(nil)
}

@_cdecl("rnl_platform_schedule_main")
func scheduleMain(callback: @escaping @convention(c) (UnsafeMutableRawPointer?) -> Void, data: UnsafeMutableRawPointer?) {
    DispatchQueue.main.async {
        callback(data)
    }
}
"#;

    fs::write(
        project_dir.join("platforms/macos/Sources/Platform.swift"),
        platform_swift,
    )?;
    println!("  {} platforms/macos/Sources/Platform.swift", "created".green());

    Ok(())
}

fn create_windows_platform(project_dir: &Path) -> Result<()> {
    let platform_dir = project_dir.join("platforms/windows/src");
    fs::create_dir_all(&platform_dir)?;

    // Create Platform.cs
    let platform_cs = r#"// Platform.cs - Windows platform initialization
using System;
using System.Runtime.InteropServices;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

public static class Platform
{
    private static Application? app;
    private static Window? mainWindow;
    private static StackPanel? rootPanel;

    [UnmanagedCallersOnly(EntryPoint = "rnl_platform_init")]
    public static void Init()
    {
        WinRT.ComWrappersSupport.InitializeComWrappers();
        // Register built-in elements
        BoxElement.Register();
        ButtonElement.Register();
        TextElement.Register();
    }

    [UnmanagedCallersOnly(EntryPoint = "rnl_platform_create_window")]
    public static IntPtr CreateWindow(IntPtr title, int width, int height)
    {
        var titleStr = Marshal.PtrToStringUTF8(title) ?? "RNL App";
        
        mainWindow = new Window
        {
            Title = titleStr,
        };
        
        rootPanel = new StackPanel
        {
            Orientation = Orientation.Vertical,
        };
        mainWindow.Content = rootPanel;
        
        return GCHandle.ToIntPtr(GCHandle.Alloc(mainWindow));
    }

    [UnmanagedCallersOnly(EntryPoint = "rnl_platform_get_root_container")]
    public static IntPtr GetRootContainer(IntPtr window)
    {
        return GCHandle.ToIntPtr(GCHandle.Alloc(rootPanel));
    }

    [UnmanagedCallersOnly(EntryPoint = "rnl_platform_run")]
    public static int Run()
    {
        mainWindow?.Activate();
        return 0;
    }

    [UnmanagedCallersOnly(EntryPoint = "rnl_platform_quit")]
    public static void Quit()
    {
        mainWindow?.Close();
    }

    [UnmanagedCallersOnly(EntryPoint = "rnl_platform_schedule_main")]
    public static void ScheduleMain(IntPtr callback, IntPtr data)
    {
        mainWindow?.DispatcherQueue.TryEnqueue(() =>
        {
            var fn = Marshal.GetDelegateForFunctionPointer<Action<IntPtr>>(callback);
            fn(data);
        });
    }
}
"#;

    fs::write(project_dir.join("platforms/windows/src/Platform.cs"), platform_cs)?;
    println!("  {} platforms/windows/src/Platform.cs", "created".green());

    Ok(())
}

// Helper functions for case conversion
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else if c == '-' {
            result.push('_');
        } else {
            result.push(c);
        }
    }
    result
}

fn to_pascal_case(s: &str) -> String {
    s.split(|c| c == '-' || c == '_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect()
}
