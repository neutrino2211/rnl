# RNL - React Native Libre

> A multi-platform React Native framework where the core is Rust, but platform implementations can be written in the native language of each platform (Swift, C#, C++, etc.)

**Status:** Early development (M1 complete)

## Vision

React Native, but truly native — not a bridge to a single runtime, but a thin coordination layer that lets each platform shine in its own language.

- **Linux:** GTK4/libadwaita via C++ or Rust
- **macOS:** AppKit/SwiftUI via Swift
- **Windows:** WinUI 3 via C#
- **Future:** Android (Kotlin), iOS (Swift), Web (WASM)

Developers write React components. Platform authors implement native elements in their preferred language. The Rust core orchestrates everything.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                           rnl CLI                               │
│      (project scaffold, build orchestration, linking)           │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
        ┌─────────────────────────┼─────────────────────────────┐
        ▼                         ▼                             ▼
┌───────────────┐    ┌────────────────────┐    ┌───────────────────┐
│   RUST CORE   │    │ PLATFORM ELEMENTS  │    │    JS BUNDLE      │
│  (librnl.a)   │    │   (.o / .obj)      │    │   (esbuild)       │
├───────────────┤    ├────────────────────┤    └───────────────────┘
│ • QuickJS RT  │    │ Linux: C++/Rust    │
│ • Registry    │    │ macOS: Swift       │
│ • Reconciler  │    │ Windows: C#        │
│ • Event loop  │    │                    │
│ • C ABI       │    │ Implements:        │
└───────┬───────┘    │ rnl_element_*      │
        │            └──────────┬─────────┘
        │                       │
        └───────────┬───────────┘
                    ▼
            ┌──────────────┐
            │ FINAL BINARY │
            │   (linked)   │
            └──────────────┘
```

## Project Structure

```
rnl/
├── Cargo.toml           # Workspace manifest
├── core/                # Rust core library (QuickJS, registry, bridge)
│   ├── src/
│   │   ├── lib.rs       # C API entry points
│   │   ├── runtime.rs   # QuickJS setup
│   │   ├── registry.rs  # Element factory registry
│   │   └── bridge.rs    # JS ↔ native bridge
│   └── include/
│       └── rnl.h        # C header for platform code
├── rnl-cli/             # Build tool
│   └── src/
│       ├── main.rs
│       ├── cli.rs       # clap argument parsing
│       ├── config.rs    # rnl.toml parsing
│       └── commands/    # init, build, run, add, doctor, clean
└── README.md
```

## Quick Start (WIP)

```bash
# Build the CLI
cargo build -p rnl-cli

# Create a new project
./target/debug/rnl init my-app --platforms linux

# Build and run (requires GTK4 dev libraries)
cd my-app
npm install
rnl build
rnl run
```

## Development Milestones

### M1: Core Works ✅
- [x] CLI creates projects (`rnl init`)
- [x] Rust core compiles with QuickJS
- [x] Element registry works
- [x] JS bridge connects JS ↔ native calls

### M2: Linux MVP (next)
- [ ] GTK4 platform bootstrap
- [ ] Core elements (box, text, button)
- [ ] Counter app runs on Linux

### M3: Cross-Platform
- [ ] macOS via Swift/AppKit
- [ ] Windows via C#/WinUI3

## License

MIT
