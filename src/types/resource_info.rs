
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ResourceInfo {
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
        fn test_resource_info_creation(
            cpu_cores in any::<u32>(),
            ram_mb in any::<u64>(),
            disk_mb in any::<u64>(),
        ) {
            let info = ResourceInfo {
                cpu_cores,
                ram_mb,
                disk_mb,
            };
            assert_eq!(info.cpu_cores, cpu_cores);
            assert_eq!(info.ram_mb, ram_mb);
            assert_eq!(info.disk_mb, disk_mb);
        }
    }
}
