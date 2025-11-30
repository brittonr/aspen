
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::job::Job;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JobAssignment {
    pub job: Job,
    pub worker_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use crate::types::job_requirements::JobRequirements;

    prop_compose! {
        fn arb_uuid()(
            bytes in any::<[u8; 16]>(),
        ) -> Uuid {
            Uuid::from_bytes(bytes)
        }
    }

    prop_compose! {
        fn arb_job_requirements()(
            cpu_cores in any::<u32>(),
            ram_mb in any::<u64>(),
            disk_mb in any::<u64>(),
        ) -> JobRequirements {
            JobRequirements {
                cpu_cores,
                ram_mb,
                disk_mb,
            }
        }
    }

    prop_compose! {
        fn arb_job()(
            id in arb_uuid(),
            requirements in arb_job_requirements(),
        ) -> Job {
            Job {
                id,
                requirements,
            }
        }
    }

    proptest! {
        #[test]
        fn test_job_assignment_creation(
            job in arb_job(),
            worker_id in arb_uuid(),
        ) {
            let assignment = JobAssignment {
                job: job.clone(),
                worker_id: worker_id.clone(),
            };
            assert_eq!(assignment.job, job);
            assert_eq!(assignment.worker_id, worker_id);
        }
    }
}
