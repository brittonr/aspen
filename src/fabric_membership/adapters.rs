use std::collections::BTreeMap;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipProviderSnapshot {
    pub profile: MembershipSourceProfile,
    pub view: MembershipView,
    pub descriptors: Vec<NodeDescriptor>,
    pub detector_profiles: Vec<FailureDetectorProfile>,
    pub failure_observations: Vec<FailureObservation>,
    pub reservations: Vec<CapacityReservation>,
    pub observed_now_ticks: u64,
    pub required_compatibility_ref: String,
}

pub trait MembershipPlacementProvider {
    fn provider_kind(&self) -> MembershipProviderKind;
    fn snapshot(&mut self) -> std::result::Result<MembershipProviderSnapshot, String>;
}

#[derive(Debug, Clone)]
pub struct StaticMembershipProvider {
    snapshot: MembershipProviderSnapshot,
}

impl StaticMembershipProvider {
    pub fn new(snapshot: MembershipProviderSnapshot) -> std::result::Result<Self, String> {
        require_provider_kind(&snapshot, MembershipProviderKind::Static)?;
        Ok(Self { snapshot })
    }
}

impl MembershipPlacementProvider for StaticMembershipProvider {
    fn provider_kind(&self) -> MembershipProviderKind {
        MembershipProviderKind::Static
    }

    fn snapshot(&mut self) -> std::result::Result<MembershipProviderSnapshot, String> {
        Ok(self.snapshot.clone())
    }
}

#[derive(Debug, Clone)]
pub struct PolicyManagedMembershipProvider {
    current: MembershipProviderSnapshot,
}

impl PolicyManagedMembershipProvider {
    pub fn new(current: MembershipProviderSnapshot) -> std::result::Result<Self, String> {
        if !matches!(
            current.profile.provider_kind,
            MembershipProviderKind::PolicyManaged | MembershipProviderKind::ConsistencyBacked
        ) {
            return Err("policy-managed provider requires a live policy or consistency profile".to_string());
        }
        Ok(Self { current })
    }

    pub fn replace_snapshot(&mut self, next: MembershipProviderSnapshot) -> std::result::Result<(), String> {
        if next.profile.provider_kind != self.current.profile.provider_kind {
            return Err("live provider kind cannot change during snapshot replacement".to_string());
        }
        if next.profile.profile_ref != self.current.profile.profile_ref {
            return Err("live provider profile cannot drift during snapshot replacement".to_string());
        }
        if next.view.epoch <= self.current.view.epoch {
            return Err("live membership view epoch must advance".to_string());
        }
        self.current = next;
        Ok(())
    }
}

impl MembershipPlacementProvider for PolicyManagedMembershipProvider {
    fn provider_kind(&self) -> MembershipProviderKind {
        self.current.profile.provider_kind
    }

    fn snapshot(&mut self) -> std::result::Result<MembershipProviderSnapshot, String> {
        Ok(self.current.clone())
    }
}

#[derive(Debug, Clone)]
pub struct DeterministicSimulationMembershipProvider {
    snapshots: Vec<MembershipProviderSnapshot>,
    cursor: usize,
}

impl DeterministicSimulationMembershipProvider {
    pub fn new(snapshots: Vec<MembershipProviderSnapshot>) -> std::result::Result<Self, String> {
        if snapshots.is_empty() {
            return Err("deterministic membership provider requires at least one snapshot".to_string());
        }
        let mut previous_epoch = None;
        let expected_profile_ref = snapshots[0].profile.profile_ref.clone();
        let expected_authority_scope = snapshots[0].profile.authority_scope.clone();
        for snapshot in &snapshots {
            require_provider_kind(snapshot, MembershipProviderKind::DeterministicSimulation)?;
            if snapshot.profile.profile_ref != expected_profile_ref
                || snapshot.profile.authority_scope != expected_authority_scope
            {
                return Err(
                    "deterministic membership snapshots cannot drift source profile or authority scope".to_string()
                );
            }
            if previous_epoch.is_some_and(|epoch| snapshot.view.epoch <= epoch) {
                return Err("deterministic membership snapshots must have increasing epochs".to_string());
            }
            previous_epoch = Some(snapshot.view.epoch);
        }
        Ok(Self { snapshots, cursor: 0 })
    }
}

impl MembershipPlacementProvider for DeterministicSimulationMembershipProvider {
    fn provider_kind(&self) -> MembershipProviderKind {
        MembershipProviderKind::DeterministicSimulation
    }

