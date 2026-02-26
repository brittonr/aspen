//! Nickel configuration language support for Aspen.
//!
//! This crate provides the integration between Nickel and Aspen's configuration system,
//! replacing TOML with a programmable configuration language that supports:
//!
//! - **Contracts**: Runtime type checking and validation
//! - **Merge semantics**: Layer base configs with environment-specific overrides
//! - **Functions**: Create reusable config fragments and transformations
//! - **Gradual typing**: Add types incrementally where they help
//!
//! # Example
//!
//! ```nickel
//! # cluster.ncl
//! let base = {
//!   cookie = "my-cluster",
//!   iroh.enable_raft_auth = true,
//! } in
//!
//! base & { node_id = 1, data_dir = "/var/lib/aspen/node-1" }
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use aspen_nickel::load_nickel_config;
//! use std::path::Path;
//!
//! let config = load_nickel_config(Path::new("cluster.ncl"))?;
//! ```

mod error;
mod loader;
mod schema;

pub use error::NickelConfigError;
pub use loader::load_nickel_config;
pub use loader::load_nickel_config_str;
