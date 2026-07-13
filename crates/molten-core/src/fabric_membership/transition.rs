use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FencingEnforcement {
    ProcessLocal,
    NodeLocalDurable,
    QuorumOrdered,
    ExternallyEnforced,
}

impl FencingEnforcement {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProcessLocal => "process-local",
            Self::NodeLocalDurable => "node-local-durable",
            Self::QuorumOrdered => "quorum-ordered",
            Self::ExternallyEnforced => "externally-enforced",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FencingProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub authority_ref: String,
    pub effect_port_ref: String,
    pub enforcement: FencingEnforcement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentState {
    Proposed,
    Reserved,
    Assigned,
    Acknowledged,
    Active,
    Draining,
    Replacing,
    Released,
    Failed,
    Quarantined,
}

impl AssignmentState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Reserved => "reserved",
            Self::Assigned => "assigned",
            Self::Acknowledged => "acknowledged",
            Self::Active => "active",
            Self::Draining => "draining",
            Self::Replacing => "replacing",
            Self::Released => "released",
            Self::Failed => "failed",
            Self::Quarantined => "quarantined",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Released | Self::Quarantined)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentProposal {
    pub assignment_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub role_id: String,
    pub role_kind: String,
    pub node_id: String,
    pub service_generation: u64,
    pub assignment_epoch: u64,
    pub fencing_token: u64,
    pub fencing_profile_ref: String,
    pub resource_reservation_ref: String,
    pub placement_plan_ref: String,
    pub authority_ref: String,
    pub durable_state_ref: Option<String>,
    pub predecessor_assignment_ref: Option<String>,
    pub predecessor_epoch: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleAssignment {
    pub schema: String,
    pub assignment_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub role_id: String,
    pub role_kind: String,
    pub node_id: String,
    pub service_generation: u64,
    pub assignment_epoch: u64,
    pub fencing_token: u64,
    pub fencing_profile_ref: String,
    pub resource_reservation_ref: String,
    pub placement_plan_ref: String,
    pub authority_ref: String,
    pub durable_state_ref: Option<String>,
    pub predecessor_assignment_ref: Option<String>,
    pub predecessor_epoch: Option<u64>,
    pub state: AssignmentState,
    pub uncertain_old_owner: bool,
    pub transition_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentCommandKind {
    Reserve,
    Assign,
    Acknowledge,
    Activate,
    BeginDrain,
    BeginReplacement,
    Release,
    Fail,
    Quarantine,
}

impl AssignmentCommandKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reserve => "reserve",
            Self::Assign => "assign",
            Self::Acknowledge => "acknowledge",
            Self::Activate => "activate",
            Self::BeginDrain => "begin-drain",
            Self::BeginReplacement => "begin-replacement",
            Self::Release => "release",
            Self::Fail => "fail",
            Self::Quarantine => "quarantine",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentCommand {
    pub kind: AssignmentCommandKind,
    pub assignment_id: String,
    pub service_generation: u64,
    pub assignment_epoch: u64,
    pub fencing_token: u64,
    pub transition_ref: String,
    pub uncertain_old_owner: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentTransition {
    pub previous_state: AssignmentState,
    pub next: RoleAssignment,
    pub kind: AssignmentCommandKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentIssue {
    SchemaMismatch,
    EmptyField(&'static str),
    MalformedToken(&'static str),
    MalformedRef(&'static str),
    ZeroServiceGeneration,
    ZeroAssignmentEpoch,
    ZeroFencingToken,
    PredecessorFieldsIncomplete,
    SuccessorEpochNotAdvanced,
    AssignmentIdentityMismatch,
    StaleServiceGeneration {
        expected: u64,
        actual: u64,
    },
    StaleAssignmentEpoch {
        expected: u64,
        actual: u64,
    },
    StaleFencingToken {
        expected: u64,
        actual: u64,
    },
    InvalidTransition {
        state: AssignmentState,
        command: AssignmentCommandKind,
    },
    DuplicateTransitionRef,
    TransitionLimitExceeded,
    FencingProfileMismatch,
    FencingAuthorityMismatch,
    FencingStrengthInsufficient {
        available: FencingEnforcement,
        required: FencingEnforcement,
    },
    AssignmentNotActive(AssignmentState),
    AssignmentNotReleased(AssignmentState),
    ResourceReservationMismatch,
    OperationScopeMismatch,
    DrainStateRequired,
    DrainDeadlineInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentAuthoritySnapshot {
    pub fencing_profile: FencingProfile,
    pub enforced_assignment_epoch: u64,
    pub enforced_fencing_token: u64,
    pub required_enforcement: FencingEnforcement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FencedOperation {
    pub assignment_id: String,
    pub service_generation: u64,
    pub assignment_epoch: u64,
    pub fencing_token: u64,
    pub fencing_profile_ref: String,
    pub authority_ref: String,
    pub required_enforcement: FencingEnforcement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrainProgress {
    pub assignment_id: String,
    pub assignment_epoch: u64,
    pub new_work_stopped: bool,
    pub handoff_required: bool,
    pub handoff_complete: bool,
    pub checkpoint_ref: Option<String>,
    pub role_stopped: bool,
    pub release_acknowledged: bool,
    pub grace_deadline_ticks: u64,
    pub now_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainDecision {
    Continue,
    ReadyToRelease,
    ForceReleaseUncertain,
}

impl DrainDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::ReadyToRelease => "ready-to-release",
            Self::ForceReleaseUncertain => "force-release-uncertain",
        }
    }
}

// r[impl molten.fabric_membership.recruitment]
// r[impl molten.fabric_membership.fencing]
pub fn propose_assignment(proposal: &AssignmentProposal) -> Result<RoleAssignment, Vec<AssignmentIssue>> {
    let mut issues = Vec::new();
    for (field, value) in [
        ("assignment-id", proposal.assignment_id.as_str()),
        ("extension-id", proposal.extension_id.as_str()),
        ("service-id", proposal.service_id.as_str()),
        ("role-id", proposal.role_id.as_str()),
        ("role-kind", proposal.role_kind.as_str()),
        ("node-id", proposal.node_id.as_str()),
    ] {
        validate_assignment_token(field, value, &mut issues);
    }
    for (field, value) in [
        ("fencing-profile-ref", proposal.fencing_profile_ref.as_str()),
        ("resource-reservation-ref", proposal.resource_reservation_ref.as_str()),
        ("placement-plan-ref", proposal.placement_plan_ref.as_str()),
        ("authority-ref", proposal.authority_ref.as_str()),
    ] {
        validate_assignment_ref(field, value, &mut issues);
    }
    for (field, value) in [
        ("durable-state-ref", proposal.durable_state_ref.as_deref()),
        ("predecessor-assignment-ref", proposal.predecessor_assignment_ref.as_deref()),
    ] {
        if let Some(value) = value {
            validate_assignment_ref(field, value, &mut issues);
        }
    }
    if proposal.service_generation == 0 {
        issues.push(AssignmentIssue::ZeroServiceGeneration);
    }
    if proposal.assignment_epoch == 0 {
        issues.push(AssignmentIssue::ZeroAssignmentEpoch);
    }
    if proposal.fencing_token == 0 {
        issues.push(AssignmentIssue::ZeroFencingToken);
    }
    if proposal.predecessor_assignment_ref.is_some() != proposal.predecessor_epoch.is_some() {
        issues.push(AssignmentIssue::PredecessorFieldsIncomplete);
    }
    if proposal.predecessor_epoch.is_some_and(|epoch| proposal.assignment_epoch <= epoch) {
        issues.push(AssignmentIssue::SuccessorEpochNotAdvanced);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(RoleAssignment {
        schema: ROLE_ASSIGNMENT_SCHEMA.to_string(),
        assignment_id: proposal.assignment_id.clone(),
        extension_id: proposal.extension_id.clone(),
        service_id: proposal.service_id.clone(),
        role_id: proposal.role_id.clone(),
        role_kind: proposal.role_kind.clone(),
        node_id: proposal.node_id.clone(),
        service_generation: proposal.service_generation,
        assignment_epoch: proposal.assignment_epoch,
        fencing_token: proposal.fencing_token,
        fencing_profile_ref: proposal.fencing_profile_ref.clone(),
        resource_reservation_ref: proposal.resource_reservation_ref.clone(),
        placement_plan_ref: proposal.placement_plan_ref.clone(),
        authority_ref: proposal.authority_ref.clone(),
        durable_state_ref: proposal.durable_state_ref.clone(),
        predecessor_assignment_ref: proposal.predecessor_assignment_ref.clone(),
        predecessor_epoch: proposal.predecessor_epoch,
        state: AssignmentState::Proposed,
        uncertain_old_owner: false,
        transition_refs: Vec::new(),
    })
}

// r[impl molten.fabric_membership.recruitment]
pub fn apply_assignment_command(
    assignment: &RoleAssignment,
    command: &AssignmentCommand,
) -> Result<AssignmentTransition, Vec<AssignmentIssue>> {
    let mut issues = validate_assignment(assignment);
    validate_assignment_token("command-assignment-id", &command.assignment_id, &mut issues);
    validate_assignment_ref("command-transition-ref", &command.transition_ref, &mut issues);
    if command.assignment_id != assignment.assignment_id {
        issues.push(AssignmentIssue::AssignmentIdentityMismatch);
    }
    if command.service_generation != assignment.service_generation {
        issues.push(AssignmentIssue::StaleServiceGeneration {
            expected: assignment.service_generation,
            actual: command.service_generation,
        });
    }
    if command.assignment_epoch != assignment.assignment_epoch {
        issues.push(AssignmentIssue::StaleAssignmentEpoch {
            expected: assignment.assignment_epoch,
            actual: command.assignment_epoch,
        });
    }
    if command.fencing_token != assignment.fencing_token {
        issues.push(AssignmentIssue::StaleFencingToken {
            expected: assignment.fencing_token,
            actual: command.fencing_token,
        });
    }
    if assignment.transition_refs.contains(&command.transition_ref) {
        issues.push(AssignmentIssue::DuplicateTransitionRef);
    }
    if assignment.transition_refs.len() >= MAX_MEMBERSHIP_ITEMS {
        issues.push(AssignmentIssue::TransitionLimitExceeded);
    }
    let next_state = transition_state(assignment.state, command.kind).unwrap_or_else(|| {
        issues.push(AssignmentIssue::InvalidTransition {
            state: assignment.state,
            command: command.kind,
        });
        assignment.state
    });
    if command.uncertain_old_owner
        && !matches!(command.kind, AssignmentCommandKind::BeginReplacement | AssignmentCommandKind::Fail)
    {
        issues.push(AssignmentIssue::InvalidTransition {
            state: assignment.state,
            command: command.kind,
        });
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut next = assignment.clone();
    next.state = next_state;
    next.uncertain_old_owner |= command.uncertain_old_owner;
    next.transition_refs.push(command.transition_ref.clone());
    Ok(AssignmentTransition {
        previous_state: assignment.state,
        next,
        kind: command.kind,
    })
}

fn transition_state(state: AssignmentState, command: AssignmentCommandKind) -> Option<AssignmentState> {
    match (state, command) {
        (AssignmentState::Proposed, AssignmentCommandKind::Reserve) => Some(AssignmentState::Reserved),
        (AssignmentState::Reserved, AssignmentCommandKind::Assign) => Some(AssignmentState::Assigned),
        (AssignmentState::Assigned, AssignmentCommandKind::Acknowledge) => Some(AssignmentState::Acknowledged),
        (AssignmentState::Acknowledged, AssignmentCommandKind::Activate) => Some(AssignmentState::Active),
        (AssignmentState::Active, AssignmentCommandKind::BeginDrain) => Some(AssignmentState::Draining),
        (
            AssignmentState::Active | AssignmentState::Draining | AssignmentState::Failed,
            AssignmentCommandKind::BeginReplacement,
        ) => Some(AssignmentState::Replacing),
        (
            AssignmentState::Reserved
            | AssignmentState::Assigned
            | AssignmentState::Acknowledged
            | AssignmentState::Draining
            | AssignmentState::Replacing
            | AssignmentState::Failed,
            AssignmentCommandKind::Release,
        ) => Some(AssignmentState::Released),
        (
            AssignmentState::Proposed
            | AssignmentState::Reserved
            | AssignmentState::Assigned
            | AssignmentState::Acknowledged
            | AssignmentState::Active
            | AssignmentState::Draining
            | AssignmentState::Replacing,
            AssignmentCommandKind::Fail,
        ) => Some(AssignmentState::Failed),
        (state, AssignmentCommandKind::Quarantine) if !state.is_terminal() => Some(AssignmentState::Quarantined),
        _ => None,
    }
}

pub fn validate_assignment(assignment: &RoleAssignment) -> Vec<AssignmentIssue> {
    let mut issues = Vec::new();
    if assignment.schema != ROLE_ASSIGNMENT_SCHEMA {
        issues.push(AssignmentIssue::SchemaMismatch);
    }
    for (field, value) in [
        ("assignment-id", assignment.assignment_id.as_str()),
        ("extension-id", assignment.extension_id.as_str()),
        ("service-id", assignment.service_id.as_str()),
        ("role-id", assignment.role_id.as_str()),
        ("role-kind", assignment.role_kind.as_str()),
        ("node-id", assignment.node_id.as_str()),
    ] {
        validate_assignment_token(field, value, &mut issues);
    }
    for (field, value) in [
        ("fencing-profile-ref", assignment.fencing_profile_ref.as_str()),
        ("resource-reservation-ref", assignment.resource_reservation_ref.as_str()),
        ("placement-plan-ref", assignment.placement_plan_ref.as_str()),
        ("authority-ref", assignment.authority_ref.as_str()),
    ] {
        validate_assignment_ref(field, value, &mut issues);
    }
    for (field, value) in [
        ("durable-state-ref", assignment.durable_state_ref.as_deref()),
        ("predecessor-assignment-ref", assignment.predecessor_assignment_ref.as_deref()),
    ] {
        if let Some(value) = value {
            validate_assignment_ref(field, value, &mut issues);
        }
    }
    if assignment.service_generation == 0 {
        issues.push(AssignmentIssue::ZeroServiceGeneration);
    }
    if assignment.assignment_epoch == 0 {
        issues.push(AssignmentIssue::ZeroAssignmentEpoch);
    }
    if assignment.fencing_token == 0 {
        issues.push(AssignmentIssue::ZeroFencingToken);
    }
    if assignment.predecessor_assignment_ref.is_some() != assignment.predecessor_epoch.is_some() {
        issues.push(AssignmentIssue::PredecessorFieldsIncomplete);
    }
    if assignment.predecessor_epoch.is_some_and(|epoch| assignment.assignment_epoch <= epoch) {
        issues.push(AssignmentIssue::SuccessorEpochNotAdvanced);
    }
    if assignment.transition_refs.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(AssignmentIssue::TransitionLimitExceeded);
    }
    let unique_transition_refs = assignment.transition_refs.iter().collect::<BTreeSet<_>>();
    if unique_transition_refs.len() != assignment.transition_refs.len() {
        issues.push(AssignmentIssue::DuplicateTransitionRef);
    }
    for transition_ref in &assignment.transition_refs {
        validate_assignment_ref("assignment-transition-ref", transition_ref, &mut issues);
    }
    issues
}

// r[impl molten.fabric_membership.fencing]
pub fn validate_fencing_profile(profile: &FencingProfile) -> Vec<AssignmentIssue> {
    let mut issues = Vec::new();
    if profile.schema != FENCING_PROFILE_SCHEMA {
        issues.push(AssignmentIssue::SchemaMismatch);
    }
    validate_assignment_token("fencing-profile-id", &profile.profile_id, &mut issues);
    for (field, value) in [
        ("fencing-profile-ref", profile.profile_ref.as_str()),
        ("fencing-authority-ref", profile.authority_ref.as_str()),
        ("fencing-effect-port-ref", profile.effect_port_ref.as_str()),
    ] {
        validate_assignment_ref(field, value, &mut issues);
    }
    issues
}

// r[impl molten.fabric_membership.fencing]
pub fn validate_assignment_authority(
    assignment: &RoleAssignment,
    authority: &AssignmentAuthoritySnapshot,
) -> Vec<AssignmentIssue> {
    let mut issues = validate_assignment(assignment);
    issues.extend(validate_fencing_profile(&authority.fencing_profile));
    if assignment.fencing_profile_ref != authority.fencing_profile.profile_ref {
        issues.push(AssignmentIssue::FencingProfileMismatch);
    }
    if assignment.authority_ref != authority.fencing_profile.authority_ref {
        issues.push(AssignmentIssue::FencingAuthorityMismatch);
    }
    if assignment.assignment_epoch != authority.enforced_assignment_epoch {
        issues.push(AssignmentIssue::StaleAssignmentEpoch {
            expected: authority.enforced_assignment_epoch,
            actual: assignment.assignment_epoch,
        });
    }
    if assignment.fencing_token != authority.enforced_fencing_token {
        issues.push(AssignmentIssue::StaleFencingToken {
            expected: authority.enforced_fencing_token,
            actual: assignment.fencing_token,
        });
    }
    if authority.fencing_profile.enforcement < authority.required_enforcement {
        issues.push(AssignmentIssue::FencingStrengthInsufficient {
            available: authority.fencing_profile.enforcement,
            required: authority.required_enforcement,
        });
    }
    issues
}

// r[impl molten.fabric_membership.fencing]
pub fn validate_fenced_operation(
    assignment: &RoleAssignment,
    profile: &FencingProfile,
    operation: &FencedOperation,
    enforced_assignment_epoch: u64,
    enforced_fencing_token: u64,
) -> Result<(), Vec<AssignmentIssue>> {
    let mut issues = validate_assignment(assignment);
    issues.extend(validate_fencing_profile(profile));
    if assignment.state != AssignmentState::Active {
        issues.push(AssignmentIssue::AssignmentNotActive(assignment.state));
    }
    if assignment.assignment_id != operation.assignment_id
        || assignment.service_generation != operation.service_generation
    {
        issues.push(AssignmentIssue::OperationScopeMismatch);
    }
    if assignment.fencing_profile_ref != profile.profile_ref || operation.fencing_profile_ref != profile.profile_ref {
        issues.push(AssignmentIssue::FencingProfileMismatch);
    }
    if assignment.authority_ref != profile.authority_ref || operation.authority_ref != profile.authority_ref {
        issues.push(AssignmentIssue::FencingAuthorityMismatch);
    }
    if operation.assignment_epoch != assignment.assignment_epoch
        || operation.assignment_epoch != enforced_assignment_epoch
    {
        issues.push(AssignmentIssue::StaleAssignmentEpoch {
            expected: enforced_assignment_epoch,
            actual: operation.assignment_epoch,
        });
    }
    if operation.fencing_token != assignment.fencing_token || operation.fencing_token != enforced_fencing_token {
        issues.push(AssignmentIssue::StaleFencingToken {
            expected: enforced_fencing_token,
            actual: operation.fencing_token,
        });
    }
    if profile.enforcement < operation.required_enforcement {
        issues.push(AssignmentIssue::FencingStrengthInsufficient {
            available: profile.enforcement,
            required: operation.required_enforcement,
        });
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

// r[impl molten.fabric_membership.drain_replace]
pub fn release_capacity_reservation(
    reservation: &CapacityReservation,
    assignment: &RoleAssignment,
) -> Result<CapacityReservation, AssignmentIssue> {
    if assignment.state != AssignmentState::Released {
        return Err(AssignmentIssue::AssignmentNotReleased(assignment.state));
    }
    if reservation.reservation_ref != assignment.resource_reservation_ref
        || reservation.node_id != assignment.node_id
        || reservation.assignment_epoch != assignment.assignment_epoch
    {
        return Err(AssignmentIssue::ResourceReservationMismatch);
    }
    let mut released = reservation.clone();
    released.released = true;
    Ok(released)
}

// r[impl molten.fabric_membership.drain_replace]
pub fn evaluate_drain(
    assignment: &RoleAssignment,
    progress: &DrainProgress,
) -> Result<DrainDecision, Vec<AssignmentIssue>> {
    let mut issues = validate_assignment(assignment);
    if assignment.state != AssignmentState::Draining {
        issues.push(AssignmentIssue::DrainStateRequired);
    }
    if progress.assignment_id != assignment.assignment_id || progress.assignment_epoch != assignment.assignment_epoch {
        issues.push(AssignmentIssue::OperationScopeMismatch);
    }
    if progress.grace_deadline_ticks == 0 {
        issues.push(AssignmentIssue::DrainDeadlineInvalid);
    }
    if let Some(checkpoint_ref) = &progress.checkpoint_ref {
        validate_assignment_ref("drain-checkpoint-ref", checkpoint_ref, &mut issues);
    }
    if progress.handoff_required && progress.handoff_complete && progress.checkpoint_ref.is_none() {
        issues.push(AssignmentIssue::MalformedRef("drain-checkpoint-ref"));
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let handoff_satisfied = !progress.handoff_required || progress.handoff_complete;
    if progress.new_work_stopped && handoff_satisfied && progress.role_stopped && progress.release_acknowledged {
        return Ok(DrainDecision::ReadyToRelease);
    }
    if progress.now_ticks >= progress.grace_deadline_ticks {
        return Ok(DrainDecision::ForceReleaseUncertain);
    }
    Ok(DrainDecision::Continue)
}

fn validate_assignment_token(field: &'static str, value: &str, issues: &mut Vec<AssignmentIssue>) {
    if value.is_empty() {
        issues.push(AssignmentIssue::EmptyField(field));
    } else if value.len() > MAX_MEMBERSHIP_TEXT_BYTES || !valid_fabric_token(value) {
        issues.push(AssignmentIssue::MalformedToken(field));
    }
}

fn validate_assignment_ref(field: &'static str, value: &str, issues: &mut Vec<AssignmentIssue>) {
    if !valid_blake3_ref(value) {
        issues.push(AssignmentIssue::MalformedRef(field));
    }
}
