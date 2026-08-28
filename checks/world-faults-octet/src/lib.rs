//! Focused strict-Octet compilation surface for world fault conformance.

#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

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

pub mod world_faults;
