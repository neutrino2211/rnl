//! Project initialization command

use crate::cli::InitOpts;
use crate::config::Config;
use crate::templates::{ProjectData, TemplateRenderer};
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn run(opts: InitOpts) -> Result<()> {
    let project_name = &opts.name;
    let project_dir = opts
        .dir
        .map(|d| Path::new(&d).to_path_buf())
        .unwrap_or_else(|| Path::new(project_name).to_path_buf());

    // Parse platforms
    let platforms: Vec<&str> = opts.platforms.split(',').map(|s| s.trim()).collect();
    for p in &platforms {
        if !["linux", "macos", "windows"].contains(p) {
            bail!("Unknown platform: {}. Valid options: linux, macos, windows", p);
        }
    }

    println!(
        "{} {} {}",
        "Creating".green().bold(),
        "RNL project".cyan(),
        project_name.white().bold()
    );

    // Check if directory exists
    if project_dir.exists() {
        bail!(
            "Directory '{}' already exists. Use --dir to specify a different location.",
            project_dir.display()
        );
    }

    // Create project structure
    create_project_structure(&project_dir, project_name, &platforms)?;

    println!();
    println!("{}", "✓ Project created successfully!".green().bold());
    println!();
    println!("  {}", "Next steps:".cyan());
    println!("    cd {}", project_dir.display());
    println!("    npm install");
    println!("    rnl build");
    println!("    rnl run");
    println!();

    Ok(())
}

fn create_project_structure(project_dir: &Path, name: &str, platforms: &[&str]) -> Result<()> {
    // Create directories
    let dirs = [
        "",
        "src",
        "core",
        "core/src",
        "core/include",
    ];

    for dir in &dirs {
        fs::create_dir_all(project_dir.join(dir))?;
    }

    // Create platform directories for enabled platforms
    for platform in platforms {
        let platform_dir = match *platform {
            "linux" => "platforms/linux/src/elements",
            "macos" => "platforms/macos/Sources/Elements",
            "windows" => "platforms/windows/src",
            _ => continue,
        };
        fs::create_dir_all(project_dir.join(platform_dir))?;
    }

    // Generate project data
    let data = ProjectData::new(name, platforms);

    // Create rnl.toml
    let config = Config::default_for_project(name, platforms);
    config.save(project_dir)?;
    println!("  {} rnl.toml", "created".green());

    // Create package.json
    let package_json = generate_package_json(&data);
    fs::write(project_dir.join("package.json"), package_json)?;
    println!("  {} package.json", "created".green());

    // Create tsconfig.json
    let tsconfig = generate_tsconfig();
    fs::write(project_dir.join("tsconfig.json"), tsconfig)?;
    println!("  {} tsconfig.json", "created".green());

    // Create src/index.tsx
    let index_tsx = generate_index_tsx();
    fs::write(project_dir.join("src/index.tsx"), index_tsx)?;
    println!("  {} src/index.tsx", "created".green());

    // Create src/App.tsx
    let app_tsx = generate_app_tsx(name);
    fs::write(project_dir.join("src/App.tsx"), app_tsx)?;
    println!("  {} src/App.tsx", "created".green());

    // Create rnl shim package (for esbuild to resolve)
    let rnl_shim_dir = project_dir.join("node_modules/rnl");
    fs::create_dir_all(&rnl_shim_dir)?;
    fs::write(rnl_shim_dir.join("package.json"), generate_rnl_package_json())?;
    fs::write(rnl_shim_dir.join("index.js"), generate_rnl_shim())?;
    fs::write(rnl_shim_dir.join("index.d.ts"), generate_rnl_types())?;
    fs::write(rnl_shim_dir.join("jsx-runtime.js"), generate_rnl_jsx_runtime())?;
    fs::write(rnl_shim_dir.join("jsx-runtime.d.ts"), generate_rnl_jsx_runtime_types())?;
    println!("  {} node_modules/rnl (runtime shim)", "created".green());

    // Create .gitignore
    let gitignore = generate_gitignore();
    fs::write(project_dir.join(".gitignore"), gitignore)?;
    println!("  {} .gitignore", "created".green());

    // Create README.md
    let readme = generate_readme(name);
    fs::write(project_dir.join("README.md"), readme)?;
    println!("  {} README.md", "created".green());

    Ok(())
}

