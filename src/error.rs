use thiserror::Error;

use crate::kv;

/// Crate-level error surface that wraps module-specific failures.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Kv(#[from] kv::error::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
