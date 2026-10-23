use super::*;

/// The stabilization facts of one participant over one observation interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantStabilization {
    pub participant: String,
    pub is_ready: bool,
    pub has_required_membership: bool,
    pub is_disruptive_fault_active: bool,
    pub is_durability_satisfied: bool,
    /// The participant's progress measure at the start of the interval.
    pub start_progress: u64,
    /// The measure at the end, or `None` when the end was not observed.
    pub end_progress: Option<u64>,
    /// Virtual ticks the interval covered.
    pub observed_ticks: u64,
}

/// Why a liveness property could not be evaluated or completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LivenessCondition {
    NotReady,
    MembershipUnavailable,
    DisruptiveFaultActive,
    DurabilityUnsatisfied,
    EndNotObserved,
    HorizonNotCovered,
}

impl LivenessCondition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotReady => "not-ready",
            Self::MembershipUnavailable => "membership-unavailable",
            Self::DisruptiveFaultActive => "disruptive-fault-active",
            Self::DurabilityUnsatisfied => "durability-unsatisfied",
            Self::EndNotObserved => "end-not-observed",
            Self::HorizonNotCovered => "horizon-not-covered",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessVerdict {
    Pass,
    Fail,
    NotEvaluated(LivenessCondition),
    Incomplete(LivenessCondition),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivenessResult {
    pub participant: String,
    pub verdict: LivenessVerdict,
}

/// Evaluate one participant's progress under explicit stabilization facts.
///
/// A missing precondition yields not-evaluated, never pass or fail. A missing
/// end observation or an interval shorter than the virtual-progress horizon
/// yields incomplete. Only an eligible participant observed for the full
/// horizon passes, when its measure advanced, or fails, when it did not.
// r[impl molten.fabric_simulation.participant_liveness]
pub fn evaluate_participant_liveness(
    preconditions: &LivenessPreconditions,
    facts: &ParticipantStabilization,
) -> LivenessResult {
    let missing = [
        (preconditions.requires_readiness && !facts.is_ready, LivenessCondition::NotReady),
        (
            preconditions.requires_membership && !facts.has_required_membership,
            LivenessCondition::MembershipUnavailable,
        ),
        (
            preconditions.disruptive_faults_block && facts.is_disruptive_fault_active,
            LivenessCondition::DisruptiveFaultActive,
        ),
        (
            preconditions.requires_durability && !facts.is_durability_satisfied,
            LivenessCondition::DurabilityUnsatisfied,
        ),
    ]
    .into_iter()
    .find_map(|(is_missing, condition)| is_missing.then_some(condition));
    let verdict = match (missing, facts.end_progress) {
        (Some(condition), _) => LivenessVerdict::NotEvaluated(condition),
        (None, None) => LivenessVerdict::Incomplete(LivenessCondition::EndNotObserved),
        (None, Some(_)) if facts.observed_ticks < preconditions.virtual_progress_horizon => {
            LivenessVerdict::Incomplete(LivenessCondition::HorizonNotCovered)
        }
        (None, Some(end)) if end > facts.start_progress => LivenessVerdict::Pass,
        (None, Some(_)) => LivenessVerdict::Fail,
    };
    LivenessResult {
        participant: facts.participant.clone(),
        verdict,
    }
}

/// Evaluate every participant separately: aggregate progress never hides a
/// stalled eligible participant.
// r[impl molten.fabric_simulation.participant_liveness]
pub fn evaluate_cohort_liveness(
    preconditions: &LivenessPreconditions,
    facts: &[ParticipantStabilization],
) -> Vec<LivenessResult> {
    facts.iter().map(|participant| evaluate_participant_liveness(preconditions, participant)).collect()
}
