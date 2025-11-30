
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::job_result::JobResult;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorkResult {
    pub job_id: Uuid,
    pub result: JobResult,
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

    prop_compose! {
        fn arb_job_result()(
            success in any::<bool>(),
            data in any::<Vec<u8>>(),
            error in any::<String>(),
        ) -> JobResult {
            if success {
                JobResult::Success(data)
            } else {
                JobResult::Failure(error)
            }
        }
    }

    proptest! {
        #[test]
        fn test_work_result_creation(
            job_id in arb_uuid(),
            result in arb_job_result(),
        ) {
            let work_result = WorkResult {
                job_id: job_id.clone(),
                result: result.clone(),
            };
            assert_eq!(work_result.job_id, job_id);
            assert_eq!(work_result.result, result);
        }
    }
}
