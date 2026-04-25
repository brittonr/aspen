//! Client protocol context boundary.
//!
//! The default reusable RPC surface intentionally exposes only a minimal context
//! token so handler traits and dispatch helpers can compile without the Aspen
//! node service graph. The concrete Aspen runtime context is available behind
//! the `runtime-context` feature.

#[cfg(feature = "runtime-context")]
#[path = "context_runtime.rs"]
mod runtime;

#[cfg(feature = "runtime-context")]
pub use runtime::*;

/// Minimal context token for reusable handler/dispatch tests.
///
/// This type carries no Aspen services. Runtime code enables
/// `runtime-context`, which replaces this token with the full Aspen
/// `ClientProtocolContext` from `context_runtime.rs`.
#[cfg(not(feature = "runtime-context"))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClientProtocolContext {
    _private: (),
}

#[cfg(not(feature = "runtime-context"))]
impl ClientProtocolContext {
    /// Construct an empty reusable context.
    #[must_use]
    pub const fn empty() -> Self {
        Self { _private: () }
    }
}
