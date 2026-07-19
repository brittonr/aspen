//! Production cryptographic identity adapter shells and canonical evidence.

mod artifact_auth;
mod canonical;
mod file_adapter;
mod integration;

pub use artifact_auth::*;
pub use canonical::*;
pub use file_adapter::*;
pub use integration::*;
pub use molten_core::fabric_crypto_identity::*;

#[cfg(test)]
mod tests;
