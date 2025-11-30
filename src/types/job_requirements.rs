
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JobRequirements {
    pub cpu_cores: u32,
    pub ram_mb: u64,
    pub disk_mb: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_job_requirements_creation(
            cpu_cores in any::<u32>(),
            ram_mb in any::<u64>(),
            disk_mb in any::<u64>(),
        ) {
            let reqs = JobRequirements {
                cpu_cores,
                ram_mb,
                disk_mb,
            };
            assert_eq!(reqs.cpu_cores, cpu_cores);
            assert_eq!(reqs.ram_mb, ram_mb);
            assert_eq!(reqs.disk_mb, disk_mb);
        }
    }
}