fn generate_package_json(data: &ProjectData) -> String {
    format!(
        r#"{{
  "name": "{}",
  "version": "{}",
  "description": "{}",
  "main": "src/index.tsx",
  "scripts": {{
    "build": "rnl build",
    "start": "rnl run",
    "typecheck": "tsc --noEmit"
  }},
  "dependencies": {{
    "react": "^18.2.0"
  }},
  "devDependencies": {{
    "@types/react": "^18.2.0",
    "typescript": "^5.3.0",
    "esbuild": "^0.20.0"
  }}
}}
"#,
        data.name, data.version, data.description
    )
}

fn generate_tsconfig() -> String {
    r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "jsxImportSource": "rnl",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "baseUrl": ".",
    "paths": {
      "rnl": ["./node_modules/rnl"]
    }
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "target", "dist"]
}
"#
    .to_string()
}

fn generate_index_tsx() -> String {
    r#"import { render } from 'rnl';
import { App } from './App';

render(<App />);
"#
    .to_string()
}

fn generate_app_tsx(name: &str) -> String {
    format!(
        r#"import {{ useState }} from 'rnl';

export function App() {{
    const [count, setCount] = useState(0);

    return (
        <box orientation="vertical" spacing={{12}} style={{{{ padding: 24 }}}}>
            <text>Welcome to {}!</text>
            
            <box orientation="horizontal" spacing={{8}}>
                <button label="-" onClick={{() => setCount(c => c - 1)}} />
                <text>{{String(count)}}</text>
                <button label="+" onClick={{() => setCount(c => c + 1)}} />
            </box>
            
            <button 
                label="Reset" 
                onClick={{() => setCount(0)}}
                enabled={{count !== 0}}
            />
        </box>
    );
}}
"#,
        name
    )
}

fn generate_gitignore() -> String {
    r#"# Build artifacts
/target
/dist
/node_modules

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Debug
*.log
"#
    .to_string()
}

