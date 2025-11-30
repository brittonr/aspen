
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::job_requirements::JobRequirements;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Job {
    pub id: Uuid,
    pub requirements: JobRequirements,
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

    proptest! {
        #[test]
        fn test_job_creation(
            id in arb_uuid(),
            requirements in arb_job_requirements(),
        ) {
            let job = Job {
                id: id.clone(),
                requirements: requirements.clone(),
            };
            assert_eq!(job.id, id);
            assert_eq!(job.requirements, requirements);
        }
    }
}