    fn snapshot(&mut self) -> std::result::Result<MembershipProviderSnapshot, String> {
        let snapshot = self
            .snapshots
            .get(self.cursor)
            .ok_or_else(|| "deterministic membership snapshot stream is exhausted".to_string())?
            .clone();
        self.cursor = self
            .cursor
            .checked_add(1)
            .ok_or_else(|| "deterministic membership snapshot cursor overflow".to_string())?;
        Ok(snapshot)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedProviderSnapshot {
    pub membership: CanonicalMembershipView,
    pub failures: CanonicalFailureObservationSet,
    pub reservations: Vec<CapacityReservation>,
}

// r[impl molten.fabric_membership.live_sim_parity]
// r[impl molten.fabric_membership.authority_separation]
pub fn observe_provider(
    provider: &mut dyn MembershipPlacementProvider,
) -> crate::error::Result<AdmittedProviderSnapshot> {
    let snapshot = provider
        .snapshot()
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("membership provider failed: {error}")))?;
    if provider.provider_kind() != snapshot.profile.provider_kind {
        return Err(crate::error::MoltenError::invalid_harness(
            "membership provider kind differs from its source profile",
        ));
    }
    let profile = canonical_membership_profile(&snapshot.profile)?;
    let membership = canonical_membership_view(
        &profile,
        &snapshot.view,
        &snapshot.descriptors,
        snapshot.observed_now_ticks,
        &snapshot.required_compatibility_ref,
    )?;
    let failures = canonical_failure_observations(
        &membership,
        &snapshot.detector_profiles,
        &snapshot.failure_observations,
        snapshot.observed_now_ticks,
    )?;
    Ok(AdmittedProviderSnapshot {
        membership,
        failures,
        reservations: snapshot.reservations,
    })
}

pub trait AssignmentPersistence {
    fn record_intent(
        &mut self,
        current: &RoleAssignment,
        transition: &AssignmentTransition,
    ) -> std::result::Result<String, String>;

    fn commit(
        &mut self,
        transition: &AssignmentTransition,
        intent_ref: &str,
        role_effect_ref: Option<&str>,
    ) -> std::result::Result<String, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleEffectFailure {
    pub message: String,
    pub effect_may_have_happened: bool,
}

pub trait ExtensionRoleLifecyclePort {
    fn activate(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;
    fn begin_drain(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;
    fn begin_replacement(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;
    fn release(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;
    fn fail(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;
    fn quarantine(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentExecutionPhase {
    IntentPersistence,
    RoleEffect,
    CommitPersistence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncertainAssignmentExecution {
    pub transition: AssignmentTransition,
    pub phase: AssignmentExecutionPhase,
    pub message: String,
    pub intent_ref: Option<String>,
    pub role_effect_ref: Option<String>,
    pub effect_may_have_happened: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentExecutionOutcome {
    Committed(CanonicalAssignmentTransition),
    Uncertain(UncertainAssignmentExecution),
}

// r[impl molten.fabric_membership.recruitment]
// r[impl molten.fabric_membership.drain_replace]
// r[impl molten.fabric_membership.evidence]
pub fn execute_assignment_command(
    persistence: &mut dyn AssignmentPersistence,
    lifecycle: &mut dyn ExtensionRoleLifecyclePort,
    current: &RoleAssignment,
    command: &AssignmentCommand,
    authority: &AssignmentAuthoritySnapshot,
) -> std::result::Result<AssignmentExecutionOutcome, Vec<AssignmentIssue>> {
    let authority_issues = validate_assignment_authority(current, authority);
    if !authority_issues.is_empty() {
        return Err(authority_issues);
    }
    let transition = apply_assignment_command(current, command)?;
    let intent_ref = match persistence.record_intent(current, &transition) {
        Ok(intent_ref) => intent_ref,
        Err(message) => {
            return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
                transition,
                phase: AssignmentExecutionPhase::IntentPersistence,
                message,
                intent_ref: None,
                role_effect_ref: None,
                effect_may_have_happened: false,
            }));
        }
    };
    if !valid_evidence_ref(&intent_ref) {
        return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
            transition,
            phase: AssignmentExecutionPhase::IntentPersistence,
            message: "assignment persistence returned a malformed intent ref".to_string(),
            intent_ref: Some(intent_ref),
            role_effect_ref: None,
            effect_may_have_happened: false,
        }));
    }
    let role_effect_ref = match run_role_effect(lifecycle, &transition) {
        Ok(effect_ref) => effect_ref,
        Err(failure) => {
            return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
                transition,
                phase: AssignmentExecutionPhase::RoleEffect,
                message: failure.message,
                intent_ref: Some(intent_ref),
                role_effect_ref: None,
                effect_may_have_happened: failure.effect_may_have_happened,
            }));
        }
    };
    if role_effect_ref.as_deref().is_some_and(|effect_ref| !valid_evidence_ref(effect_ref)) {
        return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
            transition,
            phase: AssignmentExecutionPhase::RoleEffect,
            message: "extension lifecycle returned a malformed effect ref".to_string(),
            intent_ref: Some(intent_ref),
            role_effect_ref,
            effect_may_have_happened: true,
        }));
    }
    let persistence_ref = match persistence.commit(&transition, &intent_ref, role_effect_ref.as_deref()) {
        Ok(persistence_ref) => persistence_ref,
        Err(message) => {
            return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
                transition,
                phase: AssignmentExecutionPhase::CommitPersistence,
                message,
                intent_ref: Some(intent_ref),
                role_effect_ref,
                effect_may_have_happened: true,
            }));
        }
    };
    if !valid_evidence_ref(&persistence_ref) {
        return Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
            transition,
            phase: AssignmentExecutionPhase::CommitPersistence,
            message: "assignment persistence returned a malformed commit ref".to_string(),
            intent_ref: Some(intent_ref),
            role_effect_ref,
            effect_may_have_happened: true,
        }));
    }
    match canonical_assignment_transition(&transition, &intent_ref, role_effect_ref.as_deref(), &persistence_ref) {
        Ok(canonical) => Ok(AssignmentExecutionOutcome::Committed(canonical)),
        Err(error) => Ok(AssignmentExecutionOutcome::Uncertain(UncertainAssignmentExecution {
            transition,
            phase: AssignmentExecutionPhase::CommitPersistence,
            message: format!("canonical assignment evidence failed after commit: {error}"),
            intent_ref: Some(intent_ref),
            role_effect_ref,
            effect_may_have_happened: true,
        })),
    }
}

