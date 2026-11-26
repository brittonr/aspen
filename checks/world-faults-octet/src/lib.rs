//! Focused strict-Octet compilation surface for world fault conformance.

#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod fabric {
    //! Production-identical mirror of the `molten-core` fabric helper the copied
    //! modules call (`crates/molten-core/src/fabric/mod.rs`).

    const BLAKE3_REF_PREFIX: &str = "blake3:";
    const BLAKE3_HEX_CHAR_COUNT: usize = 64;
    const BLAKE3_REF_CHAR_COUNT: usize = BLAKE3_REF_PREFIX.len() + BLAKE3_HEX_CHAR_COUNT;

    pub(crate) fn valid_blake3_ref(value: &str) -> bool {
        let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
            return false;
        };
        value.len() == BLAKE3_REF_CHAR_COUNT
            && hex.len() == BLAKE3_HEX_CHAR_COUNT
            && hex.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
    }
}

pub mod fabric_simulation {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum SchedulerChoiceKind {
        Runnable,
        MessageDelivery,
        TimerFire,
        StorageCompletion,
        ProcessLifecycle,
        FaultActivation,
    }

    impl SchedulerChoiceKind {
        pub const fn as_str(self) -> &'static str {
            match self {
                Self::Runnable => "runnable",
                Self::MessageDelivery => "message-delivery",
                Self::TimerFire => "timer-fire",
                Self::StorageCompletion => "storage-completion",
                Self::ProcessLifecycle => "process-lifecycle",
                Self::FaultActivation => "fault-activation",
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct EligibleChoice {
        pub kind: SchedulerChoiceKind,
        pub choice_id: String,
        pub node_id: String,
        pub generation: u64,
        pub ready_at_tick: u64,
    }
}

pub mod committed_stream;

pub mod world_faults;
