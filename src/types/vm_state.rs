
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VmState {
    Pending,
    Running,
    Stopped,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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
        fn test_vm_state_creation(
            state in arb_vm_state(),
        ) {
            let _ = state;
        }
    }
}
