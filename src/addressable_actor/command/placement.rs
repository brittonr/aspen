/// Independently provisioned placement inputs, never a caller-authored Active assignment.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
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

impl Selection {
    pub fn proposal(&self) -> crate::fabric_membership::AssignmentProposal {
        crate::fabric_membership::AssignmentProposal {
            assignment_id: self.assignment_id.clone(),
            extension_id: self.extension_id.clone(),
            service_id: self.service_id.clone(),
            role_id: self.role_id.clone(),
            role_kind: self.role_kind.clone(),
            node_id: self.node_id.clone(),
            service_generation: self.service_generation,
            assignment_epoch: self.assignment_epoch,
            fencing_token: self.fencing_token,
            fencing_profile_ref: self.fencing_profile_ref.clone(),
            resource_reservation_ref: self.resource_reservation_ref.clone(),
            placement_plan_ref: self.placement_plan_ref.clone(),
            authority_ref: self.authority_ref.clone(),
            durable_state_ref: self.durable_state_ref.clone(),
            predecessor_assignment_ref: self.predecessor_assignment_ref.clone(),
            predecessor_epoch: self.predecessor_epoch,
        }
    }

    pub fn validate(&self, actor_key_ref: &str) -> crate::error::Result<()> {
        if self.extension_id != "molten.addressable-counter"
            || self.service_id != actor_key_ref
            || self.service_generation != 1
            || self.predecessor_assignment_ref.is_some()
            || self.predecessor_epoch.is_some()
        {
            return Err(super::invalid("counter placement must select the exact initial actor service"));
        }
        crate::fabric_membership::propose_assignment(&self.proposal())
            .map_err(|issues| super::invalid(format!("counter placement proposal denied: {issues:?}")))?;
        Ok(())
    }
}

use crate::addressable_actor::native::SigningPort;
use crate::fabric_membership::*;
use crate::node_service_hosting::AuthorityPort;

pub(super) fn initialize(session: &mut super::Session, path: &str) -> crate::error::Result<serde_json::Value> {
    let proposal: Selection = super::read_json(&session.root, path)?;
    proposal.validate(&session.member())?;
    if proposal.node_id != session.configuration.node_id
        || proposal.assignment_id != session.configuration.assignment_id
        || proposal.role_id != session.configuration.role_id
        || proposal.fencing_profile_ref != session.configuration.profile.placement_profile_ref
    {
        return Err(super::invalid("placement proposal differs from selected actor configuration"));
    }
    let input = serde_json::to_string(&proposal).map_err(super::problem)?;
    let input_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "actor-placement-proposal-input-v1",
        vec![crate::preserves_rail::string(input)],
    ))?;
    let empty = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "uninitialized-actor-placement-v1",
        vec![],
    ))?;
    authorize(session, "actor-placement-initialize", &proposal.assignment_id, &empty, &input_ref)?;
    let current = session.placement.initialize(&proposal.proposal()).map_err(super::problem)?;
    Ok(
        serde_json::json!({"assignment_ref": current.assignment_ref, "state": current.assignment.state.as_str(),
        "authority": "exact-current-operator-invocation", "semantic_outcome": "progress"}),
    )
}

