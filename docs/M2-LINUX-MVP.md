# RNL Framework M2: Linux MVP ✅

**Status: COMPLETE** — Counter app runs on GTK4 with working button callbacks!

## What Works

- ✅ JSX renders to native GTK4 widgets
- ✅ Button clicks invoke JS callbacks
- ✅ React useState updates UI
- ✅ Box, Text, Button elements
- ✅ Headless debugging mode

## Quick Start

```bash
# Install GTK4 (Ubuntu/Debian)
sudo apt install libgtk-4-dev pkg-config

# Clone and build
git clone https://github.com/neutrino2211/rnl.git
cd rnl
cargo build --workspace

# Create and run a counter app
./target/debug/rnl init my-app --platforms linux
cd my-app
../target/debug/rnl build --platform linux
./target/linux/my-app
```

## Architecture

```
rnl/
├── core/                   # Rust runtime (QuickJS + bridge)
│   ├── src/
│   │   ├── lib.rs          # C ABI entry points
│   │   ├── runtime.rs      # QuickJS setup + RNLNativeModule
│   │   ├── bridge.rs       # Widget handle management
│   │   ├── callbacks.rs    # Persistent JS function storage
│   │   ├── registry.rs     # Element factory registry
│   │   └── bin/headless.rs # Debug runner (no GTK needed)
│   └── Cargo.toml
├── platforms/
│   └── linux/              # GTK4 platform
│       └── src/
│           ├── main.rs
│           ├── platform.rs
│           └── elements/
│               ├── box_element.rs
│               ├── text_element.rs
│               └── button_element.rs
└── rnl-cli/                # `rnl` command
    └── src/
        └── commands/
            ├── init.rs     # Project scaffolding + JS shim
            └── build.rs    # Bundle + compile pipeline
```

## How It Works

### Render Flow
1. `rnl build` bundles `src/index.tsx` → `target/bundle.js`
2. App starts, QuickJS loads bundle
3. React components call `jsx()` → virtual DOM objects
4. `render()` walks VDOM, calls `RNLNativeModule.createNode()` etc.
5. Bridge creates GTK widgets via element factories
6. Widgets appended to root GtkBox → window displays

### Callback Flow
1. JS: `<button onClick={() => setCount(c + 1)} />`
2. `setCallback(handle, "onClick", fn)` stores fn as `Persistent<Function>`
3. GTK button `connect_clicked` fires on click
4. Native calls `rnl_invoke_callback(callback_id)`
5. Core restores Persistent, calls JS function
6. useState updates state, re-renders UI

## Debugging

### Headless Mode (no display needed)
```bash
cargo build -p rnl-core --bin rnl-headless
./target/debug/rnl-headless target/bundle.js
```

Output:
```
[BRIDGE] createNode("box") -> 2
[BRIDGE] setAttribute(2, "orientation", "vertical")
[BRIDGE] appendChild(1, 2)
...
=== UI Tree ===
ROOT
  <box> [orientation="vertical"]
    <text>
      TEXT: "Welcome!"
    <button> [label="+"]
```

### Verbose Logs
```bash
RUST_LOG=debug ./target/linux/my-app
```

## Supported Elements

| Element | Props | Callbacks |
|---------|-------|-----------|
| `<box>` | orientation, spacing, margin*, halign, valign, hexpand, vexpand | — |
| `<text>` | children (text content), selectable, wrap | — |
| `<button>` | label, enabled/disabled, icon-name | onClick |

## Known Limitations

1. **Naive re-rendering**: Full tree rebuilt on state change (no diffing yet)
2. **No input element**: Text input not implemented
3. **Style partial**: Only margin/padding/align from style prop

## Bugs Fixed During M2

1. **Blank window**: JSX shim captured `RNLNativeModule` at bundle load time before it was set. Fixed with lazy `getRNL()` accessor.

2. **Empty children**: `jsx()`/`jsxs()` were aliased to `createElement()` which expected rest args, but react-jsx transform passes children in props. Fixed with proper jsx/jsxs functions.

3. **Double appendChild**: Children were appended both inside `render()` and by parent loop. Fixed by passing `null` for child renders.

4. **Root not found**: `__root__` type had no factory. Fixed by treating it as "box" type.

5. **Callbacks not firing**: `rnl_invoke_callback` was a stub. Implemented with `Persistent<Function>` storage and proper invocation.

## Next Steps (M3+)

- [ ] Reconciliation (diff-based updates)
- [ ] Input element
- [ ] ScrollView
- [ ] Image element
- [ ] macOS platform (AppKit/Swift)
- [ ] Hot reload
