
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::vm_config::VmConfig;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VmAssignment {
    pub vm_config: VmConfig,
    pub worker_id: Uuid,
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
        fn arb_vm_config()(
            image in ".*",
            cpu_cores in any::<u32>(),
            ram_mb in any::<u64>(),
            disk_mb in any::<u64>(),
        ) -> VmConfig {
            VmConfig {
                image,
                cpu_cores,
                ram_mb,
                disk_mb,
            }
        }
    }

    proptest! {
        #[test]
        fn test_vm_assignment_creation(
            vm_config in arb_vm_config(),
            worker_id in arb_uuid(),
        ) {
            let assignment = VmAssignment {
                vm_config: vm_config.clone(),
                worker_id: worker_id.clone(),
            };
            assert_eq!(assignment.vm_config, vm_config);
            assert_eq!(assignment.worker_id, worker_id);
        }
    }
}