fn run_role_effect(
    lifecycle: &mut dyn ExtensionRoleLifecyclePort,
    transition: &AssignmentTransition,
) -> std::result::Result<Option<String>, RoleEffectFailure> {
    let effect = match transition.kind {
        AssignmentCommandKind::Activate => Some(lifecycle.activate(&transition.next)?),
        AssignmentCommandKind::BeginDrain => Some(lifecycle.begin_drain(&transition.next)?),
        AssignmentCommandKind::BeginReplacement => Some(lifecycle.begin_replacement(&transition.next)?),
        AssignmentCommandKind::Release => Some(lifecycle.release(&transition.next)?),
        AssignmentCommandKind::Fail => Some(lifecycle.fail(&transition.next)?),
        AssignmentCommandKind::Quarantine => Some(lifecycle.quarantine(&transition.next)?),
        AssignmentCommandKind::Reserve | AssignmentCommandKind::Assign | AssignmentCommandKind::Acknowledge => None,
    };
    Ok(effect)
}

#[derive(Debug, Default)]
pub struct InMemoryAssignmentPersistence {
    pub assignments: BTreeMap<String, RoleAssignment>,
    pub intents: Vec<String>,
    pub commits: Vec<String>,
    pub fail_intent: bool,
    pub fail_commit: bool,
}

impl AssignmentPersistence for InMemoryAssignmentPersistence {
    fn record_intent(
        &mut self,
        current: &RoleAssignment,
        transition: &AssignmentTransition,
    ) -> std::result::Result<String, String> {
        if self.fail_intent {
            return Err("injected assignment intent failure".to_string());
        }
        let value = crate::preserves_rail::record("fabric-assignment-intent-v1", vec![
            crate::preserves_rail::string(&current.assignment_id),
            crate::preserves_rail::string(current.state.as_str()),
            crate::preserves_rail::string(transition.next.state.as_str()),
            crate::preserves_rail::u64_value(current.assignment_epoch),
        ]);
        let intent_ref = crate::preserves_rail::canonical_hash(&value).map_err(|error| error.to_string())?;
        self.intents.push(intent_ref.clone());
        Ok(intent_ref)
    }

    fn commit(
        &mut self,
        transition: &AssignmentTransition,
        intent_ref: &str,
        role_effect_ref: Option<&str>,
    ) -> std::result::Result<String, String> {
        if self.fail_commit {
            return Err("injected assignment commit failure".to_string());
        }
        let value = crate::preserves_rail::record("fabric-assignment-commit-v1", vec![
            crate::preserves_rail::string(intent_ref),
            crate::preserves_rail::string(role_effect_ref.unwrap_or("no-role-effect")),
            crate::preserves_rail::string(&transition.next.assignment_id),
            crate::preserves_rail::string(transition.next.state.as_str()),
        ]);
        let commit_ref = crate::preserves_rail::canonical_hash(&value).map_err(|error| error.to_string())?;
        self.assignments.insert(transition.next.assignment_id.clone(), transition.next.clone());
        self.commits.push(commit_ref.clone());
        Ok(commit_ref)
    }
}

fn valid_evidence_ref(value: &str) -> bool {
    const BLAKE3_PREFIX: &str = "blake3:";
    const BLAKE3_HEX_LENGTH: usize = 64;
    value.strip_prefix(BLAKE3_PREFIX).is_some_and(|hex| {
        hex.len() == BLAKE3_HEX_LENGTH && hex.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
    })
}

fn require_provider_kind(
    snapshot: &MembershipProviderSnapshot,
    expected: MembershipProviderKind,
) -> std::result::Result<(), String> {
    if snapshot.profile.provider_kind == expected {
        Ok(())
    } else {
        Err(format!(
            "membership provider profile kind {} does not match adapter {}",
            snapshot.profile.provider_kind.as_str(),
            expected.as_str()
        ))
    }
}
