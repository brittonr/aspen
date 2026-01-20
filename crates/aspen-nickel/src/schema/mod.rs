//! Embedded Nickel schema contracts for Aspen configuration.
//!
//! This module contains the Nickel contracts that define the structure and
//! validation rules for Aspen configuration. The contracts are embedded at
//! compile time and applied to user configuration during evaluation.

/// Get the embedded schema source.
///
/// Returns the full Nickel schema contract as a string literal.
/// This is embedded at compile time via `include_str!`.
pub fn get_schema() -> &'static str {
    include_str!("node_config.ncl")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_loads() {
        let schema = get_schema();
        assert!(!schema.is_empty());
        assert!(schema.contains("NodeConfig"));
    }
}
