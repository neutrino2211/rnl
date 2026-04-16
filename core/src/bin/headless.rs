//! Headless RNL runner for debugging
//!
//! This runs a JS bundle without any GUI, logging all native bridge calls
//! to help debug issues with bundle execution.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

use parking_lot::Mutex;
use once_cell::sync::Lazy;

// We'll create our own minimal runtime here to avoid platform dependencies

use rquickjs::{
    context::EvalOptions,
    function::Func,
    prelude::*,
    Ctx, Function, Object, Runtime,
};

/// Track created nodes for the UI tree
struct HeadlessNode {
    node_type: String,
    attributes: HashMap<String, String>,
    children: Vec<i64>,
    text: Option<String>,
}

static NODES: Lazy<Mutex<HashMap<i64, HeadlessNode>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_ID: AtomicI64 = AtomicI64::new(1);
static ROOT_HANDLE: AtomicI64 = AtomicI64::new(0);

fn create_node(node_type: String) -> i64 {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    println!("[BRIDGE] createNode({:?}) -> {}", node_type, id);
    
    NODES.lock().insert(id, HeadlessNode {
        node_type,
        attributes: HashMap::new(),
        children: Vec::new(),
        text: None,
    });
    
    id
}

fn create_text(text: String) -> i64 {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    println!("[BRIDGE] createText({:?}) -> {}", text, id);
    
    NODES.lock().insert(id, HeadlessNode {
        node_type: "__text__".to_string(),
        attributes: HashMap::new(),
        children: Vec::new(),
        text: Some(text),
    });
    
    id
}

fn debug_value(val: &rquickjs::Value) -> String {
    match val.type_of() {
        rquickjs::Type::String => format!("String({:?})", val.as_string().map(|s| s.to_string().unwrap_or_default())),
        rquickjs::Type::Int => format!("Int({})", val.as_int().unwrap_or(0)),
        rquickjs::Type::Object => "Object".to_string(),
        rquickjs::Type::Array => "Array".to_string(),
        rquickjs::Type::Function => "Function".to_string(),
        rquickjs::Type::Null => "null".to_string(),
        rquickjs::Type::Undefined => "undefined".to_string(),
        rquickjs::Type::Bool => format!("Bool({})", val.as_bool().unwrap_or(false)),
        _ => format!("{:?}", val.type_of()),
    }
}

fn set_attribute(handle: i64, name: String, value: String) {
    println!("[BRIDGE] setAttribute({}, {:?}, {:?})", handle, name, value);
    
    if let Some(node) = NODES.lock().get_mut(&handle) {
        node.attributes.insert(name, value);
    }
}

fn set_callback(handle: i64, name: String, _callback: Function) {
    println!("[BRIDGE] setCallback({}, {:?}, <function>)", handle, name);
}

fn append_child(parent: i64, child: i64) {
    println!("[BRIDGE] appendChild({}, {})", parent, child);
    
    if let Some(node) = NODES.lock().get_mut(&parent) {
        node.children.push(child);
    }
}

fn insert_before(parent: i64, child: i64, before: i64) {
    println!("[BRIDGE] insertBefore({}, {}, {})", parent, child, before);
    
    if let Some(node) = NODES.lock().get_mut(&parent) {
        if let Some(pos) = node.children.iter().position(|&c| c == before) {
            node.children.insert(pos, child);
        } else {
            node.children.push(child);
        }
    }
}

fn remove_child(parent: i64, child: i64) {
    println!("[BRIDGE] removeChild({}, {})", parent, child);
    
    if let Some(node) = NODES.lock().get_mut(&parent) {
        node.children.retain(|&c| c != child);
    }
}

fn set_text(handle: i64, text: String) {
    println!("[BRIDGE] setText({}, {:?})", handle, text);
    
    if let Some(node) = NODES.lock().get_mut(&handle) {
        node.text = Some(text);
    }
}

fn get_root_handle() -> i64 {
    let handle = ROOT_HANDLE.load(Ordering::SeqCst);
    println!("[BRIDGE] getRootHandle() -> {}", handle);
    handle
}

/// Print the UI tree recursively
fn print_tree(handle: i64, indent: usize) {
    let nodes = NODES.lock();
    if let Some(node) = nodes.get(&handle) {
        let prefix = "  ".repeat(indent);
        
        if node.node_type == "__text__" {
            println!("{}TEXT: {:?}", prefix, node.text.as_deref().unwrap_or(""));
        } else if node.node_type == "__root__" {
            println!("{}ROOT (handle={})", prefix, handle);
        } else {
            let attrs: Vec<String> = node.attributes
                .iter()
                .map(|(k, v)| format!("{}={:?}", k, v))
                .collect();
            let attrs_str = if attrs.is_empty() { 
                String::new() 
            } else { 
                format!(" [{}]", attrs.join(", ")) 
            };
            println!("{}<{}>{} (handle={})", prefix, node.node_type, attrs_str, handle);
        }
        
        // Print children
        let children = node.children.clone();
        drop(nodes); // Release lock before recursing
        
        for child_id in children {
            print_tree(child_id, indent + 1);
        }
    }
}

