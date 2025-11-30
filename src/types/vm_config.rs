
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VmConfig {
    pub image: String,
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
        fn test_vm_config_creation(
            image in ".*",
            cpu_cores in any::<u32>(),
            ram_mb in any::<u64>(),
            disk_mb in any::<u64>(),
        ) {
            let config = VmConfig {
                image: image.clone(),
                cpu_cores,
                ram_mb,
                disk_mb,
            };
            assert_eq!(config.image, image);
            assert_eq!(config.cpu_cores, cpu_cores);
            assert_eq!(config.ram_mb, ram_mb);
            assert_eq!(config.disk_mb, disk_mb);
        }
    }
}
