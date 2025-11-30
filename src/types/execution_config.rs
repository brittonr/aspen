
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ExecutionConfig {
    pub timeout_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_execution_config_creation(
            timeout_seconds in any::<u64>(),
        ) {
            let config = ExecutionConfig {
                timeout_seconds,
            };
            assert_eq!(config.timeout_seconds, timeout_seconds);
        }
    }
}
