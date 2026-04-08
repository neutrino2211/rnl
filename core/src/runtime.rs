//! JavaScript runtime using QuickJS
//!
//! This module sets up the QuickJS runtime and provides:
//! - JS evaluation
//! - console.log binding
//! - setTimeout/setInterval bindings
//! - Native module bridge

use rquickjs::{
    context::EvalOptions,
    function::Func,
    prelude::*,
    Ctx, Error as QjsError, Function, Object, Runtime, Value,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("JavaScript error: {0}")]
    JsError(String),

    #[error("Runtime initialization failed: {0}")]
    InitError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<QjsError> for RuntimeError {
    fn from(err: QjsError) -> Self {
        RuntimeError::JsError(format!("{:?}", err))
    }
}

/// JavaScript runtime wrapper
pub struct JsRuntime {
    runtime: Runtime,
    context: rquickjs::Context,
}

impl JsRuntime {
    /// Create a new JS runtime with RNL bindings
    pub fn new() -> Result<Self, RuntimeError> {
        let runtime = Runtime::new().map_err(|e| RuntimeError::InitError(format!("{:?}", e)))?;

        // Set memory limit (64MB default)
        runtime.set_memory_limit(64 * 1024 * 1024);

        let context =
            rquickjs::Context::full(&runtime).map_err(|e| RuntimeError::InitError(format!("{:?}", e)))?;

        // Set up global bindings
        context.with(|ctx| {
            setup_console(&ctx)?;
            setup_timers(&ctx)?;
            setup_rnl_module(&ctx)?;
            Ok::<_, RuntimeError>(())
        })?;

        Ok(Self { runtime, context })
    }

    /// Evaluate JavaScript code
    pub fn eval(&mut self, code: &str, filename: &str) -> Result<(), RuntimeError> {
        self.context.with(|ctx| {
            let mut options = EvalOptions::default();
            options.strict = false;
            options.backtrace_barrier = true;
            
            ctx.eval_with_options::<(), _>(code, options)
                .map_err(|e| RuntimeError::JsError(format!("{}: {:?}", filename, e)))
        })
    }

    /// Call a global function
    pub fn call_function(&mut self, name: &str, _args: &[&str]) -> Result<(), RuntimeError> {
        self.context.with(|ctx| {
            let globals = ctx.globals();
            if let Ok(func) = globals.get::<_, Function>(name) {
                // Call with no arguments for now - proper args handling would need more work
                func.call::<_, ()>(())
                    .map_err(|e| RuntimeError::JsError(format!("{}: {:?}", name, e)))?;
            }
            Ok(())
        })
    }

    /// Execute pending jobs (microtasks, timers)
    pub fn execute_pending_jobs(&mut self) {
        loop {
            match self.runtime.execute_pending_job() {
                Ok(false) => break, // No more jobs
                Ok(true) => continue, // More jobs pending
                Err(_) => break, // Error, stop
            }
        }
    }
}

/// Set up console object with log/warn/error/debug
fn setup_console(ctx: &Ctx) -> Result<(), RuntimeError> {
    let globals = ctx.globals();
    let console = Object::new(ctx.clone())?;

    // console.log
    console.set(
        "log",
        Func::from(|args: Rest<Value>| {
            let msg = format_console_args(args);
            log::info!("[js] {}", msg);
        }),
    )?;

    // console.warn
    console.set(
        "warn",
        Func::from(|args: Rest<Value>| {
            let msg = format_console_args(args);
            log::warn!("[js] {}", msg);
        }),
    )?;

    // console.error
    console.set(
        "error",
        Func::from(|args: Rest<Value>| {
            let msg = format_console_args(args);
            log::error!("[js] {}", msg);
        }),
    )?;

    // console.debug
    console.set(
        "debug",
        Func::from(|args: Rest<Value>| {
            let msg = format_console_args(args);
            log::debug!("[js] {}", msg);
        }),
    )?;

    globals.set("console", console)?;
    Ok(())
}

