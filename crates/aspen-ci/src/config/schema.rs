//! Embedded CI schema for Nickel validation.

/// Returns the embedded CI schema as a string.
///
/// This schema is compiled into the binary and used to validate
/// pipeline configurations at runtime.
pub fn get_schema() -> &'static str {
    include_str!("schema/ci_schema.ncl")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_not_empty() {
        let schema = get_schema();
        assert!(!schema.is_empty());
        assert!(schema.contains("PipelineConfig"));
        assert!(schema.contains("StageConfig"));
        assert!(schema.contains("JobConfig"));
    }
}
