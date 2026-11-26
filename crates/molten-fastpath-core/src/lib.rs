#![cfg_attr(not(test), no_std)]

//! Pure bounded design model of Aspen's blocked consensus fast-path acceleration.
//!
//! The crate owns the model profile admission and derived quorums, the conflict contract,
//! stable-view fast admission with its proposer-promise and proposer-acknowledgement guards,
//! the view-change recovery transitions, the session-keyed application ledger, the bounded
//! fault corpus, and the model-only evidence vocabulary. It has no filesystem, process,
//! network, clock, Tokio, Iroh, Redb, Wasmtime, or Preserves dependency. The `molten` shell
//! (`src/fabric_consistency/fastpath/`) re-exports it at the historical facade path and maps
//! each typed [`Rejection`] into a classified `Failure`.
// r[impl molten.consensus.no_std_raft_and_reliability_kernels]
// r[impl molten.project.tier1_no_std_kernel_crates]

extern crate alloc;

pub mod commute;
pub mod envelope;
pub mod hazard;
pub mod quorate;
pub mod succession;
pub mod witness;

#[cfg(test)]
mod tests;

/// The typed rejection of a fast-path model input, one variant per validator family.
///
/// The core never names a `molten` failure class; the shell maps every rejection to one, so the
/// typed reason survives until that outer boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    Conflict(crate::commute::ConflictContractIssue),
    Fault(crate::hazard::FaultScenarioIssue),
    Ledger(crate::quorate::LedgerIssue),
    Profile(crate::envelope::ProfileIssue),
    Recovery(crate::succession::RecoveryIssue),
}

impl core::fmt::Display for Rejection {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Conflict(issue) => write!(formatter, "conflict-contract:{issue}"),
            Self::Fault(issue) => write!(formatter, "fault-scenario:{issue}"),
            Self::Ledger(issue) => write!(formatter, "application-ledger:{issue}"),
            Self::Profile(issue) => write!(formatter, "model-profile:{issue}"),
            Self::Recovery(issue) => write!(formatter, "view-change-recovery:{issue}"),
        }
    }
}

impl core::error::Error for Rejection {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Conflict(issue) => Some(issue),
            Self::Fault(issue) => Some(issue),
            Self::Ledger(issue) => Some(issue),
            Self::Profile(issue) => Some(issue),
            Self::Recovery(issue) => Some(issue),
        }
    }
}

impl From<crate::commute::ConflictContractIssue> for Rejection {
    fn from(issue: crate::commute::ConflictContractIssue) -> Self {
        Self::Conflict(issue)
    }
}

impl From<crate::hazard::FaultScenarioIssue> for Rejection {
    fn from(issue: crate::hazard::FaultScenarioIssue) -> Self {
        Self::Fault(issue)
    }
}

impl From<crate::quorate::LedgerIssue> for Rejection {
    fn from(issue: crate::quorate::LedgerIssue) -> Self {
        Self::Ledger(issue)
    }
}

impl From<crate::envelope::ProfileIssue> for Rejection {
    fn from(issue: crate::envelope::ProfileIssue) -> Self {
        Self::Profile(issue)
    }
}

impl From<crate::succession::RecoveryIssue> for Rejection {
    fn from(issue: crate::succession::RecoveryIssue) -> Self {
        Self::Recovery(issue)
    }
}
