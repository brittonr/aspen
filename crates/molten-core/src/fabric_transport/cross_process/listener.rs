use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossProcessListenerIdentity {
    pub listener_identity_ref: String,
    pub descriptor_ref: String,
    pub profile_id: String,
    pub protocol_id: String,
    pub alpn: String,
    pub extension_id: String,
    pub service_id: String,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossProcessListenerPhase {
    Planned,
    Starting,
    Ready,
    Draining,
    Closing,
    Cleaning,
    Closed,
    Replaced,
}

impl CrossProcessListenerPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Starting => "starting",
            Self::Ready => "ready",
            Self::Draining => "draining",
            Self::Closing => "closing",
            Self::Cleaning => "cleaning",
            Self::Closed => "closed",
            Self::Replaced => "replaced",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Closed | Self::Replaced)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerDrainReason {
    OperatorRequest,
    Cancellation,
    RegistrationRevoked,
    CapabilityRevoked,
    ProfileRevoked,
    AdapterFailure,
    Replacement,
}

impl ListenerDrainReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OperatorRequest => "operator-request",
            Self::Cancellation => "cancellation",
            Self::RegistrationRevoked => "registration-revoked",
            Self::CapabilityRevoked => "capability-revoked",
            Self::ProfileRevoked => "profile-revoked",
            Self::AdapterFailure => "adapter-failure",
            Self::Replacement => "replacement",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerTerminalClass {
    Clean,
    Cancelled,
    Revoked,
    Failed,
    Replaced,
}

impl ListenerTerminalClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Cancelled => "cancelled",
            Self::Revoked => "revoked",
            Self::Failed => "failed",
            Self::Replaced => "replaced",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossProcessListenerState {
    pub identity: CrossProcessListenerIdentity,
    pub phase: CrossProcessListenerPhase,
    pub max_sessions: u64,
    pub active_sessions: u64,
    pub accepted_sessions: u64,
    pub drain_reason: Option<ListenerDrainReason>,
    pub terminal_class: Option<ListenerTerminalClass>,
    pub cleanup_evidence_ref: Option<String>,
}

impl CrossProcessListenerState {
    pub const fn is_ready(&self) -> bool {
        matches!(self.phase, CrossProcessListenerPhase::Ready)
    }

