//! Client protocol context.
//!
//! Re-exports `ClientProtocolContext` from `aspen-rpc-core` for use by handlers.
//! Handler-specific extensions (like SecretsService) are handled separately.

// Re-export ClientProtocolContext from aspen-rpc-core
pub use aspen_rpc_core::ClientProtocolContext;

#[cfg(any(test, feature = "testing"))]
pub mod test_support {
    //! Test utilities for creating mock `ClientProtocolContext` instances.

    pub use aspen_rpc_core::test_support::*;
}
