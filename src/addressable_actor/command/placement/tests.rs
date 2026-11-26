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

impl effects::NativeActivation for ScriptedNative {
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

fn transition_request(assignment: &RoleAssignment, kind: AssignmentCommandKind, label: &str) -> AssignmentCommand {
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
    let mut lifecycle = effects::Lifecycle::new(native, 1);
    execute_and_abort_pre_effect(
        placement,
        &mut lifecycle,
        &current,
        &transition_request(&current, kind, label),
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
    let mut fresh = effects::Lifecycle::new(&mut other, 1);
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
    let mut lifecycle = effects::Lifecycle::new(&mut native, 1);
    let assignment = propose_assignment(&proposal()).expect("proposal");
    let refused =
        apply_assignment_command(&assignment, &transition_request(&assignment, AssignmentCommandKind::Reserve, "x"))
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
