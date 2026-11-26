//! Closed initial crash-fault matrix; unsupported mechanisms cannot acquire live claims.

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Mechanism {
    NoFault,
    ProcessRestart {
        endpoint: alloc::string::String,
        down_ms: u64,
    },
    TemporaryPartition {
        left: alloc::vec::Vec<alloc::string::String>,
        right: alloc::vec::Vec<alloc::string::String>,
        duration_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub fault: Mechanism,
    pub heal_deadline_ms: u64,
    pub recovery_deadline_ms: u64,
    pub stable_membership_ms: u64,
    pub final_read_deadline_ms: u64,
}

pub const UNSUPPORTED: &[&str] = &[
    "clock",
    "durability",
    "byzantine",
    "queue",
    "lock",
    "transaction",
    "production-readiness",
    "release-eligibility",
    "universal-consensus-correctness",
];

/// Why a fault profile is outside the closed matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    InvalidEndpointCohort,
    InvalidFaultBound,
    UnstableRecoveryBound,
    UnknownFaultEndpoint,
    FaultDurationExceedsHealDeadline,
    InvalidPartitionSides,
    IncompletePartitionCohort,
}

impl Issue {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidEndpointCohort => "invalid-endpoint-cohort",
            Self::InvalidFaultBound => "invalid-fault-bound",
            Self::UnstableRecoveryBound => "unstable-recovery-bound",
            Self::UnknownFaultEndpoint => "unknown-fault-endpoint",
            Self::FaultDurationExceedsHealDeadline => "fault-duration-exceeds-heal-deadline",
            Self::InvalidPartitionSides => "invalid-partition-sides",
            Self::IncompletePartitionCohort => "incomplete-partition-cohort",
        }
    }
}

impl core::fmt::Display for Issue {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl core::error::Error for Issue {}

// r[impl molten.consensus.live_fault_profile]
pub fn admit(profile: &Profile, endpoints: &alloc::collections::BTreeSet<alloc::string::String>) -> Result<(), Issue> {
    if endpoints.len() < 3 || endpoints.len() > 9 {
        return Err(Issue::InvalidEndpointCohort);
    }
    for bound in [
        profile.heal_deadline_ms,
        profile.recovery_deadline_ms,
        profile.stable_membership_ms,
        profile.final_read_deadline_ms,
    ] {
        if !(1..=120_000).contains(&bound) {
            return Err(Issue::InvalidFaultBound);
        }
    }
    if profile.stable_membership_ms > profile.recovery_deadline_ms {
        return Err(Issue::UnstableRecoveryBound);
    }
    match &profile.fault {
        Mechanism::NoFault => Ok(()),
        Mechanism::ProcessRestart { endpoint, down_ms } => {
            if !endpoints.contains(endpoint) {
                return Err(Issue::UnknownFaultEndpoint);
            }
            admit_disruption(*down_ms, profile.heal_deadline_ms)
        }
        Mechanism::TemporaryPartition {
            left,
            right,
            duration_ms,
        } => {
            admit_disruption(*duration_ms, profile.heal_deadline_ms)?;
            let left_set: alloc::collections::BTreeSet<_> = left.iter().cloned().collect();
            let right_set: alloc::collections::BTreeSet<_> = right.iter().cloned().collect();
            if left_set.is_empty()
                || right_set.is_empty()
                || left_set.len() != left.len()
                || right_set.len() != right.len()
                || !left_set.is_disjoint(&right_set)
            {
                return Err(Issue::InvalidPartitionSides);
            }
            if left_set.union(&right_set).cloned().collect::<alloc::collections::BTreeSet<_>>() != *endpoints {
                return Err(Issue::IncompletePartitionCohort);
            }
            Ok(())
        }
    }
}

fn admit_disruption(duration_ms: u64, heal_deadline_ms: u64) -> Result<(), Issue> {
    if duration_ms == 0 {
        return Err(Issue::InvalidFaultBound);
    }
    if duration_ms > heal_deadline_ms {
        return Err(Issue::FaultDurationExceedsHealDeadline);
    }
    Ok(())
}
