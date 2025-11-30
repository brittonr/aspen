
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn arb_execution_status()(
            status in 0..4
        ) -> ExecutionStatus {
            match status {
                0 => ExecutionStatus::Pending,
                1 => ExecutionStatus::Running,
                2 => ExecutionStatus::Completed,
                _ => ExecutionStatus::Failed,
            }
        }
    }

    proptest! {
        #[test]
        fn test_execution_status_creation(
            status in arb_execution_status(),
        ) {
            let _ = status;
        }
    }
}
