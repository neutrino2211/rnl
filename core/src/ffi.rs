//! FFI types and functions exposed to C
//!
//! This module re-exports types that should be visible in the generated header.

pub use crate::registry::RnlElementFactory;

// Re-export C API functions (they're defined in lib.rs, registry.rs, and bridge.rs)
// These are marked with #[no_mangle] and extern "C" so they appear in the C ABI.
