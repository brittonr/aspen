use super::*;

/// One admitted peer's observed availability of the required item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerAvailability {
    pub peer_id: String,
    /// Whether the peer holds the exact required item; `None` when no
    /// observation of the peer was recorded.
    pub has_item: Option<bool>,
}

/// A finite virtual progress horizon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualHorizon {
    /// Virtual ticks since the recovery window became stable.
    pub elapsed_ticks: u64,
    pub limit_ticks: u64,
}

/// The facts recovery progress is evaluated over, for one participant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryProgressInput {
    pub participant_id: String,
    pub required: EntryIdentity,
    pub required_index: u64,
    /// The participant's admitted local durable state already holds the item.
    pub is_locally_sufficient: bool,
    /// The participant holds the item after repair.
    pub is_repaired: bool,
    pub admitted_peers: Vec<PeerAvailability>,
    /// A disruptive fault is active, so the recovery window is not stable.
    pub is_disrupted: bool,
    /// The final drain of in-flight effects was observed.
    pub is_final_drain_observed: bool,
    /// A remote repair was requested.
    pub is_remote_repair_requested: bool,
    pub horizon: VirtualHorizon,
}

/// The verdict of one participant's recovery progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProgressVerdict {
    Pass,
    Fail,
    NotEvaluated,
    Incomplete,
}

/// The finding behind a verdict.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProgressFinding {
    LocallySufficient,
    /// Local state was sufficient, yet a remote repair was requested.
    UnnecessaryRemoteRepair,
    Repaired,
    /// An admitted peer held the item and the participant did not repair it
    /// within the horizon.
    MissedAvailableItem,
    /// Complete observations show every permitted source lacks the item.
    GloballyUnavailable,
    HorizonNotReached,
    UnstableWindow,
    MissingPeerObservation(String),
    MissingFinalDrain,
}

/// The participant-scoped recovery progress result, bound to its facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryProgress {
    pub participant_id: String,
    pub required: EntryIdentity,
    pub required_index: u64,
    pub admitted_peer_ids: Vec<String>,
    pub horizon: VirtualHorizon,
    pub verdict: ProgressVerdict,
    pub finding: ProgressFinding,
}

/// Evaluate one participant's recovery progress.
///
/// Missing observations yield incomplete, never pass, fail, or global absence.
/// Global absence needs a complete observation of every admitted peer lacking
/// the exact item.
// r[impl molten.consensus.storage_fault_recovery_progress]
pub fn evaluate_recovery_progress(input: &RecoveryProgressInput) -> RecoveryProgress {
    let (verdict, finding) = progress_outcome(input);
    RecoveryProgress {
        participant_id: input.participant_id.clone(),
        required: input.required.clone(),
        required_index: input.required_index,
        admitted_peer_ids: input.admitted_peers.iter().map(|peer| peer.peer_id.clone()).collect(),
        horizon: input.horizon,
        verdict,
        finding,
    }
}

fn progress_outcome(input: &RecoveryProgressInput) -> (ProgressVerdict, ProgressFinding) {
    if input.is_locally_sufficient {
        return if input.is_remote_repair_requested {
            (ProgressVerdict::Fail, ProgressFinding::UnnecessaryRemoteRepair)
        } else {
            (ProgressVerdict::Pass, ProgressFinding::LocallySufficient)
        };
    }
    if let Some(peer) = input.admitted_peers.iter().find(|peer| peer.has_item.is_none()) {
        return (ProgressVerdict::Incomplete, ProgressFinding::MissingPeerObservation(peer.peer_id.clone()));
    }
    if !input.is_final_drain_observed {
        return (ProgressVerdict::Incomplete, ProgressFinding::MissingFinalDrain);
    }
    if input.is_repaired {
        return (ProgressVerdict::Pass, ProgressFinding::Repaired);
    }
    if input.is_disrupted {
        return (ProgressVerdict::NotEvaluated, ProgressFinding::UnstableWindow);
    }
    let is_available = input.admitted_peers.iter().any(|peer| peer.has_item == Some(true));
    if !is_available {
        return (ProgressVerdict::Pass, ProgressFinding::GloballyUnavailable);
    }
    if input.horizon.elapsed_ticks >= input.horizon.limit_ticks {
        (ProgressVerdict::Fail, ProgressFinding::MissedAvailableItem)
    } else {
        (ProgressVerdict::NotEvaluated, ProgressFinding::HorizonNotReached)
    }
}
