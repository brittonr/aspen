//! Pipeline configuration module.
//!
//! This module handles loading and validating CI pipeline configurations
//! written in Nickel. The configuration schema is defined in `schema/ci_schema.ncl`.

pub mod loader;
pub(crate) mod schema;
pub mod types;

// Re-export main loading functions
pub use loader::load_pipeline_config;
pub use loader::load_pipeline_config_str;
