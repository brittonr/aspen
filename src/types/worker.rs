
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::resource_info::ResourceInfo;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Worker {
    pub id: Uuid,
    pub resources: ResourceInfo,
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
        fn arb_resource_info()(
            cpu_cores in any::<u32>(),
            ram_mb in any::<u64>(),
            disk_mb in any::<u64>(),
        ) -> ResourceInfo {
            ResourceInfo {
                cpu_cores,
                ram_mb,
                disk_mb,
            }
        }
    }

    proptest! {
        #[test]
        fn test_worker_creation(
            id in arb_uuid(),
            resources in arb_resource_info(),
        ) {
            let worker = Worker {
                id: id.clone(),
                resources: resources.clone(),
            };
            assert_eq!(worker.id, id);
            assert_eq!(worker.resources, resources);
        }
    }
}
