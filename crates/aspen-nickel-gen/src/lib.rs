//! Nickel contract generator for Rust configuration structs.
//!
//! This crate parses Rust source files containing configuration structs and generates
//! corresponding Nickel contracts with proper typing, defaults, and documentation.
//!
//! # Usage
//!
//! ```rust,ignore
//! use aspen_nickel_gen::{parse_config_file, generate_nickel_contracts};
//!
//! let metadata = parse_config_file("src/config.rs")?;
//! let nickel = generate_nickel_contracts(&metadata)?;
//! std::fs::write("schema/config.ncl", nickel)?;
//! ```

mod error;
mod generator;
mod parser;
mod types;

pub use error::GeneratorError;
pub use generator::generate_nickel_contracts;
pub use parser::parse_config_file;
pub use types::ConfigEnum;
pub use types::ConfigField;
pub use types::ConfigStruct;
pub use types::FieldType;
pub use types::ParsedConfig;