pub(super) fn step(session: &mut super::Session, input: &serde_json::Value) -> crate::error::Result<serde_json::Value> {
    let kind = match input.get("kind").and_then(serde_json::Value::as_str) {
        Some("reserve") => AssignmentCommandKind::Reserve,
        Some("assign") => AssignmentCommandKind::Assign,
        Some("acknowledge") => AssignmentCommandKind::Acknowledge,
        Some("activate") => AssignmentCommandKind::Activate,
        _ => return Err(super::invalid("bounded actor placement supports reserve, assign, acknowledge and activate")),
    };
    let current = session
        .placement
        .current(&session.configuration.assignment_id)
        .map_err(super::problem)?
        .ok_or_else(|| super::invalid("actor placement is not initialized"))?;
    let native = session.observe()?;
    let effect_port_ref = validated_effect_port(session, &current, &native)?;
    let input_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("actor-placement-step-input-v1", vec![
            crate::preserves_rail::string(&current.assignment_ref),
            crate::preserves_rail::string(kind.as_str()),
            crate::preserves_rail::string(&native.manifest_ref),
            crate::preserves_rail::u64_value(session.tick),
            crate::preserves_rail::string(&session.membership.admission_ref),
        ]))?;
    authorize(
        session,
        "actor-placement-step",
        &current.assignment.assignment_id,
        &current.assignment_ref,
        &input_ref,
    )?;
    let command = AssignmentCommand {
        kind,
        assignment_id: current.assignment.assignment_id.clone(),
        service_generation: current.assignment.service_generation,
        assignment_epoch: current.assignment.assignment_epoch,
        fencing_token: current.assignment.fencing_token,
        transition_ref: input_ref,
        uncertain_old_owner: false,
    };
    // The durable current role slot supplies the enforced epoch/token, not caller booleans.
    let authority = AssignmentAuthoritySnapshot {
        fencing_profile: FencingProfile {
            schema: FENCING_PROFILE_SCHEMA.to_string(),
            profile_id: "counter-node-local-fencing-v1".to_string(),
            profile_ref: current.assignment.fencing_profile_ref.clone(),
            authority_ref: current.assignment.authority_ref.clone(),
            effect_port_ref,
            enforcement: FencingEnforcement::NodeLocalDurable,
        },
        enforced_assignment_epoch: current.assignment.assignment_epoch,
        enforced_fencing_token: current.assignment.fencing_token,
        required_enforcement: FencingEnforcement::NodeLocalDurable,
    };
    let mut native = Native {
        owner: &mut session.owner,
        signing: &mut session.signing,
    };
    let mut lifecycle = Lifecycle::new(&mut native, session.tick);
    let outcome = execute_and_abort_pre_effect(
        &mut session.placement,
        &mut lifecycle,
        &current.assignment,
        &command,
        &authority,
    )?;
    step_result(session, outcome)
}

/// Runs one placement command and, when the lifecycle refused the role effect before any native
/// effect, clears that exact pending intent through the owner's port-verified abort so the next
/// placement step reads a committed current assignment instead of failing on the pending intent.
/// The refusal itself is still returned as an error.
fn execute_and_abort_pre_effect(
    placement: &mut dyn AssignmentPersistence,
    lifecycle: &mut Lifecycle<'_>,
    current: &RoleAssignment,
    command: &AssignmentCommand,
    authority: &AssignmentAuthoritySnapshot,
) -> crate::error::Result<AssignmentExecutionOutcome> {
    let outcome =
        execute_assignment_command(placement, lifecycle, current, command, authority).map_err(super::problem)?;
    let AssignmentExecutionOutcome::Uncertain(uncertain) = &outcome else {
        return Ok(outcome);
    };
    let (AssignmentExecutionPhase::RoleEffect, false, Some(intent_ref)) =
        (&uncertain.phase, uncertain.effect_may_have_happened, uncertain.intent_ref.as_deref())
    else {
        return Ok(outcome);
    };
    placement
        .abort_pre_effect_intent(&current.assignment_id, intent_ref, lifecycle)
        .map_err(|error| super::invalid(format!("placement refusal left an unresolved intent: {error}")))?;
    Err(super::invalid(format!(
        "placement role effect refused before any native effect; pending intent aborted: {}",
        uncertain.message
    )))
}

fn validated_effect_port(
    session: &super::Session,
    current: &CurrentAssignment,
    native: &crate::system_extension::NativeInstanceRecord,
) -> crate::error::Result<String> {
    let manifest = session.owner.actor_manifest(&session.member())?;
    let binding = manifest
        .binding_for(&crate::fabric::FabricPortKey {
            port_id: FABRIC_ASSIGNMENT_PORT_ID.to_string(),
            version: FABRIC_MEMBERSHIP_PORT_VERSION.to_string(),
        })
        .ok_or_else(|| super::invalid("actor has no admitted membership assignment binding"))?;
    let descriptor = crate::fabric_membership::port_descriptors(&session.membership)
        .into_iter()
        .find(|descriptor| descriptor.port_id == FABRIC_ASSIGNMENT_PORT_ID)
        .ok_or_else(|| super::invalid("membership profile has no assignment descriptor"))?;
    let (descriptor_ref, _) = crate::fabric::canonical_port_descriptor(&descriptor)?;
    if binding.descriptor_ref != descriptor_ref
        || session.membership.profile.authority_scope != session.configuration.node_id
        || current.assignment.node_id != session.configuration.node_id
    {
        return Err(super::invalid(
            "placement substituted exact admitted membership profile, descriptor or node scope",
        ));
    }
    if binding.binding.implementation_profile != session.membership.profile.profile_id
        || current.assignment.service_id != session.member()
        || current.assignment.extension_id != native.extension_id
        || current.assignment.service_generation != native.lifecycle.generation.get()
        || !native.unresolved.is_empty()
    {
        return Err(super::invalid("placement native identity, profile or generation fence differs"));
    }
    Ok(binding.binding_ref.clone())
}