/// Format console arguments to a string
fn format_console_args(args: Rest<Value>) -> String {
    args.0
        .iter()
        .map(|v| match v.type_of() {
            rquickjs::Type::String => v.as_string().map(|s| s.to_string().unwrap_or_default()).unwrap_or_default(),
            rquickjs::Type::Int => v.as_int().map(|i| i.to_string()).unwrap_or_default(),
            rquickjs::Type::Float => v.as_float().map(|f| f.to_string()).unwrap_or_default(),
            rquickjs::Type::Bool => v.as_bool().map(|b| b.to_string()).unwrap_or_default(),
            rquickjs::Type::Null => "null".to_string(),
            rquickjs::Type::Undefined => "undefined".to_string(),
            _ => "[object]".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Set up setTimeout and setInterval (stub implementations)
fn setup_timers(ctx: &Ctx) -> Result<(), RuntimeError> {
    let globals = ctx.globals();

    // setTimeout - simplified version that doesn't actually delay
    // Full implementation would require integration with platform event loop
    globals.set(
        "setTimeout",
        Func::from(|callback: Function, _delay: i32| {
            // In a real implementation, this would schedule on the platform event loop
            // For now, execute immediately as a stub
            let _ = callback.call::<_, ()>(());
            0i32 // Return timer ID
        }),
    )?;

    // clearTimeout
    globals.set(
        "clearTimeout",
        Func::from(|_id: i32| {
            // Stub - would cancel the timer
        }),
    )?;

    // setInterval
    globals.set(
        "setInterval",
        Func::from(|_callback: Function, _delay: i32| {
            // Stub - would schedule recurring callback
            0i32
        }),
    )?;

    // clearInterval
    globals.set(
        "clearInterval",
        Func::from(|_id: i32| {
            // Stub - would cancel the interval
        }),
    )?;

    Ok(())
}

/// Set up the RNLNativeModule object for JS ↔ native communication
fn setup_rnl_module(ctx: &Ctx) -> Result<(), RuntimeError> {
    let globals = ctx.globals();
    let module = Object::new(ctx.clone())?;

    // These will be properly implemented in bridge.rs
    // For now, stubs that log what's happening

    // createNode(type: string) -> handle: number
    module.set(
        "createNode",
        Func::from(|node_type: String| -> i64 {
            log::debug!("createNode({})", node_type);
            crate::bridge::create_node(&node_type)
        }),
    )?;

    // createText(text: string) -> handle: number
    module.set(
        "createText",
        Func::from(|text: String| -> i64 {
            log::debug!("createText({})", text);
            crate::bridge::create_text(&text)
        }),
    )?;

    // setAttribute(handle: number, name: string, value: string)
    module.set(
        "setAttribute",
        Func::from(|handle: i64, name: String, value: String| {
            log::debug!("setAttribute({}, {}, {})", handle, name, value);
            crate::bridge::set_attribute(handle, &name, &value);
        }),
    )?;

    // setCallback(handle: number, name: string, callback: function)
    // Note: We can't easily pass Function across the bridge, so we'll skip this for now
    module.set(
        "setCallback",
        Func::from(|handle: i64, name: String| {
            log::debug!("setCallback({}, {}) - stub", handle, name);
        }),
    )?;

    // appendChild(parent: number, child: number)
    module.set(
        "appendChild",
        Func::from(|parent: i64, child: i64| {
            log::debug!("appendChild({}, {})", parent, child);
            crate::bridge::append_child(parent, child);
        }),
    )?;

    // insertBefore(parent: number, child: number, before: number)
    module.set(
        "insertBefore",
        Func::from(|parent: i64, child: i64, before: i64| {
            log::debug!("insertBefore({}, {}, {})", parent, child, before);
            crate::bridge::insert_before(parent, child, before);
        }),
    )?;

    // removeChild(parent: number, child: number)
    module.set(
        "removeChild",
        Func::from(|parent: i64, child: i64| {
            log::debug!("removeChild({}, {})", parent, child);
            crate::bridge::remove_child(parent, child);
        }),
    )?;

    // setText(handle: number, text: string)
    module.set(
        "setText",
        Func::from(|handle: i64, text: String| {
            log::debug!("setText({}, {})", handle, text);
            crate::bridge::set_text(handle, &text);
        }),
    )?;

    // getRootHandle() -> handle: number
    module.set(
        "getRootHandle",
        Func::from(|| -> i64 {
            log::debug!("getRootHandle()");
            crate::bridge::get_root_handle()
        }),
    )?;

    globals.set("RNLNativeModule", module)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_log() {
        let mut runtime = JsRuntime::new().unwrap();
        let result = runtime.eval(r#"console.log("Hello from JS!");"#, "<test>");
        assert!(result.is_ok());
    }

    #[test]
    fn test_arithmetic() {
        let mut runtime = JsRuntime::new().unwrap();
        let result = runtime.eval(
            r#"
            const x = 1 + 2;
            console.log("1 + 2 =", x);
        "#,
            "<test>",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_rnl_module_exists() {
        let mut runtime = JsRuntime::new().unwrap();
        let result = runtime.eval(
            r#"
            if (typeof RNLNativeModule === 'undefined') {
                throw new Error('RNLNativeModule not defined');
            }
            console.log('RNLNativeModule methods:', Object.keys(RNLNativeModule));
        "#,
            "<test>",
        );
        assert!(result.is_ok());
    }
}
