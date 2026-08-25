use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryPhase {
    Normal,
    Paused,
    Agreed,
    MarkerCommitted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryState {
    pub acceleration_view: u64,
    pub base_view: u64,
    pub last_normal_view: u64,
    pub phase: RecoveryPhase,
    pub accepted_commands: BTreeSet<String>,
    pub recovery_set: BTreeSet<String>,
    pub marker_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    BeginViewChange { new_base_view: u64 },
    AgreeRecoverySet { commands: BTreeSet<String> },
    CommitRecoveryMarker { marker_ref: String },
    ResumeNormalView { new_acceleration_view: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryIssue {
    EmptyMarker,
    NewViewNotAhead,
    RecoverySetDropsAcceptedCommand,
    RecoverySetNotAgreed,
    RecoveryNotPaused,
    ResumeBeforeMarker,
    ViewMismatch,
}

// r[impl molten.consensus.fast_path_model.view_change_recovery]
pub fn transition_recovery(state: &RecoveryState, action: &RecoveryAction) -> Result<RecoveryState, RecoveryIssue> {
    match action {
        RecoveryAction::BeginViewChange { new_base_view } => begin_view_change(state, *new_base_view),
        RecoveryAction::AgreeRecoverySet { commands } => agree_recovery_set(state, commands),
        RecoveryAction::CommitRecoveryMarker { marker_ref } => commit_marker(state, marker_ref),
        RecoveryAction::ResumeNormalView { new_acceleration_view } => resume_normal(state, *new_acceleration_view),
    }
}

fn begin_view_change(state: &RecoveryState, new_base_view: u64) -> Result<RecoveryState, RecoveryIssue> {
    if new_base_view <= state.base_view {
        return Err(RecoveryIssue::NewViewNotAhead);
    }
    let mut next = state.clone();
    if state.phase == RecoveryPhase::Normal {
        next.last_normal_view = state.acceleration_view;
        next.recovery_set.clear();
        next.marker_ref = None;
    }
    next.base_view = new_base_view;
    next.phase = RecoveryPhase::Paused;
    Ok(next)
}

fn agree_recovery_set(state: &RecoveryState, commands: &BTreeSet<String>) -> Result<RecoveryState, RecoveryIssue> {
    if state.phase != RecoveryPhase::Paused {
        return Err(RecoveryIssue::RecoveryNotPaused);
    }
    if !state.accepted_commands.is_subset(commands) || !state.recovery_set.is_subset(commands) {
        return Err(RecoveryIssue::RecoverySetDropsAcceptedCommand);
    }
    let mut next = state.clone();
    next.recovery_set = commands.clone();
    next.phase = RecoveryPhase::Agreed;
    Ok(next)
}

fn commit_marker(state: &RecoveryState, marker_ref: &str) -> Result<RecoveryState, RecoveryIssue> {
    if state.phase != RecoveryPhase::Agreed {
        return Err(RecoveryIssue::RecoverySetNotAgreed);
    }
    if marker_ref.trim().is_empty() {
        return Err(RecoveryIssue::EmptyMarker);
    }
    let mut next = state.clone();
    next.marker_ref = Some(marker_ref.to_owned());
    next.phase = RecoveryPhase::MarkerCommitted;
    Ok(next)
}

fn resume_normal(state: &RecoveryState, new_acceleration_view: u64) -> Result<RecoveryState, RecoveryIssue> {
    if state.phase != RecoveryPhase::MarkerCommitted {
        return Err(RecoveryIssue::ResumeBeforeMarker);
    }
    if new_acceleration_view != state.base_view || new_acceleration_view <= state.last_normal_view {
        return Err(RecoveryIssue::ViewMismatch);
    }
    let mut next = state.clone();
    next.acceleration_view = new_acceleration_view;
    next.phase = RecoveryPhase::Normal;
    next.accepted_commands.clear();
    next.recovery_set.clear();
    next.marker_ref = None;
    Ok(next)
}
