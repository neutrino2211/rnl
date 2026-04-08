# RNL Framework M2: Linux MVP

## Completion Status

### Sprint 6: GTK4 Platform Bootstrap ✅
- Created `platforms/linux/` with Rust GTK4 crate
- Implemented `main.rs` that initializes GTK4 app + RNL core
- Platform loads `target/bundle.js` via the core runtime
- Registered platform with core via C ABI

**Files created:**
- `platforms/linux/Cargo.toml` - GTK4 dependencies
- `platforms/linux/build.rs` - Build configuration
- `platforms/linux/src/main.rs` - Entry point
- `platforms/linux/src/platform.rs` - Platform implementation

**C ABI functions implemented:**
- `rnl_platform_init()` - Register elements
- `rnl_platform_create_window()` - Create GTK window
- `rnl_platform_get_root_container()` - Get content area
- `rnl_platform_run()` - Start GTK event loop
- `rnl_platform_quit()` - Request exit
- `rnl_platform_schedule_main()` - Schedule callback on main thread

### Sprint 7: Core Elements (box, text, button) ✅
- Implemented `GtkBox` wrapper registering as "box" element
- Implemented `GtkLabel` wrapper registering as "text" element
- Implemented `GtkButton` wrapper registering as "button" element
- All implement `RnlElementFactory` C ABI

**Files created:**
- `platforms/linux/src/elements/mod.rs` - Element registration
- `platforms/linux/src/elements/box_element.rs` - GtkBox wrapper
- `platforms/linux/src/elements/text_element.rs` - GtkLabel wrapper
- `platforms/linux/src/elements/button_element.rs` - GtkButton wrapper

**Attributes supported:**
| Element | Attributes |
|---------|------------|
| box | orientation, spacing, homogeneous, hexpand, vexpand, margin*, halign, valign |
| text | children, text, selectable, wrap, ellipsize, justify, xalign, yalign, markup |
| button | label, children, enabled, disabled, has-frame, icon-name, onClick |

### Sprint 8: Build Pipeline Integration ✅
- Updated `rnl build --platform linux` to detect Rust vs C++ platforms
- `build_linux_rust_from_dir()` compiles GTK4 platform with cargo
- Detects RNL workspace and builds from there
- Links platform with librnl.a from core
- Output binary to `target/linux/app`

**Build flow:**
1. Bundle JS → `target/bundle.js` (always succeeds)
2. Build core (`cargo build -p rnl-core`)
3. Detect platform location (project or RNL workspace)
4. Build platform with GTK4

### Sprint 9: Counter App Runs ⚠️ (Requires GTK4)
- Test pipeline: `rnl init test-app && cd test-app && rnl build`
- JS bundle creates successfully
- Counter app template generated with +/- buttons
- **Requires GTK4 dev libraries to compile native code**

## Requirements

### To build on Linux:
```bash
# Ubuntu/Debian
sudo apt install libgtk-4-dev pkg-config

# Fedora
sudo dnf install gtk4-devel

# Arch
sudo pacman -S gtk4
```

### What works without GTK4:
- JS bundling (`target/bundle.js` created)
- Project scaffolding (`rnl init`)
- Core library tests (`cargo test -p rnl-core`)
- CLI tests (`cargo test -p rnl-cli`)

## Architecture

```
rnl/
├── core/               # Rust runtime (QuickJS + element registry)
│   ├── src/
│   │   ├── lib.rs      # Entry point
│   │   ├── runtime.rs  # JS runtime (QuickJS)
│   │   ├── bridge.rs   # JS ↔ Native bridge
│   │   ├── registry.rs # Element factory registry
│   │   └── callbacks.rs # Callback management
│   └── include/
│       └── rnl.h       # C ABI header
├── platforms/
│   └── linux/          # GTK4 platform
│       └── src/
│           ├── main.rs
│           ├── platform.rs
│           └── elements/
│               ├── box_element.rs
│               ├── text_element.rs
│               └── button_element.rs
└── rnl-cli/            # Command line tool
    └── src/
        └── commands/
            └── build.rs # Build pipeline
```

## Testing on a GTK4 Machine

```bash
# Clone and build
git clone https://github.com/neutrino2211/rnl.git
cd rnl

# Install GTK4 (Ubuntu/Debian)
sudo apt install libgtk-4-dev pkg-config

# Build the framework
cargo build --workspace

# Create and run a test app
./target/debug/rnl init counter-app --platforms linux
cd counter-app
../target/debug/rnl build --platform linux
../target/debug/rnl run
```

## Known Limitations

1. **Callbacks not fully wired**: The callback system stores IDs but the actual JS function invocation from native code needs more work (requires storing QuickJS Function objects properly)

2. **No re-render**: State changes in React don't trigger re-renders yet (needs reconciliation)

3. **Style not fully implemented**: CSS-like style objects are partially supported

## Future Work

- Wire up callback invocation (native → JS)
- Implement React reconciliation for re-renders
- Add more elements (input, scroll, etc.)
- macOS platform (AppKit)
- Windows platform (.NET)
