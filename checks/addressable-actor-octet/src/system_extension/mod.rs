#[path = "generation/mod.rs"]
mod counters;

pub use counters::Generation;
pub use counters::Incarnation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthState {
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecyclePhase {
    Initializing,
    Starting,
    Running,
    Recovering,
    Draining,
    Drained,
    Failed,
    Restarting,
    Quarantined,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleState {
    pub generation: Generation,
    pub incarnation: Incarnation,
    pub phase: LifecyclePhase,
    pub restart_attempts: u64,
    pub health: HealthState,
    pub checkpoint_ref: Option<String>,
}
