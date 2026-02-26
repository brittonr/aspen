//! Pipeline configuration module.
//!
//! This module handles loading and validating CI pipeline configurations.
//! When the `nickel` feature is enabled, configurations can be written in Nickel
//! with schema validation. The configuration types are always available via `types`.

#[cfg(feature = "nickel")]
pub mod loader;
pub(crate) mod schema;
pub mod types;

// Re-export main loading functions (only available with nickel feature)
#[cfg(feature = "nickel")]
pub use loader::load_pipeline_config;
#[cfg(feature = "nickel")]
pub use loader::load_pipeline_config_str;
#[cfg(feature = "nickel")]
pub use loader::load_pipeline_config_str_async;
