
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum JobResult {
    Success(Vec<u8>),
    Failure(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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
        fn test_job_result_creation(
            result in arb_job_result(),
        ) {
            let _ = result;
        }
    }
}
