
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::vm_state::VmState;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VmInstance {
    pub id: Uuid,
    pub state: VmState,
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
        fn arb_vm_state()(
            state in 0..4
        ) -> VmState {
            match state {
                0 => VmState::Pending,
                1 => VmState::Running,
                2 => VmState::Stopped,
                _ => VmState::Failed,
            }
        }
    }

    proptest! {
        #[test]
        fn test_vm_instance_creation(
            id in arb_uuid(),
            state in arb_vm_state(),
        ) {
            let instance = VmInstance {
                id: id.clone(),
                state: state.clone(),
            };
            assert_eq!(instance.id, id);
            assert_eq!(instance.state, state);
        }
    }
}