fn generate_readme(name: &str) -> String {
    format!(
        r#"# {}

An application built with [RNL](https://github.com/neutrino2211/rnl) (React Native Libre).

## Getting Started

```bash
# Install dependencies
npm install

# Build the application
rnl build

# Run the application
rnl run
```

## Project Structure

```
{}/
├── src/              # React/TypeScript source code
│   ├── index.tsx     # Entry point
│   └── App.tsx       # Main app component
├── core/             # Rust core (usually not modified)
├── platforms/        # Platform-specific implementations
├── rnl.toml          # Project configuration
└── package.json      # Node.js dependencies
```

## Building for Release

```bash
rnl build --release
```

## Learn More

- [RNL Documentation](https://rnl.dev/docs)
- [React Documentation](https://react.dev)
"#,
        name, name
    )
}

fn generate_rnl_package_json() -> String {
    r#"{
  "name": "rnl",
  "version": "0.1.0",
  "main": "index.js",
  "types": "index.d.ts"
}
"#.to_string()
}

fn generate_rnl_shim() -> String {
    r#"// RNL Runtime Shim
// This bridges React to native RNL components via RNLNativeModule

// Lazy accessor - RNLNativeModule must be accessed dynamically because
// the bundle may be loaded before the native module is injected into globalThis
function getRNL() {
    return globalThis.RNLNativeModule || {};
}

// ============ Hooks Implementation ============

// Global render context
let currentComponentId = 0;
let hookIndex = 0;
const componentStates = new Map(); // componentId -> { hooks: [], element: vdom }

// Root render state for re-rendering
let rootComponent = null;
let rootElement = null;
let rootHandle = null;
let isRendering = false;
let pendingRerender = false;

function getComponentState(componentId) {
    if (!componentStates.has(componentId)) {
        componentStates.set(componentId, { hooks: [] });
    }
    return componentStates.get(componentId);
}

export function useState(initialValue) {
    const componentId = currentComponentId;
    const state = getComponentState(componentId);
    const idx = hookIndex++;
    
    if (state.hooks[idx] === undefined) {
        state.hooks[idx] = typeof initialValue === 'function' ? initialValue() : initialValue;
    }
    
    const setState = (newValue) => {
        const current = state.hooks[idx];
        const next = typeof newValue === 'function' ? newValue(current) : newValue;
        
        // Only re-render if value actually changed
        if (next !== current) {
            state.hooks[idx] = next;
            scheduleRerender();
        }
    };
    
    return [state.hooks[idx], setState];
}

function scheduleRerender() {
    if (isRendering) {
        // Already rendering, mark for another pass
        pendingRerender = true;
        return;
    }
    
    // Use setTimeout(0) to batch multiple setState calls
    setTimeout(() => {
        if (rootComponent && rootElement) {
            rerender();
        }
    }, 0);
}

export function useEffect(effect, deps) {
    const state = getComponentState();
    const idx = hookIndex++;
    
    const prevDeps = state.hooks[idx]?.deps;
    const hasChanged = !prevDeps || !deps || deps.some((d, i) => d !== prevDeps[i]);
    
    if (hasChanged) {
        if (state.hooks[idx]?.cleanup) {
            state.hooks[idx].cleanup();
        }
        const cleanup = effect();
        state.hooks[idx] = { deps, cleanup };
    }
}

export function useCallback(callback, deps) {
    const state = getComponentState();
    const idx = hookIndex++;
    
    const prevDeps = state.hooks[idx]?.deps;
    const hasChanged = !prevDeps || !deps || deps.some((d, i) => d !== prevDeps[i]);
    
    if (hasChanged) {
        state.hooks[idx] = { callback, deps };
    }
    
    return state.hooks[idx].callback;
}

export function useMemo(factory, deps) {
    const state = getComponentState();
    const idx = hookIndex++;
    
    const prevDeps = state.hooks[idx]?.deps;
    const hasChanged = !prevDeps || !deps || deps.some((d, i) => d !== prevDeps[i]);
    
    if (hasChanged) {
        state.hooks[idx] = { value: factory(), deps };
    }
    
    return state.hooks[idx].value;
}

export function useRef(initialValue) {
    const state = getComponentState();
    const idx = hookIndex++;
    
    if (state.hooks[idx] === undefined) {
        state.hooks[idx] = { current: initialValue };
    }
    
    return state.hooks[idx];
}

// Simple JSX runtime
// createElement is for React.createElement() style (children as extra args)
export function createElement(type, props, ...children) {
    return { type, props: { ...props, children: children.flat() } };
}

// jsx/jsxs are for react-jsx transform (children already in props)
export function jsx(type, props, key) {
    return { type, props, key };
}

export function jsxs(type, props, key) {
    return { type, props, key };
}

// ============ Render Implementation ============

// Internal render that creates widgets
function renderElement(element) {
    const RNL = getRNL();
    
    if (element === null || element === undefined) return null;
    if (typeof element === 'string' || typeof element === 'number') {
        return RNL.createText?.(String(element));
    }
    
    const { type, props } = element;
    
    // Function component
    if (typeof type === 'function') {
        // Each component instance gets a stable ID based on render order
        const componentId = currentComponentId++;
        const prevHookIndex = hookIndex;
        hookIndex = 0;
        
        const result = type(props);
        
        hookIndex = prevHookIndex;
        
        return renderElement(result);
    }
    
    // Native element
    const handle = RNL.createNode?.(type);
    if (!handle) {
        console.warn('Unknown element type:', type);
        return null;
    }
    
    // Set attributes
    if (props) {
        for (const [key, value] of Object.entries(props)) {
            if (key === 'children') continue;
            if (key.startsWith('on') && typeof value === 'function') {
                RNL.setCallback?.(handle, key, value);
            } else if (key === 'style' && typeof value === 'object') {
                for (const [styleProp, styleVal] of Object.entries(value)) {
                    RNL.setAttribute?.(handle, `style.${styleProp}`, String(styleVal));
                }
            } else {
                RNL.setAttribute?.(handle, key, String(value));
            }
        }
        
        // Render children
        const children = props.children || [];
        for (const child of Array.isArray(children) ? children : [children]) {
            const childHandle = renderElement(child);
            if (childHandle) {
                RNL.appendChild?.(handle, childHandle);
            }
        }
    }
    
    return handle;
}

// Clear all children from a container
function clearContainer(containerHandle) {
    const RNL = getRNL();
    // We need a way to clear children - for now we'll rely on the platform
    // to handle this via a special "clear" call or by tracking children
    RNL.clearChildren?.(containerHandle);
}

// Re-render the root component
function rerender() {
    if (!rootComponent || !rootElement) return;
    
    const RNL = getRNL();
    const root = RNL.getRootHandle?.();
    if (!root) return;
    
    isRendering = true;
    pendingRerender = false;
    
    // Reset component IDs for consistent hook ordering
    currentComponentId = 0;
    
    // Remove old tree
    if (rootHandle) {
        RNL.removeChild?.(root, rootHandle);
    }
    
    // Render new tree
    rootHandle = renderElement(rootElement);
    
    // Append to root
    if (rootHandle) {
        RNL.appendChild?.(root, rootHandle);
    }
    
    isRendering = false;
    
    // Check if another rerender was requested during this render
    if (pendingRerender) {
        scheduleRerender();
    }
}

// Public render function - called once to mount the app
export function render(element, container) {
    const RNL = getRNL();
    
    // Store for re-renders
    rootElement = element;
    
    isRendering = true;
    currentComponentId = 0;
    
    // Initial render
    rootHandle = renderElement(element);
    
    // Append to root container
    if (rootHandle) {
        const root = RNL.getRootHandle?.();
        if (root) {
            RNL.appendChild?.(root, rootHandle);
        }
    }
    
    isRendering = false;
    
    return rootHandle;
}

// Fragment just returns its children
export const Fragment = ({ children }) => children;

export default { useState, useEffect, useCallback, useMemo, useRef, createElement, render, jsx, jsxs, Fragment };
"#.to_string()
}

fn generate_rnl_types() -> String {
    r#"// RNL Type Definitions

import { ReactNode, ReactElement } from 'react';

// Hooks
export function useState<T>(initialValue: T | (() => T)): [T, (value: T | ((prev: T) => T)) => void];
export function useEffect(effect: () => void | (() => void), deps?: any[]): void;
export function useCallback<T extends (...args: any[]) => any>(callback: T, deps: any[]): T;
export function useMemo<T>(factory: () => T, deps: any[]): T;
export function useRef<T>(initialValue: T): { current: T };

// JSX
export function createElement(type: any, props: any, ...children: any[]): any;
export function render(element: ReactElement, container?: any): any;

export const jsx: typeof createElement;
export const jsxs: typeof createElement;
export const Fragment: React.FC<{ children?: ReactNode }>;

// Native element props
interface BoxProps {
    orientation?: 'horizontal' | 'vertical';
    spacing?: number;
    style?: React.CSSProperties;
    children?: ReactNode;
}

interface TextProps {
    style?: React.CSSProperties;
    children?: ReactNode;
}

interface ButtonProps {
    label?: string;
    enabled?: boolean;
    onClick?: () => void;
    style?: React.CSSProperties;
}

interface InputProps {
    value?: string;
    placeholder?: string;
    onChange?: (value: string) => void;
    style?: React.CSSProperties;
}

// Declare JSX intrinsic elements
declare global {
    namespace JSX {
        interface IntrinsicElements {
            box: BoxProps;
            text: TextProps;
            button: ButtonProps;
            input: InputProps;
        }
    }
}

export {};
"#.to_string()
}

fn generate_rnl_jsx_runtime() -> String {
    r#"// RNL JSX Runtime (for react-jsx transform)
export { jsx, jsxs, Fragment } from './index.js';
"#.to_string()
}

fn generate_rnl_jsx_runtime_types() -> String {
    r#"// RNL JSX Runtime Types
export { jsx, jsxs, Fragment } from './index';
"#.to_string()
}
