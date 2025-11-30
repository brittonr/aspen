
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VmStats {
    pub cpu_usage: f64,
    pub ram_usage_mb: u64,
    pub disk_usage_mb: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_vm_stats_creation(
            cpu_usage in any::<f64>(),
            ram_usage_mb in any::<u64>(),
            disk_usage_mb in any::<u64>(),
        ) {
            let stats = VmStats {
                cpu_usage,
                ram_usage_mb,
                disk_usage_mb,
            };
            assert_eq!(stats.cpu_usage, cpu_usage);
            assert_eq!(stats.ram_usage_mb, ram_usage_mb);
            assert_eq!(stats.disk_usage_mb, disk_usage_mb);
        }
    }
}
