use super::*;

const NO_AUTOMATIC_RETRIES: u64 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossProcessSessionIdentity {
    pub session_ref: String,
    pub descriptor_ref: String,
    pub service_id: String,
    pub generation: u64,
    pub local_role: EndpointParticipantRole,
    pub expected_peer_context_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossProcessSessionPhase {
    Planned,
    Dialing,
    Active,
    Queued,
    AwaitingAcknowledgement,
    Draining,
    Closing,
    Cleaning,
    Closed,
}

impl CrossProcessSessionPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Dialing => "dialing",
            Self::Active => "active",
            Self::Queued => "queued",
            Self::AwaitingAcknowledgement => "awaiting-acknowledgement",
            Self::Draining => "draining",
            Self::Closing => "closing",
            Self::Cleaning => "cleaning",
            Self::Closed => "closed",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Closed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTerminalClass {
    Clean,
    Cancelled,
    LocalRefusal,
    RemoteRefusal,
    Disconnect,
    Reset,
    Timeout,
    MalformedInput,
    Overload,
    AdapterFailure,
}

impl SessionTerminalClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Cancelled => "cancelled",
            Self::LocalRefusal => "local-refusal",
            Self::RemoteRefusal => "remote-refusal",
            Self::Disconnect => "disconnect",
            Self::Reset => "reset",
            Self::Timeout => "timeout",
            Self::MalformedInput => "malformed-input",
            Self::Overload => "overload",
            Self::AdapterFailure => "adapter-failure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossProcessSessionState {
    pub identity: CrossProcessSessionIdentity,
    pub phase: CrossProcessSessionPhase,
    pub resources: EndpointResourceBounds,
    pub queued_bytes: u64,
    pub inflight_bytes: u64,
    pub frames_submitted: u64,
    pub frames_acknowledged: u64,
    pub frames_received: u64,
    pub delivery: DeliveryOutcome,
    pub retry: RetryDisposition,
    pub automatic_retry_count: u64,
    pub terminal_class: Option<SessionTerminalClass>,
    pub cleanup_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossProcessSessionCommandKind {
    BeginDial,
    Established,
    QueueFrame,
    FrameSubmitted,
    AcknowledgeFrame,
    ReceiveFrame,
    BeginDrain,
    Close,
    BeginCleanup,
    CompleteCleanup,
    Cancel,
    Fail,
}

impl CrossProcessSessionCommandKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BeginDial => "begin-dial",
            Self::Established => "established",
            Self::QueueFrame => "queue-frame",
            Self::FrameSubmitted => "frame-submitted",
            Self::AcknowledgeFrame => "acknowledge-frame",
            Self::ReceiveFrame => "receive-frame",
            Self::BeginDrain => "begin-drain",
            Self::Close => "close",
            Self::BeginCleanup => "begin-cleanup",
            Self::CompleteCleanup => "complete-cleanup",
            Self::Cancel => "cancel",
            Self::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossProcessSessionCommand {
    BeginDial {
        observed_descriptor_ref: String,
        callback_generation: u64,
    },
    Established {
        observed_peer_context_ref: String,
        callback_generation: u64,
    },
    QueueFrame {
        payload_bytes: u64,
        callback_generation: u64,
    },
    FrameSubmitted {
        payload_bytes: u64,
        callback_generation: u64,
    },
    AcknowledgeFrame {
        payload_bytes: u64,
        callback_generation: u64,
    },
    ReceiveFrame {
        payload_bytes: u64,
        callback_generation: u64,
    },
    BeginDrain,
    Close,
    BeginCleanup,
    CompleteCleanup {
        cleanup_evidence_ref: String,
    },
    Cancel,
    Fail {
        class: SessionTerminalClass,
        delivery_definitive: bool,
    },
}

impl CrossProcessSessionCommand {
    pub const fn kind(&self) -> CrossProcessSessionCommandKind {
        match self {
            Self::BeginDial { .. } => CrossProcessSessionCommandKind::BeginDial,
            Self::Established { .. } => CrossProcessSessionCommandKind::Established,
            Self::QueueFrame { .. } => CrossProcessSessionCommandKind::QueueFrame,
            Self::FrameSubmitted { .. } => CrossProcessSessionCommandKind::FrameSubmitted,
            Self::AcknowledgeFrame { .. } => CrossProcessSessionCommandKind::AcknowledgeFrame,
            Self::ReceiveFrame { .. } => CrossProcessSessionCommandKind::ReceiveFrame,
            Self::BeginDrain => CrossProcessSessionCommandKind::BeginDrain,
            Self::Close => CrossProcessSessionCommandKind::Close,
            Self::BeginCleanup => CrossProcessSessionCommandKind::BeginCleanup,
            Self::CompleteCleanup { .. } => CrossProcessSessionCommandKind::CompleteCleanup,
            Self::Cancel => CrossProcessSessionCommandKind::Cancel,
            Self::Fail { .. } => CrossProcessSessionCommandKind::Fail,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionShellAction {
    None,
    Dial,
    DeliverEstablished,
    QueueFrame,
    WriteFrame,
    DeliverAcknowledgement,
    DeliverFrame,
    StopNewFrames,
    CloseConnection,
    PersistCleanup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossProcessSessionTransition {
    pub next: CrossProcessSessionState,
    pub action: SessionShellAction,
}

// r[impl molten.fabric_transport.cross_process_session]
pub fn plan_cross_process_session(
    dial_plan: &EndpointDialPlan,
    session_ref: &str,
    local_role: EndpointParticipantRole,
) -> Result<CrossProcessSessionState, Vec<CrossProcessTransportIssue>> {
    let mut issues = Vec::new();
    validate_ref("cross-process-session-ref", session_ref, &mut issues);
    validate_ref("cross-process-descriptor-ref", &dial_plan.descriptor_ref, &mut issues);
    validate_ref("cross-process-peer-context-ref", &dial_plan.peer_context_ref, &mut issues);
    validate_token("cross-process-session-service", &dial_plan.service_id, &mut issues);
    if dial_plan.generation == 0 {
        issues.push(CrossProcessTransportIssue::GenerationMismatch);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(CrossProcessSessionState {
        identity: CrossProcessSessionIdentity {
            session_ref: session_ref.to_string(),
            descriptor_ref: dial_plan.descriptor_ref.clone(),
            service_id: dial_plan.service_id.clone(),
            generation: dial_plan.generation,
            local_role,
            expected_peer_context_ref: dial_plan.peer_context_ref.clone(),
        },
        phase: CrossProcessSessionPhase::Planned,
        resources: dial_plan.resources.clone(),
        queued_bytes: 0,
        inflight_bytes: 0,
        frames_submitted: 0,
        frames_acknowledged: 0,
        frames_received: 0,
        delivery: DeliveryOutcome::NotAttempted,
        retry: RetryDisposition::NotApplicable,
        automatic_retry_count: NO_AUTOMATIC_RETRIES,
        terminal_class: None,
        cleanup_evidence_ref: None,
    })
}

// r[impl molten.fabric_transport.cross_process_session]
pub fn apply_cross_process_session_command(
    state: &CrossProcessSessionState,
    command: &CrossProcessSessionCommand,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    if state.phase.is_terminal() {
        return Err(vec![CrossProcessTransportIssue::StaleSessionCallback]);
    }
    match command {
        CrossProcessSessionCommand::BeginDial {
            observed_descriptor_ref,
            callback_generation,
        } => transition_begin_dial(state, command.kind(), observed_descriptor_ref, *callback_generation),
        CrossProcessSessionCommand::Established {
            observed_peer_context_ref,
            callback_generation,
        } => transition_established(state, command.kind(), observed_peer_context_ref, *callback_generation),
        CrossProcessSessionCommand::QueueFrame {
            payload_bytes,
            callback_generation,
        } => transition_queue_frame(state, command.kind(), *payload_bytes, *callback_generation),
        CrossProcessSessionCommand::FrameSubmitted {
            payload_bytes,
            callback_generation,
        } => transition_frame_submitted(state, command.kind(), *payload_bytes, *callback_generation),
        CrossProcessSessionCommand::AcknowledgeFrame {
            payload_bytes,
            callback_generation,
        } => transition_acknowledged(state, command.kind(), *payload_bytes, *callback_generation),
        CrossProcessSessionCommand::ReceiveFrame {
            payload_bytes,
            callback_generation,
        } => transition_receive_frame(state, command.kind(), *payload_bytes, *callback_generation),
        CrossProcessSessionCommand::BeginDrain => transition_begin_drain(state, command.kind()),
        CrossProcessSessionCommand::Close => transition_close(state, command.kind()),
        CrossProcessSessionCommand::BeginCleanup => transition_begin_cleanup(state, command.kind()),
        CrossProcessSessionCommand::CompleteCleanup { cleanup_evidence_ref } => {
            transition_complete_cleanup(state, command.kind(), cleanup_evidence_ref)
        }
        CrossProcessSessionCommand::Cancel => transition_cancel(state, command.kind()),
        CrossProcessSessionCommand::Fail {
            class,
            delivery_definitive,
        } => transition_fail(state, command.kind(), *class, *delivery_definitive),
    }
}

fn transition_begin_dial(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
    observed_descriptor_ref: &str,
    callback_generation: u64,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[CrossProcessSessionPhase::Planned])?;
    validate_session_callback(state, callback_generation)?;
    if observed_descriptor_ref != state.identity.descriptor_ref {
        return Err(vec![CrossProcessTransportIssue::EndpointIdentityMismatch]);
    }
    applied_session(state, CrossProcessSessionPhase::Dialing, SessionShellAction::Dial)
}

fn transition_established(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
    observed_peer_context_ref: &str,
    callback_generation: u64,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[CrossProcessSessionPhase::Dialing])?;
    validate_session_callback(state, callback_generation)?;
    if observed_peer_context_ref != state.identity.expected_peer_context_ref {
        return Err(vec![CrossProcessTransportIssue::PeerContextMismatch]);
    }
    applied_session(state, CrossProcessSessionPhase::Active, SessionShellAction::DeliverEstablished)
}

fn transition_queue_frame(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
    payload_bytes: u64,
    callback_generation: u64,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[CrossProcessSessionPhase::Active])?;
    validate_session_callback(state, callback_generation)?;
    validate_frame_size(state, payload_bytes)?;
    let next_queued = checked_add(state.queued_bytes, payload_bytes).map_err(|issue| vec![issue])?;
    if next_queued > state.resources.max_queued_bytes {
        return Err(vec![CrossProcessTransportIssue::SessionQueueLimitExceeded]);
    }
    let mut next = state.clone();
    next.phase = CrossProcessSessionPhase::Queued;
    next.queued_bytes = next_queued;
    Ok(CrossProcessSessionTransition {
        next,
        action: SessionShellAction::QueueFrame,
    })
}

fn transition_frame_submitted(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
    payload_bytes: u64,
    callback_generation: u64,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[CrossProcessSessionPhase::Queued])?;
    validate_session_callback(state, callback_generation)?;
    if payload_bytes == 0 || payload_bytes != state.queued_bytes {
        return Err(vec![CrossProcessTransportIssue::SessionAccountingMismatch]);
    }
    let next_inflight = checked_add(state.inflight_bytes, payload_bytes).map_err(|issue| vec![issue])?;
    if next_inflight > state.resources.max_inflight_bytes {
        return Err(vec![CrossProcessTransportIssue::SessionInflightLimitExceeded]);
    }
    let mut next = state.clone();
    next.phase = CrossProcessSessionPhase::AwaitingAcknowledgement;
    next.queued_bytes = 0;
    next.inflight_bytes = next_inflight;
    next.frames_submitted = checked_increment(state.frames_submitted).map_err(|issue| vec![issue])?;
    next.delivery = DeliveryOutcome::Pending;
    next.retry = RetryDisposition::UnsafeWithoutReconciliation;
    Ok(CrossProcessSessionTransition {
        next,
        action: SessionShellAction::WriteFrame,
    })
}

fn transition_acknowledged(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
    payload_bytes: u64,
    callback_generation: u64,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[CrossProcessSessionPhase::AwaitingAcknowledgement])?;
    validate_session_callback(state, callback_generation)?;
    if payload_bytes == 0 || payload_bytes != state.inflight_bytes {
        return Err(vec![CrossProcessTransportIssue::SessionAccountingMismatch]);
    }
    let mut next = state.clone();
    next.phase = CrossProcessSessionPhase::Active;
    next.inflight_bytes = 0;
    next.frames_acknowledged = checked_increment(state.frames_acknowledged).map_err(|issue| vec![issue])?;
    next.delivery = DeliveryOutcome::Delivered;
    next.retry = RetryDisposition::NotApplicable;
    Ok(CrossProcessSessionTransition {
        next,
        action: SessionShellAction::DeliverAcknowledgement,
    })
}

fn transition_receive_frame(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
    payload_bytes: u64,
    callback_generation: u64,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[CrossProcessSessionPhase::Active])?;
    validate_session_callback(state, callback_generation)?;
    validate_frame_size(state, payload_bytes)?;
    let mut next = state.clone();
    next.frames_received = checked_increment(state.frames_received).map_err(|issue| vec![issue])?;
    Ok(CrossProcessSessionTransition {
        next,
        action: SessionShellAction::DeliverFrame,
    })
}

