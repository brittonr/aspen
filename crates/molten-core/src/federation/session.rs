use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Planned,
    Fetching,
    Verified,
    Admitted,
    Committed,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub session_ref: String,
    pub generation: u64,
    pub plan: PullPlan,
    pub phase: SessionPhase,
    pub verified: Vec<String>,
    pub admitted: Vec<String>,
    pub committed: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    BeginFetch,
    Verified {
        head_ref: String,
        evidence_ref: String,
    },
    Admitted {
        head_ref: String,
        prerequisites: ImportPrerequisites,
    },
    Committed {
        head_ref: String,
        evidence_ref: String,
    },
    Denied {
        evidence_ref: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionIssue {
    Fence,
    Deadline,
    Terminal,
    Unplanned,
    Unverified,
    Unadmitted,
    Evidence,
    Retry,
    Bounds,
}

// r[impl molten.federated_pull_runtime.receiver_pull]
// r[impl molten.federated_pull_runtime.status_evidence]
pub fn advance_session(
    state: &Session,
    event: &SessionEvent,
    generation: u64,
    now: u64,
    bounds: &PullBounds,
) -> Result<Session, SessionIssue> {
    if generation != state.generation || generation != state.plan.generation {
        return Err(SessionIssue::Fence);
    }
    if now > state.plan.deadline && !matches!(event, SessionEvent::Denied { .. }) {
        return Err(SessionIssue::Deadline);
    }
    if matches!(state.phase, SessionPhase::Committed | SessionPhase::Denied) {
        return Err(SessionIssue::Terminal);
    }
    if state.plan.missing.len() > bounds.inventory_entries
        || state.verified.len() > bounds.inventory_entries
        || state.admitted.len() > bounds.inventory_entries
        || state.committed.len() > bounds.inventory_entries
        || state.evidence_refs.len() > bounds.retained_status
        || bounds.retained_status == 0
    {
        return Err(SessionIssue::Bounds);
    }
    let mut next = state.clone();
    apply_event(&mut next, event, bounds)?;
    if next.evidence_refs.len() > bounds.retained_status {
        return Err(SessionIssue::Bounds);
    }
    Ok(next)
}

fn apply_event(state: &mut Session, event: &SessionEvent, bounds: &PullBounds) -> Result<(), SessionIssue> {
    match event {
        SessionEvent::BeginFetch => {
            if state.attempts > bounds.retries {
                return Err(SessionIssue::Retry);
            }
            state.attempts = state.attempts.checked_add(1).ok_or(SessionIssue::Retry)?;
            state.phase = SessionPhase::Fetching;
        }
        SessionEvent::Verified { head_ref, evidence_ref } => {
            planned(state, head_ref)?;
            retain_evidence(state, evidence_ref)?;
            if !state.verified.contains(head_ref) {
                state.verified.push(head_ref.clone());
            }
            state.phase = SessionPhase::Verified;
        }
        SessionEvent::Admitted {
            head_ref,
            prerequisites,
        } => {
            planned(state, head_ref)?;
            if !state.verified.contains(head_ref) {
                return Err(SessionIssue::Unverified);
            }
            let refs = [
                &prerequisites.schema_ref,
                &prerequisites.policy_ref,
                &prerequisites.capability_ref,
                &prerequisites.provenance_ref,
                &prerequisites.resource_ref,
                &prerequisites.retention_ref,
                &prerequisites.import_ref,
            ];
            if state.evidence_refs.len().saturating_add(refs.len()) > bounds.retained_status {
                return Err(SessionIssue::Bounds);
            }
            for reference in refs {
                retain_evidence(state, reference)?;
            }
            if !state.admitted.contains(head_ref) {
                state.admitted.push(head_ref.clone());
            }
            state.phase = SessionPhase::Admitted;
        }
        SessionEvent::Committed { head_ref, evidence_ref } => {
            planned(state, head_ref)?;
            if !state.admitted.contains(head_ref) {
                return Err(SessionIssue::Unadmitted);
            }
            retain_evidence(state, evidence_ref)?;
            if !state.committed.contains(head_ref) {
                state.committed.push(head_ref.clone());
            }
            if state.committed.len() == state.plan.missing.len() {
                state.phase = SessionPhase::Committed;
            }
        }
        SessionEvent::Denied { evidence_ref } => {
            if !canonical_ref(evidence_ref) {
                return Err(SessionIssue::Evidence);
            }
            if state.evidence_refs.len() == bounds.retained_status && !state.evidence_refs.contains(evidence_ref) {
                state.evidence_refs.remove(0);
            }
            retain_evidence(state, evidence_ref)?;
            state.phase = SessionPhase::Denied;
        }
    }
    Ok(())
}

fn planned(state: &Session, head: &str) -> Result<(), SessionIssue> {
    if !state.plan.missing.iter().any(|entry| entry.head_ref == head) {
        return Err(SessionIssue::Unplanned);
    }
    Ok(())
}
fn retain_evidence(state: &mut Session, reference: &str) -> Result<(), SessionIssue> {
    if !canonical_ref(reference) {
        return Err(SessionIssue::Evidence);
    }
    if !state.evidence_refs.iter().any(|existing| existing == reference) {
        state.evidence_refs.push(reference.to_string());
    }
    Ok(())
}
