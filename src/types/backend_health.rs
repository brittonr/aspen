
use serde::{Deserialize, Serialize};

use super::health_status::HealthStatus;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BackendHealth {
    pub status: HealthStatus,
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
        fn test_backend_health_creation(
            status in arb_health_status(),
        ) {
            let health = BackendHealth {
                status: status.clone(),
            };
            assert_eq!(health.status, status);
        }
    }
}