fn transition_begin_drain(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[CrossProcessSessionPhase::Active])?;
    applied_session(state, CrossProcessSessionPhase::Draining, SessionShellAction::StopNewFrames)
}

fn transition_close(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[CrossProcessSessionPhase::Active, CrossProcessSessionPhase::Draining])?;
    if state.queued_bytes != 0 || state.inflight_bytes != 0 {
        return Err(vec![CrossProcessTransportIssue::SessionWorkRemains]);
    }
    let mut next = state.clone();
    next.phase = CrossProcessSessionPhase::Closing;
    next.terminal_class = Some(SessionTerminalClass::Clean);
    Ok(CrossProcessSessionTransition {
        next,
        action: SessionShellAction::CloseConnection,
    })
}

fn transition_cancel(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[
        CrossProcessSessionPhase::Planned,
        CrossProcessSessionPhase::Dialing,
        CrossProcessSessionPhase::Active,
        CrossProcessSessionPhase::Queued,
        CrossProcessSessionPhase::AwaitingAcknowledgement,
        CrossProcessSessionPhase::Draining,
    ])?;
    let mut next = state.clone();
    next.phase = CrossProcessSessionPhase::Closing;
    next.delivery = delivery_after_terminal(state, false);
    next.retry = retry_for_delivery(next.delivery);
    next.queued_bytes = 0;
    next.inflight_bytes = 0;
    next.terminal_class = Some(SessionTerminalClass::Cancelled);
    Ok(CrossProcessSessionTransition {
        next,
        action: SessionShellAction::CloseConnection,
    })
}

