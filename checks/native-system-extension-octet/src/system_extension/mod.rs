#![allow(
    tigerstyle::path_segment_repetition,
    reason = "the focused shim preserves exact Molten system-extension names for the real native-host core"
)]

const MAX_TEXT_CHARS: usize = 256;
const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_HEX_CHARS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecyclePhase {
    Absent,
    Installed,
    Admitted,
    Initializing,
    Initialized,
    Starting,
    Running,
    Checkpointing,
    Recovering,
    Draining,
    Drained,
    Failed,
    Restarting,
    Upgrading,
    RollingBack,
    ShuttingDown,
    Quarantined,
    Stopped,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    Unknown,
    Starting,
    Healthy,
    Degraded,
    Failed,
    Quarantined,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleState {
    pub generation: u64,
    pub phase: LifecyclePhase,
    pub restart_attempts: u64,
    pub health: HealthState,
    pub checkpoint_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceUsage {
    pub concurrent_callbacks: u64,
    pub queued_events: u64,
    pub inflight_bytes: u64,
    pub open_streams: u64,
    pub timers: u64,
    pub effect_requests: u64,
}

impl ResourceUsage {
    pub const fn is_idle(self) -> bool {
        self.concurrent_callbacks == 0
            && self.queued_events == 0
            && self.inflight_bytes == 0
            && self.open_streams == 0
            && self.timers == 0
            && self.effect_requests == 0
    }
}

pub(crate) fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TEXT_CHARS
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':'))
}

pub(crate) fn valid_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(BLAKE3_PREFIX) else {
        return false;
    };
    hex.len() == BLAKE3_HEX_CHARS
        && hex.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
}

pub mod native_host;
pub use native_host::*;
