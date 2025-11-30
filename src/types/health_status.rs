
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Ok,
    Warning(String),
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn arb_health_status()(
            status in 0..3,
            warning in ".*",
            error in ".*",
        ) -> HealthStatus {
            match status {
                0 => HealthStatus::Ok,
                1 => HealthStatus::Warning(warning),
                _ => HealthStatus::Error(error),
            }
        }
    }

    proptest! {
        #[test]
        fn test_health_status_creation(
            status in arb_health_status(),
        ) {
            let _ = status;
        }
    }
}