fn transition_fail(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
    class: SessionTerminalClass,
    delivery_definitive: bool,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[
        CrossProcessSessionPhase::Planned,
        CrossProcessSessionPhase::Dialing,
        CrossProcessSessionPhase::Active,
        CrossProcessSessionPhase::Queued,
        CrossProcessSessionPhase::AwaitingAcknowledgement,
        CrossProcessSessionPhase::Draining,
    ])?;
    let mut next = state.clone();
    next.phase = CrossProcessSessionPhase::Closing;
    next.delivery = delivery_after_terminal(state, delivery_definitive);
    next.retry = retry_for_delivery(next.delivery);
    next.queued_bytes = 0;
    next.inflight_bytes = 0;
    next.terminal_class = Some(class);
    Ok(CrossProcessSessionTransition {
        next,
        action: SessionShellAction::CloseConnection,
    })
}

fn transition_begin_cleanup(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[CrossProcessSessionPhase::Closing])?;
    applied_session(state, CrossProcessSessionPhase::Cleaning, SessionShellAction::PersistCleanup)
}

fn transition_complete_cleanup(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
    cleanup_evidence_ref: &str,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    require_session_phase(state, command, &[CrossProcessSessionPhase::Cleaning])?;
    let mut issues = Vec::new();
    validate_ref("session-cleanup-evidence-ref", cleanup_evidence_ref, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut next = state.clone();
    next.phase = CrossProcessSessionPhase::Closed;
    next.cleanup_evidence_ref = Some(cleanup_evidence_ref.to_string());
    Ok(CrossProcessSessionTransition {
        next,
        action: SessionShellAction::None,
    })
}

fn validate_frame_size(
    state: &CrossProcessSessionState,
    payload_bytes: u64,
) -> Result<(), Vec<CrossProcessTransportIssue>> {
    if payload_bytes == 0 || payload_bytes > state.resources.max_frame_bytes {
        Err(vec![CrossProcessTransportIssue::SessionFrameLimitExceeded])
    } else {
        Ok(())
    }
}

fn validate_session_callback(
    state: &CrossProcessSessionState,
    callback_generation: u64,
) -> Result<(), Vec<CrossProcessTransportIssue>> {
    if callback_generation == state.identity.generation {
        Ok(())
    } else {
        Err(vec![CrossProcessTransportIssue::StaleSessionCallback])
    }
}

fn require_session_phase(
    state: &CrossProcessSessionState,
    command: CrossProcessSessionCommandKind,
    allowed: &[CrossProcessSessionPhase],
) -> Result<(), Vec<CrossProcessTransportIssue>> {
    if allowed.contains(&state.phase) {
        Ok(())
    } else {
        Err(vec![CrossProcessTransportIssue::InvalidSessionTransition {
            from: state.phase,
            command,
        }])
    }
}

fn delivery_after_terminal(state: &CrossProcessSessionState, delivery_definitive: bool) -> DeliveryOutcome {
    if delivery_definitive {
        DeliveryOutcome::NotDelivered
    } else if state.inflight_bytes != 0 || state.phase == CrossProcessSessionPhase::AwaitingAcknowledgement {
        DeliveryOutcome::Uncertain
    } else {
        DeliveryOutcome::NotAttempted
    }
}

fn retry_for_delivery(delivery: DeliveryOutcome) -> RetryDisposition {
    if delivery == DeliveryOutcome::Uncertain {
        RetryDisposition::UnsafeWithoutReconciliation
    } else {
        RetryDisposition::HigherLevelPolicyRequired
    }
}

fn applied_session(
    state: &CrossProcessSessionState,
    phase: CrossProcessSessionPhase,
    action: SessionShellAction,
) -> Result<CrossProcessSessionTransition, Vec<CrossProcessTransportIssue>> {
    let mut next = state.clone();
    next.phase = phase;
    Ok(CrossProcessSessionTransition { next, action })
}