fn step_result(
    session: &super::Session,
    outcome: AssignmentExecutionOutcome,
) -> crate::error::Result<serde_json::Value> {
    match outcome {
        AssignmentExecutionOutcome::Committed(receipt) => {
            session.retain(&receipt.value)?;
            let observed = session
                .placement
                .current(&session.configuration.assignment_id)
                .map_err(super::problem)?
                .ok_or_else(|| super::invalid("committed placement disappeared"))?;
            Ok(
                serde_json::json!({"assignment_ref": observed.assignment_ref, "state": observed.assignment.state.as_str(),
                "receipt_ref": receipt.transition_ref, "receipt": crate::preserves_rail::to_text(&receipt.value)?, "semantic_outcome": "progress"}),
            )
        }
        AssignmentExecutionOutcome::Uncertain(uncertain) => {
            Err(super::invalid(format!("placement outcome unknown; never replay role effect: {uncertain:?}")))
        }
    }
}

fn authorize(
    session: &mut super::Session,
    operation: &str,
    subject: &str,
    state_ref: &str,
    input_ref: &str,
) -> crate::error::Result<()> {
    let request = crate::node_service_hosting::OperationRequest {
        operation: operation.to_string(),
        subject: subject.to_string(),
        logical_tick: session.tick,
        state_ref: state_ref.to_string(),
        input_ref: input_ref.to_string(),
    };
    session.signing.prepare(&request)?;
    let mut current = crate::node_service_host::authority::Current::open(
        &session.root,
        &session.bundle_ref,
        crate::world_witness::live_clock()?,
    )?;
    current.admit(&request)?;
    Ok(())
}

/// Native activation behind the placement lifecycle. `prepare` (preview and signing) executes
/// nothing; `rewake` is the role effect.
trait NativeActivation {
    fn prepare(&mut self, service_id: &str, tick: u64) -> crate::error::Result<()>;
    fn rewake(
        &mut self,
        service_id: &str,
        tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord>;
}

struct Native<'a> {
    owner: &'a mut crate::node_service_host::installed::Owner,
    signing: &'a mut dyn SigningPort,
}

impl NativeActivation for Native<'_> {
    fn prepare(&mut self, service_id: &str, tick: u64) -> crate::error::Result<()> {
        let preview = self
            .owner
            .actor_preview(crate::node_service_hosting::native::actor::Input::Rewake(service_id), tick)?;
        self.signing.prepare(&preview)
    }

    fn rewake(
        &mut self,
        service_id: &str,
        tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        self.owner.actor_rewake(service_id, tick)
    }
}

/// Placement role lifecycle. It records every activation it refused before any native effect,
/// keyed by assignment and command transition ref, and reports exactly those as `NotExecuted`.
struct Lifecycle<'a> {
    native: &'a mut dyn NativeActivation,
    tick: u64,
    refusals: Vec<(String, String, String)>,
}

impl<'a> Lifecycle<'a> {
    fn new(native: &'a mut dyn NativeActivation, tick: u64) -> Self {
        Self {
            native,
            tick,
            refusals: Vec::new(),
        }
    }

    fn refuse(&mut self, assignment: &RoleAssignment, error: impl std::fmt::Debug) -> RoleEffectFailure {
        let failure = role_failure(error, false);
        let transition_ref = assignment.transition_refs.last().cloned().unwrap_or_default();
        let refusal_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "actor-placement-pre-effect-refusal-v1",
            vec![
                crate::preserves_rail::string(&assignment.assignment_id),
                crate::preserves_rail::string(&transition_ref),
                crate::preserves_rail::string(&failure.message),
            ],
        ));
        match refusal_ref {
            Ok(refusal_ref) => {
                self.refusals.push((assignment.assignment_id.clone(), transition_ref, refusal_ref));
                failure
            }
            // Without a refusal identity the port cannot later prove non-execution.
            Err(error) => role_failure(error, true),
        }
    }
}

