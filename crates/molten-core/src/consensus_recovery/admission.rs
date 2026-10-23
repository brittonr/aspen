use super::*;

/// The facts a participant presents before it may rejoin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairAdmissionInput {
    /// Persistent state was repaired, corrupt, or uncertain.
    pub is_persistent_state_suspect: bool,
    /// The storage adapter reopened and reconciled, and serves.
    pub is_storage_serving: bool,
    /// Protocol reconciliation ran with no wait or conflict outstanding.
    pub is_protocol_reconciled: bool,
    /// The recovered log or snapshot passed validation.
    pub is_log_or_snapshot_validated: bool,
    /// The participant belongs to the current membership.
    pub is_membership_current: bool,
    pub hard_state: HardStateDecision,
    /// The current term the membership reports.
    pub current_term: u64,
}

/// A role a participant may be admitted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RejoinRole {
    /// May vote, lead, and serve durable operations.
    Voter,
    /// May replicate and catch up, but not vote, lead, or serve durable operations.
    NonVoting,
}

/// Why a participant may not vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdmissionBlocker {
    StorageNotServing,
    ProtocolNotReconciled,
    LogOrSnapshotNotValidated,
    MembershipNotCurrent,
    /// The vote record is lost and the current term is inside its fence.
    VoteFenced,
    HardStateUnresolved,
}

/// The admission decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairAdmission {
    pub role: Option<RejoinRole>,
    pub blockers: Vec<AdmissionBlocker>,
}

/// Admit a participant after a storage fault.
///
/// A participant with suspect persistent state may vote, lead, or serve
/// durable operations only after storage serving, protocol reconciliation,
/// log or snapshot validation, current membership, and a resolved term and
/// vote. A lost vote record admits only a non-voting role until the current
/// term is past its fence. Local repair alone never admits a voter.
// r[impl molten.consensus.repair_admission]
pub fn admit_repaired_participant(input: &RepairAdmissionInput) -> RepairAdmission {
    if !input.is_persistent_state_suspect
        && input.is_storage_serving
        && input.hard_state == HardStateDecision::KeepLocal
    {
        return RepairAdmission {
            role: Some(RejoinRole::Voter),
            blockers: Vec::new(),
        };
    }
    let mut blockers = Vec::with_capacity(6);
    let checks = [
        (input.is_storage_serving, AdmissionBlocker::StorageNotServing),
        (input.is_protocol_reconciled, AdmissionBlocker::ProtocolNotReconciled),
        (input.is_log_or_snapshot_validated, AdmissionBlocker::LogOrSnapshotNotValidated),
        (input.is_membership_current, AdmissionBlocker::MembershipNotCurrent),
    ];
    blockers.extend(checks.into_iter().filter(|(is_met, _)| !is_met).map(|(_, blocker)| blocker));
    let vote_blocker = match input.hard_state {
        HardStateDecision::KeepLocal => None,
        HardStateDecision::VoteFenced { through_term } if input.current_term > through_term => None,
        HardStateDecision::VoteFenced { .. } => Some(AdmissionBlocker::VoteFenced),
        HardStateDecision::Wait => Some(AdmissionBlocker::HardStateUnresolved),
    };
    let role = match (blockers.is_empty(), vote_blocker) {
        (true, None) => Some(RejoinRole::Voter),
        (true, Some(AdmissionBlocker::VoteFenced)) => Some(RejoinRole::NonVoting),
        _ => None,
    };
    blockers.extend(vote_blocker);
    RepairAdmission { role, blockers }
}