    pub const fn is_terminal(&self) -> bool {
        self.phase.is_terminal()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListenerReadinessObservation {
    pub endpoint_setup: bool,
    pub exact_alpn_active: bool,
    pub registration_owned: bool,
    pub transport_capability_active: bool,
    pub protocol_capability_active: bool,
    pub profile_active: bool,
}

impl ListenerReadinessObservation {
    pub const fn fully_ready() -> Self {
        Self {
            endpoint_setup: true,
            exact_alpn_active: true,
            registration_owned: true,
            transport_capability_active: true,
            protocol_capability_active: true,
            profile_active: true,
        }
    }

    pub const fn is_ready(self) -> bool {
        self.endpoint_setup
            && self.exact_alpn_active
            && self.registration_owned
            && self.transport_capability_active
            && self.protocol_capability_active
            && self.profile_active
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossProcessListenerCommandKind {
    Start,
    MarkReady,
    AcceptSession,
    SessionTerminal,
    BeginDrain,
    Close,
    BeginCleanup,
    CompleteCleanup,
    Cancel,
    Fail,
    Replace,
}

impl CrossProcessListenerCommandKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::MarkReady => "mark-ready",
            Self::AcceptSession => "accept-session",
            Self::SessionTerminal => "session-terminal",
            Self::BeginDrain => "begin-drain",
            Self::Close => "close",
            Self::BeginCleanup => "begin-cleanup",
            Self::CompleteCleanup => "complete-cleanup",
            Self::Cancel => "cancel",
            Self::Fail => "fail",
            Self::Replace => "replace",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossProcessListenerCommand {
    Start,
    MarkReady(ListenerReadinessObservation),
    AcceptSession {
        callback_generation: u64,
    },
    SessionTerminal {
        callback_generation: u64,
    },
    BeginDrain {
        reason: ListenerDrainReason,
    },
    Close,
    BeginCleanup,
    CompleteCleanup {
        cleanup_evidence_ref: String,
    },
    Cancel,
    Fail,
    Replace {
        replacement_generation: u64,
        cleanup_evidence_ref: String,
    },
}

impl CrossProcessListenerCommand {
    pub const fn kind(&self) -> CrossProcessListenerCommandKind {
        match self {
            Self::Start => CrossProcessListenerCommandKind::Start,
            Self::MarkReady(_) => CrossProcessListenerCommandKind::MarkReady,
            Self::AcceptSession { .. } => CrossProcessListenerCommandKind::AcceptSession,
            Self::SessionTerminal { .. } => CrossProcessListenerCommandKind::SessionTerminal,
            Self::BeginDrain { .. } => CrossProcessListenerCommandKind::BeginDrain,
            Self::Close => CrossProcessListenerCommandKind::Close,
            Self::BeginCleanup => CrossProcessListenerCommandKind::BeginCleanup,
            Self::CompleteCleanup { .. } => CrossProcessListenerCommandKind::CompleteCleanup,
            Self::Cancel => CrossProcessListenerCommandKind::Cancel,
            Self::Fail => CrossProcessListenerCommandKind::Fail,
            Self::Replace { .. } => CrossProcessListenerCommandKind::Replace,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerShellAction {
    None,
    BindEndpoint,
    PublishDescriptor,
    AcceptSession,
    StopAccepting,
    CloseEndpoint,
    PersistCleanup,
    FenceReplacement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossProcessListenerTransition {
    pub next: CrossProcessListenerState,
    pub action: ListenerShellAction,
}

// r[impl molten.fabric_transport.cross_process_listener]
pub fn plan_cross_process_listener(
    profile: &TransportProfile,
    protocol: &ProtocolDescriptor,
    descriptor: &CrossProcessEndpointDescriptor,
    existing: &[CrossProcessListenerState],
) -> Result<CrossProcessListenerState, Vec<CrossProcessTransportIssue>> {
    let mut issues = validate_cross_process_endpoint(profile, protocol, descriptor).err().unwrap_or_default();
    let active_listener_count = existing.iter().filter(|listener| !listener.is_terminal()).count();
    let active_listener_count = match u64::try_from(active_listener_count) {
        Ok(count) => count,
        Err(_) => {
            issues.push(CrossProcessTransportIssue::CounterOverflow);
            0
        }
    };
    if active_listener_count >= profile.limits.max_listeners {
        issues.push(CrossProcessTransportIssue::ListenerLimitExceeded);
    }
    if existing.iter().any(|listener| {
        !listener.is_terminal()
            && listener.identity.alpn == descriptor.alpn
            && listener.identity.service_id == descriptor.service_id
            && listener.identity.generation == descriptor.generation
    }) {
        issues.push(CrossProcessTransportIssue::DuplicateListener);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(CrossProcessListenerState {
        identity: CrossProcessListenerIdentity {
            listener_identity_ref: descriptor.listener_identity_ref.clone(),
            descriptor_ref: descriptor.descriptor_ref.clone(),
            profile_id: descriptor.profile_id.clone(),
            protocol_id: descriptor.protocol_id.clone(),
            alpn: descriptor.alpn.clone(),
            extension_id: descriptor.extension_id.clone(),
            service_id: descriptor.service_id.clone(),
            generation: descriptor.generation,
        },
        phase: CrossProcessListenerPhase::Planned,
        max_sessions: descriptor.resources.max_sessions,
        active_sessions: 0,
        accepted_sessions: 0,
        drain_reason: None,
        terminal_class: None,
        cleanup_evidence_ref: None,
    })
}

// r[impl molten.fabric_transport.cross_process_listener]
pub fn apply_cross_process_listener_command(
    state: &CrossProcessListenerState,
    command: &CrossProcessListenerCommand,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    match command {
        CrossProcessListenerCommand::Start => transition_start(state, command.kind()),
        CrossProcessListenerCommand::MarkReady(readiness) => transition_ready(state, command.kind(), *readiness),
        CrossProcessListenerCommand::AcceptSession { callback_generation } => {
            transition_accept(state, command.kind(), *callback_generation)
        }
        CrossProcessListenerCommand::SessionTerminal { callback_generation } => {
            transition_session_terminal(state, command.kind(), *callback_generation)
        }
        CrossProcessListenerCommand::BeginDrain { reason } => transition_drain(state, command.kind(), *reason),
        CrossProcessListenerCommand::Close => transition_close(state, command.kind()),
        CrossProcessListenerCommand::BeginCleanup => transition_begin_cleanup(state, command.kind()),
        CrossProcessListenerCommand::CompleteCleanup { cleanup_evidence_ref } => {
            transition_complete_cleanup(state, command.kind(), cleanup_evidence_ref)
        }
        CrossProcessListenerCommand::Cancel => transition_forced_drain(
            state,
            command.kind(),
            ListenerDrainReason::Cancellation,
            ListenerTerminalClass::Cancelled,
        ),
        CrossProcessListenerCommand::Fail => transition_forced_drain(
            state,
            command.kind(),
            ListenerDrainReason::AdapterFailure,
            ListenerTerminalClass::Failed,
        ),
        CrossProcessListenerCommand::Replace {
            replacement_generation,
            cleanup_evidence_ref,
        } => transition_replace(state, command.kind(), *replacement_generation, cleanup_evidence_ref),
    }
}

fn transition_start(
    state: &CrossProcessListenerState,
    command: CrossProcessListenerCommandKind,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    require_listener_phase(state, command, &[CrossProcessListenerPhase::Planned])?;
    applied_listener(state, CrossProcessListenerPhase::Starting, ListenerShellAction::BindEndpoint)
}

fn transition_ready(
    state: &CrossProcessListenerState,
    command: CrossProcessListenerCommandKind,
    readiness: ListenerReadinessObservation,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    require_listener_phase(state, command, &[CrossProcessListenerPhase::Starting])?;
    if !readiness.is_ready() {
        return Err(vec![CrossProcessTransportIssue::ListenerReadinessIncomplete]);
    }
    applied_listener(state, CrossProcessListenerPhase::Ready, ListenerShellAction::PublishDescriptor)
}

fn transition_accept(
    state: &CrossProcessListenerState,
    command: CrossProcessListenerCommandKind,
    callback_generation: u64,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    require_listener_phase(state, command, &[CrossProcessListenerPhase::Ready])?;
    validate_listener_callback(state, callback_generation)?;
    if state.active_sessions >= state.max_sessions {
        return Err(vec![CrossProcessTransportIssue::ListenerSessionLimitExceeded]);
    }
    let mut next = state.clone();
    next.active_sessions = checked_increment(state.active_sessions).map_err(|issue| vec![issue])?;
    next.accepted_sessions = checked_increment(state.accepted_sessions).map_err(|issue| vec![issue])?;
    Ok(CrossProcessListenerTransition {
        next,
        action: ListenerShellAction::AcceptSession,
    })
}

fn transition_session_terminal(
    state: &CrossProcessListenerState,
    command: CrossProcessListenerCommandKind,
    callback_generation: u64,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    require_listener_phase(state, command, &[CrossProcessListenerPhase::Ready, CrossProcessListenerPhase::Draining])?;
    validate_listener_callback(state, callback_generation)?;
    if state.active_sessions == 0 {
        return Err(vec![CrossProcessTransportIssue::ListenerHasNoActiveSessions]);
    }
    let mut next = state.clone();
    next.active_sessions -= 1;
    Ok(CrossProcessListenerTransition {
        next,
        action: ListenerShellAction::None,
    })
}

fn transition_drain(
    state: &CrossProcessListenerState,
    command: CrossProcessListenerCommandKind,
    reason: ListenerDrainReason,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    require_listener_phase(state, command, &[CrossProcessListenerPhase::Ready])?;
    let mut next = state.clone();
    next.phase = CrossProcessListenerPhase::Draining;
    next.drain_reason = Some(reason);
    next.terminal_class = Some(match reason {
        ListenerDrainReason::Cancellation => ListenerTerminalClass::Cancelled,
        ListenerDrainReason::RegistrationRevoked
        | ListenerDrainReason::CapabilityRevoked
        | ListenerDrainReason::ProfileRevoked => ListenerTerminalClass::Revoked,
        ListenerDrainReason::AdapterFailure => ListenerTerminalClass::Failed,
        ListenerDrainReason::Replacement => ListenerTerminalClass::Replaced,
        ListenerDrainReason::OperatorRequest => ListenerTerminalClass::Clean,
    });
    Ok(CrossProcessListenerTransition {
        next,
        action: ListenerShellAction::StopAccepting,
    })
}

fn transition_forced_drain(
    state: &CrossProcessListenerState,
    command: CrossProcessListenerCommandKind,
    reason: ListenerDrainReason,
    terminal_class: ListenerTerminalClass,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    require_listener_phase(state, command, &[
        CrossProcessListenerPhase::Planned,
        CrossProcessListenerPhase::Starting,
        CrossProcessListenerPhase::Ready,
        CrossProcessListenerPhase::Draining,
    ])?;
    let mut next = state.clone();
    next.phase = if state.active_sessions == 0 {
        CrossProcessListenerPhase::Closing
    } else {
        CrossProcessListenerPhase::Draining
    };
    next.drain_reason = Some(reason);
    next.terminal_class = Some(terminal_class);
    Ok(CrossProcessListenerTransition {
        next,
        action: if state.active_sessions == 0 {
            ListenerShellAction::CloseEndpoint
        } else {
            ListenerShellAction::StopAccepting
        },
    })
}

fn transition_close(
    state: &CrossProcessListenerState,
    command: CrossProcessListenerCommandKind,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    require_listener_phase(state, command, &[
        CrossProcessListenerPhase::Planned,
        CrossProcessListenerPhase::Starting,
        CrossProcessListenerPhase::Ready,
        CrossProcessListenerPhase::Draining,
    ])?;
    if state.active_sessions != 0 {
        return Err(vec![CrossProcessTransportIssue::ListenerHasActiveSessions]);
    }
    let mut next = state.clone();
    next.phase = CrossProcessListenerPhase::Closing;
    if next.terminal_class.is_none() {
        next.terminal_class = Some(ListenerTerminalClass::Clean);
    }
    Ok(CrossProcessListenerTransition {
        next,
        action: ListenerShellAction::CloseEndpoint,
    })
}

fn transition_begin_cleanup(
    state: &CrossProcessListenerState,
    command: CrossProcessListenerCommandKind,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    require_listener_phase(state, command, &[CrossProcessListenerPhase::Closing])?;
    applied_listener(state, CrossProcessListenerPhase::Cleaning, ListenerShellAction::PersistCleanup)
}

fn transition_complete_cleanup(
    state: &CrossProcessListenerState,
    command: CrossProcessListenerCommandKind,
    cleanup_evidence_ref: &str,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    require_listener_phase(state, command, &[CrossProcessListenerPhase::Cleaning])?;
    let mut issues = Vec::new();
    validate_ref("listener-cleanup-evidence-ref", cleanup_evidence_ref, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut next = state.clone();
    next.phase = CrossProcessListenerPhase::Closed;
    next.cleanup_evidence_ref = Some(cleanup_evidence_ref.to_string());
    Ok(CrossProcessListenerTransition {
        next,
        action: ListenerShellAction::None,
    })
}

fn transition_replace(
    state: &CrossProcessListenerState,
    command: CrossProcessListenerCommandKind,
    replacement_generation: u64,
    cleanup_evidence_ref: &str,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    require_listener_phase(state, command, &[CrossProcessListenerPhase::Closed])?;
    let mut issues = Vec::new();
    validate_ref("replacement-cleanup-evidence-ref", cleanup_evidence_ref, &mut issues);
    if state.cleanup_evidence_ref.as_deref() != Some(cleanup_evidence_ref) {
        issues.push(CrossProcessTransportIssue::CleanupEvidenceRequired);
    }
    if replacement_generation <= state.identity.generation {
        issues.push(CrossProcessTransportIssue::GenerationDidNotAdvance);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut next = state.clone();
    next.phase = CrossProcessListenerPhase::Replaced;
    next.terminal_class = Some(ListenerTerminalClass::Replaced);
    Ok(CrossProcessListenerTransition {
        next,
        action: ListenerShellAction::FenceReplacement,
    })
}

fn require_listener_phase(
    state: &CrossProcessListenerState,
    command: CrossProcessListenerCommandKind,
    allowed: &[CrossProcessListenerPhase],
) -> Result<(), Vec<CrossProcessTransportIssue>> {
    if allowed.contains(&state.phase) {
        Ok(())
    } else {
        Err(vec![CrossProcessTransportIssue::InvalidListenerTransition {
            from: state.phase,
            command,
        }])
    }
}

fn validate_listener_callback(
    state: &CrossProcessListenerState,
    callback_generation: u64,
) -> Result<(), Vec<CrossProcessTransportIssue>> {
    if callback_generation == state.identity.generation {
        Ok(())
    } else {
        Err(vec![CrossProcessTransportIssue::StaleListenerCallback])
    }
}

fn applied_listener(
    state: &CrossProcessListenerState,
    phase: CrossProcessListenerPhase,
    action: ListenerShellAction,
) -> Result<CrossProcessListenerTransition, Vec<CrossProcessTransportIssue>> {
    let mut next = state.clone();
    next.phase = phase;
    Ok(CrossProcessListenerTransition { next, action })
}