fn setup_rnl_module(ctx: &Ctx) -> Result<(), rquickjs::Error> {
    let globals = ctx.globals();
    let module = Object::new(ctx.clone())?;

    module.set("createNode", Func::from(|node_type: String| -> i64 {
        create_node(node_type)
    }))?;

    module.set("createText", Func::from(|text: String| -> i64 {
        create_text(text)
    }))?;

    module.set("setAttribute", Func::from(|handle: i64, name: String, value: String| {
        set_attribute(handle, name, value);
    }))?;

    module.set("setCallback", Func::from(|handle: i64, name: String, callback: Function| {
        set_callback(handle, name, callback);
    }))?;

    module.set("appendChild", Func::from(|parent: i64, child: i64| {
        append_child(parent, child);
    }))?;

    module.set("insertBefore", Func::from(|parent: i64, child: i64, before: i64| {
        insert_before(parent, child, before);
    }))?;

    module.set("removeChild", Func::from(|parent: i64, child: i64| {
        remove_child(parent, child);
    }))?;

    module.set("setText", Func::from(|handle: i64, text: String| {
        set_text(handle, text);
    }))?;

    module.set("getRootHandle", Func::from(|| -> i64 {
        get_root_handle()
    }))?;

    globals.set("RNLNativeModule", module)?;
    
    println!("[HEADLESS] RNLNativeModule registered on globalThis");
    
    Ok(())
}

fn setup_console(ctx: &Ctx) -> Result<(), rquickjs::Error> {
    let globals = ctx.globals();
    let console = Object::new(ctx.clone())?;

    console.set("log", Func::from(|args: Rest<rquickjs::Value>| {
        let msg: Vec<String> = args.0.iter().map(|v| format!("{:?}", v)).collect();
        println!("[JS LOG] {}", msg.join(" "));
    }))?;

    console.set("warn", Func::from(|args: Rest<rquickjs::Value>| {
        let msg: Vec<String> = args.0.iter().map(|v| format!("{:?}", v)).collect();
        println!("[JS WARN] {}", msg.join(" "));
    }))?;

    console.set("error", Func::from(|args: Rest<rquickjs::Value>| {
        let msg: Vec<String> = args.0.iter().map(|v| format!("{:?}", v)).collect();
        println!("[JS ERROR] {}", msg.join(" "));
    }))?;

    globals.set("console", console)?;
    Ok(())
}

fn main() {
    println!("=== RNL Headless Runner ===\n");

    // Get bundle path from args
    let args: Vec<String> = std::env::args().collect();
    let bundle_path = args.get(1).map(|s| s.as_str()).unwrap_or("target/bundle.js");

    println!("[HEADLESS] Loading bundle from: {}", bundle_path);

    // Load bundle
    let bundle_content = match std::fs::read_to_string(bundle_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("[HEADLESS] Failed to load bundle: {}", e);
            std::process::exit(1);
        }
    };

    println!("[HEADLESS] Bundle loaded ({} bytes)\n", bundle_content.len());

    // Create a fake root node
    let root_id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    ROOT_HANDLE.store(root_id, Ordering::SeqCst);
    NODES.lock().insert(root_id, HeadlessNode {
        node_type: "__root__".to_string(),
        attributes: HashMap::new(),
        children: Vec::new(),
        text: None,
    });
    println!("[HEADLESS] Created root container (handle={})\n", root_id);

    // Create QuickJS runtime
    let runtime = Runtime::new().expect("Failed to create runtime");
    runtime.set_memory_limit(64 * 1024 * 1024);
    
    let context = rquickjs::Context::full(&runtime).expect("Failed to create context");

    // Setup globals and run bundle
    let result = context.with(|ctx| {
        setup_console(&ctx)?;
        setup_rnl_module(&ctx)?;

        println!("\n[HEADLESS] === Executing Bundle ===\n");

        let mut options = EvalOptions::default();
        options.strict = false;
        options.backtrace_barrier = true;

        ctx.eval_with_options::<(), _>(bundle_content.as_bytes().to_vec(), options)
    });

    match result {
        Ok(_) => {
            println!("\n[HEADLESS] === Bundle executed successfully ===\n");
        }
        Err(e) => {
            eprintln!("\n[HEADLESS] === Bundle execution FAILED ===");
            eprintln!("[HEADLESS] Error: {:?}\n", e);
        }
    }

    // Print the resulting UI tree
    println!("\n[HEADLESS] === UI Tree ===\n");
    let root = ROOT_HANDLE.load(Ordering::SeqCst);
    print_tree(root, 0);
    
    println!("\n[HEADLESS] === Done ===");
}
