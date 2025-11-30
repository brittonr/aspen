
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ExecutionHandle {
    pub job_id: Uuid,
    pub execution_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn arb_uuid()(
            bytes in any::<[u8; 16]>(),
        ) -> Uuid {
            Uuid::from_bytes(bytes)
        }
    }

    proptest! {
        #[test]
        fn test_execution_handle_creation(
            job_id in arb_uuid(),
            execution_id in arb_uuid(),
        ) {
            let handle = ExecutionHandle {
                job_id: job_id.clone(),
                execution_id: execution_id.clone(),
            };
            assert_eq!(handle.job_id, job_id);
            assert_eq!(handle.execution_id, execution_id);
        }
    }
}
