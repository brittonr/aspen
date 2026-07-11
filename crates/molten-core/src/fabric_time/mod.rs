//! Pure time, timer, scheduler, entropy, deadline, and lease contracts.
//!
//! Live clocks, sleeps, OS scheduling, and production entropy belong to the
//! outer runtime shell. This module accepts explicit values and returns bounded
//! deterministic transitions.

mod domain;
mod entropy;
mod lease;
mod profile;
mod scheduler;
mod timer;

#[cfg(test)]
mod tests;

pub use domain::*;
pub use entropy::*;
pub use lease::*;
pub use profile::*;
pub use scheduler::*;
pub use timer::*;

pub const FABRIC_TIME_PROFILE_SCHEMA: &str = "molten.fabric-time.profile.v1";
pub const FABRIC_TIME_OBSERVATION_SCHEMA: &str = "molten.fabric-time.observation.v1";
pub const FABRIC_TIMER_EVENT_SCHEMA: &str = "molten.fabric-time.timer-event.v1";
pub const FABRIC_SCHEDULER_EVENT_SCHEMA: &str = "molten.fabric-time.scheduler-event.v1";
pub const FABRIC_ENTROPY_EVENT_SCHEMA: &str = "molten.fabric-time.entropy-event.v1";
pub const FABRIC_DEADLINE_LEASE_SCHEMA: &str = "molten.fabric-time.deadline-lease.v1";
pub const FABRIC_TIME_RUN_REPORT_SCHEMA: &str = "molten.fabric-time.run-report.v1";

pub(crate) const MAX_TIME_COLLECTION_ITEMS: usize = 4_096;
pub(crate) const MAX_TIME_IDENTIFIER_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeNonClaim {
    NoGlobalTime,
    NoSynchronizedClocks,
    NoDistributedLeaseExclusivity,
    NoFairness,
    NoLiveness,
    NoSafeRetry,
    NoPartitionAbsence,
    NoRemoteDeadlineAgreement,
}

impl TimeNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoGlobalTime => "does-not-prove-global-time",
            Self::NoSynchronizedClocks => "does-not-prove-synchronized-clocks",
            Self::NoDistributedLeaseExclusivity => "does-not-prove-distributed-lease-exclusivity",
            Self::NoFairness => "does-not-prove-fairness",
            Self::NoLiveness => "does-not-prove-liveness",
            Self::NoSafeRetry => "does-not-prove-safe-retry",
            Self::NoPartitionAbsence => "does-not-prove-absence-of-partitions",
            Self::NoRemoteDeadlineAgreement => "does-not-prove-remote-deadline-agreement",
        }
    }
}

const REQUIRED_TIME_NON_CLAIM_COUNT: usize = 8;

pub const REQUIRED_TIME_NON_CLAIMS: [TimeNonClaim; REQUIRED_TIME_NON_CLAIM_COUNT] = [
    TimeNonClaim::NoGlobalTime,
    TimeNonClaim::NoSynchronizedClocks,
    TimeNonClaim::NoDistributedLeaseExclusivity,
    TimeNonClaim::NoFairness,
    TimeNonClaim::NoLiveness,
    TimeNonClaim::NoSafeRetry,
    TimeNonClaim::NoPartitionAbsence,
    TimeNonClaim::NoRemoteDeadlineAgreement,
];

pub(crate) fn valid_time_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TIME_IDENTIFIER_BYTES
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'-'))
}

pub(crate) fn valid_time_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("blake3:") else {
        return false;
    };
    const BLAKE3_HEX_BYTES: usize = 64;
    hex.len() == BLAKE3_HEX_BYTES && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn has_duplicates<T: Ord>(values: &[T]) -> bool {
    let mut ordered = std::collections::BTreeSet::new();
    values.iter().any(|value| !ordered.insert(value))
}