impl ExtensionRoleLifecyclePort for Lifecycle<'_> {
    fn activate(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        if let Err(error) = self.native.prepare(&assignment.service_id, self.tick) {
            return Err(self.refuse(assignment, error));
        }
        let observed =
            self.native.rewake(&assignment.service_id, self.tick).map_err(|error| role_failure(error, true))?;
        if observed.lifecycle.phase != crate::system_extension::LifecyclePhase::Running
            || observed.lifecycle.generation.get() != assignment.service_generation
        {
            return Err(role_failure("native role activation did not reach exact Running generation", true));
        }
        crate::system_extension::canonical_instance_record(&observed)
            .map(|receipt| receipt.record_ref)
            .map_err(|error| role_failure(error, true))
    }
    fn begin_drain(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        Err(self.refuse(assignment, "drain is outside placement initialization"))
    }
    fn begin_replacement(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        Err(self.refuse(assignment, "replacement requires its existing owner protocol"))
    }
    fn release(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        Err(self.refuse(assignment, "release requires its existing owner protocol"))
    }
    fn fail(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        Err(self.refuse(assignment, "failure requires its existing owner protocol"))
    }
    fn quarantine(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        Err(self.refuse(assignment, "quarantine requires its existing owner protocol"))
    }
    fn observe_intent(&mut self, transition: &AssignmentTransition, intent_ref: &str) -> RoleEffectObservation {
        let Some(transition_ref) = transition.next.transition_refs.last() else {
            return RoleEffectObservation::Unknown;
        };
        self.refusals
            .iter()
            .find(|(assignment_id, refused, _)| {
                *assignment_id == transition.next.assignment_id && refused == transition_ref
            })
            .map_or(RoleEffectObservation::Unknown, |(assignment_id, refused, refusal_ref)| {
                RoleEffectObservation::NotExecuted(RoleIntentEvidence {
                    assignment_id: assignment_id.clone(),
                    transition_ref: refused.clone(),
                    kind: transition.kind,
                    intent_ref: intent_ref.to_string(),
                    evidence_ref: refusal_ref.clone(),
                })
            })
    }
}
fn role_failure(error: impl std::fmt::Debug, effect_may_have_happened: bool) -> RoleEffectFailure {
    RoleEffectFailure {
        message: format!("actual native placement role: {error:?}"),
        effect_may_have_happened,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    /// Native activation whose preparation can be refused and whose rewake (the role effect)
    /// always loses its outcome.
    #[derive(Default)]
    struct ScriptedNative {
        refuse_prepare: bool,
        rewakes: usize,
    }

    impl NativeActivation for ScriptedNative {
        fn prepare(&mut self, _service_id: &str, _tick: u64) -> crate::error::Result<()> {
            if self.refuse_prepare {
                return Err(super::super::invalid("injected signing.prepare refusal"));
            }
            Ok(())
        }

        fn rewake(
            &mut self,
            _service_id: &str,
            _tick: u64,
        ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
            self.rewakes += 1;
            Err(super::super::invalid("injected lost rewake outcome"))
        }
    }

    fn proposal() -> AssignmentProposal {
        AssignmentProposal {
            assignment_id: "assignment-a".to_string(),
            extension_id: "molten.addressable-counter".to_string(),
            service_id: "counter-a".to_string(),
            role_id: "counter".to_string(),
            role_kind: "actor".to_string(),
            node_id: "node-a".to_string(),
            service_generation: 1,
            assignment_epoch: 1,
            fencing_token: 1,
            fencing_profile_ref: reference("fencing-profile"),
            resource_reservation_ref: reference("reservation"),
            placement_plan_ref: reference("plan"),
            authority_ref: reference("authority"),
            durable_state_ref: None,
            predecessor_assignment_ref: None,
            predecessor_epoch: None,
        }
    }

    fn authority(assignment: &RoleAssignment) -> AssignmentAuthoritySnapshot {
        AssignmentAuthoritySnapshot {
            fencing_profile: FencingProfile {
                schema: FENCING_PROFILE_SCHEMA.to_string(),
                profile_id: "counter-node-local-fencing-v1".to_string(),
                profile_ref: assignment.fencing_profile_ref.clone(),
                authority_ref: assignment.authority_ref.clone(),
                effect_port_ref: reference("effect-port"),
                enforcement: FencingEnforcement::NodeLocalDurable,
            },
            enforced_assignment_epoch: assignment.assignment_epoch,
            enforced_fencing_token: assignment.fencing_token,
            required_enforcement: FencingEnforcement::NodeLocalDurable,
        }
    }

    fn command(assignment: &RoleAssignment, kind: AssignmentCommandKind, label: &str) -> AssignmentCommand {
        AssignmentCommand {
            kind,
            assignment_id: assignment.assignment_id.clone(),
            service_generation: assignment.service_generation,
            assignment_epoch: assignment.assignment_epoch,
            fencing_token: assignment.fencing_token,
            transition_ref: reference(label),
            uncertain_old_owner: false,
        }
    }

    fn step(
        placement: &mut dyn AssignmentPersistence,
        native: &mut ScriptedNative,
        kind: AssignmentCommandKind,
        label: &str,
    ) -> crate::error::Result<AssignmentExecutionOutcome> {
        let current = placement.current("assignment-a").expect("current read").expect("initialized").assignment;
        let mut lifecycle = Lifecycle::new(native, 1);
        execute_and_abort_pre_effect(
            placement,
            &mut lifecycle,
            &current,
            &command(&current, kind, label),
            &authority(&current),
        )
    }

    // r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
    #[test]
    fn refused_activation_aborts_its_intent_and_the_next_step_reads_current() {
        let mut placement = crate::fabric_membership::persistence::InMemory::default();
        placement.initialize(&proposal()).expect("initialize");
        let mut native = ScriptedNative::default();
        for (kind, label) in [
            (AssignmentCommandKind::Reserve, "reserve"),
            (AssignmentCommandKind::Assign, "assign"),
            (AssignmentCommandKind::Acknowledge, "acknowledge"),
        ] {
            let outcome = step(&mut placement, &mut native, kind, label).expect("effectless step");
            assert!(matches!(outcome, AssignmentExecutionOutcome::Committed(_)));
        }
        let acknowledged = placement.current("assignment-a").expect("current").expect("acknowledged");

        native.refuse_prepare = true;
        let refused = step(&mut placement, &mut native, AssignmentCommandKind::Activate, "activate-refused")
            .expect_err("refusal is reported");
        assert!(refused.to_string().contains("pending intent aborted"), "{refused}");
        assert_eq!(native.rewakes, 0, "no native effect ran");
        assert_eq!(placement.inspect_pending("assignment-a").expect("inspect"), None);
        assert_eq!(placement.current("assignment-a").expect("next step reads current"), Some(acknowledged));

        // A rewake whose outcome is lost is never aborted: the intent stays pending and uncertain.
        native.refuse_prepare = false;
        let outcome = step(&mut placement, &mut native, AssignmentCommandKind::Activate, "activate-lost")
            .expect("uncertain outcome is returned");
        let AssignmentExecutionOutcome::Uncertain(uncertain) = outcome else {
            panic!("lost rewake is uncertain")
        };
        assert!(uncertain.effect_may_have_happened);
        assert_eq!(native.rewakes, 1);
        assert!(placement.current("assignment-a").is_err());
        let pending = placement.inspect_pending("assignment-a").expect("inspect").expect("pending");
        let mut other = ScriptedNative::default();
        let mut fresh = Lifecycle::new(&mut other, 1);
        assert!(matches!(
            fresh.observe_intent(&pending.transition, &pending.intent_ref),
            RoleEffectObservation::Unknown
        ));
        assert!(placement.abort_pre_effect_intent("assignment-a", &pending.intent_ref, &mut fresh).is_err());
    }

    #[test]
    fn lifecycle_reports_refusals_only_for_the_exact_refused_transition() {
        let mut native = ScriptedNative {
            refuse_prepare: true,
            rewakes: 0,
        };
        let mut lifecycle = Lifecycle::new(&mut native, 1);
        let assignment = propose_assignment(&proposal()).expect("proposal");
        let refused = apply_assignment_command(&assignment, &command(&assignment, AssignmentCommandKind::Reserve, "x"))
            .expect("reserve");
        let mut activating = refused.clone();
        activating.kind = AssignmentCommandKind::Activate;
        assert!(lifecycle.activate(&activating.next).is_err());
        let intent_ref = reference("intent");
        let RoleEffectObservation::NotExecuted(evidence) = lifecycle.observe_intent(&activating, &intent_ref) else {
            panic!("refused transition is reported as not executed")
        };
        assert_eq!(evidence.intent_ref, intent_ref);
        assert_eq!(evidence.transition_ref, reference("x"));
        let mut other = activating.clone();
        other.next.transition_refs = vec![reference("other")];
        assert!(matches!(lifecycle.observe_intent(&other, &intent_ref), RoleEffectObservation::Unknown));
    }
}
