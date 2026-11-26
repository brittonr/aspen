// Recording test doubles for the role lifecycle, fencing and quarantine authority ports.

/// Test lifecycle port that keeps its own intent-bound effect record: refused and executed effects
/// are keyed by (assignment ID, command transition ref), as a real port's durable log would be.
#[derive(Debug, Default)]
struct RecordingLifecycle {
    calls: Vec<String>,
    fail_activate: bool,
    refuse_activate: bool,
    lost_activate_ack: bool,
    malformed_activate_ref: bool,
    refused: Vec<(String, String)>,
    executed: Vec<(String, String, String)>,
    scripted_observation: Option<RoleEffectObservation>,
}

fn effect_key(assignment: &RoleAssignment) -> (String, String) {
    (assignment.assignment_id.clone(), assignment.transition_refs.last().cloned().unwrap_or_default())
}

impl RecordingLifecycle {
    fn effect(&mut self, kind: &str, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.calls.push(format!("{kind}:{}", assignment.assignment_id));
        let effect_ref = test_ref(&format!("role-effect-{}", self.calls.len()));
        let (assignment_id, transition_ref) = effect_key(assignment);
        self.executed.push((assignment_id, transition_ref, effect_ref.clone()));
        Ok(effect_ref)
    }
}

impl ExtensionRoleLifecyclePort for RecordingLifecycle {
    fn activate(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        if self.refuse_activate {
            self.refused.push(effect_key(assignment));
            return Err(RoleEffectFailure {
                message: "injected activation refusal before any effect".to_string(),
                effect_may_have_happened: false,
            });
        }
        if self.fail_activate {
            self.calls.push(format!("activate:{}", assignment.assignment_id));
            return Err(RoleEffectFailure {
                message: "injected activation uncertainty".to_string(),
                effect_may_have_happened: true,
            });
        }
        if self.lost_activate_ack {
            self.effect("activate", assignment)?;
            return Err(RoleEffectFailure {
                message: "injected lost activation acknowledgement".to_string(),
                effect_may_have_happened: true,
            });
        }
        if self.malformed_activate_ref {
            self.calls.push(format!("activate:{}", assignment.assignment_id));
            return Ok("malformed-effect-ref".to_string());
        }
        self.effect("activate", assignment)
    }

    fn begin_drain(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.effect("drain", assignment)
    }

    fn begin_replacement(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.effect("replace", assignment)
    }

    fn release(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.effect("release", assignment)
    }

    fn fail(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.effect("fail", assignment)
    }

    fn quarantine(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.effect("quarantine", assignment)
    }

    fn observe_intent(&mut self, transition: &AssignmentTransition, intent_ref: &str) -> RoleEffectObservation {
        if let Some(observation) = &self.scripted_observation {
            return observation.clone();
        }
        let (assignment_id, transition_ref) = effect_key(&transition.next);
        let evidence = |evidence_ref: String| RoleIntentEvidence {
            assignment_id: assignment_id.clone(),
            transition_ref: transition_ref.clone(),
            kind: transition.kind,
            intent_ref: intent_ref.to_string(),
            evidence_ref,
        };
        if let Some((_, _, effect_ref)) =
            self.executed.iter().find(|(id, reference, _)| *id == assignment_id && *reference == transition_ref)
        {
            return RoleEffectObservation::Executed(evidence(effect_ref.clone()));
        }
        if self.refused.contains(&(assignment_id.clone(), transition_ref.clone())) {
            return RoleEffectObservation::NotExecuted(evidence(test_ref(&format!("refusal-{transition_ref}"))));
        }
        RoleEffectObservation::Unknown
    }
}

/// Test fencing authority: reports the fence it currently enforces, or is unavailable.
struct ObservedFence(Option<FenceObservation>);

impl FencingAuthorityPort for ObservedFence {
    fn observe_fence(&mut self, _successor: &RoleAssignment) -> crate::fabric::FabricPortResult<FenceObservation> {
        self.0.clone().ok_or_else(|| crate::fabric::FabricPortError::capability("fencing authority unavailable"))
    }
}

fn enforced_fence(assignment: &RoleAssignment, enforcement: FencingEnforcement) -> FenceObservation {
    let mut authority = authority_snapshot(assignment);
    authority.fencing_profile.enforcement = enforcement;
    FenceObservation {
        fencing_profile: authority.fencing_profile,
        enforced_assignment_epoch: authority.enforced_assignment_epoch,
        enforced_fencing_token: authority.enforced_fencing_token,
    }
}

/// Test quarantine authority: records every request, and authorizes it by echoing it exactly
/// unless told to deny or to answer a different request.
#[derive(Debug, Default)]
struct RecordingQuarantineAuthority {
    deny: bool,
    answer_other_request: bool,
    requests: Vec<QuarantineAuthorizationRequest>,
}

impl QuarantineAuthorityPort for RecordingQuarantineAuthority {
    fn authorize_quarantine(
        &mut self,
        request: &QuarantineAuthorizationRequest,
    ) -> crate::fabric::FabricPortResult<QuarantineAuthorization> {
        self.requests.push(request.clone());
        if self.deny {
            return Err(crate::fabric::FabricPortError::capability("quarantine recovery not authorized"));
        }
        let mut answered = request.clone();
        if self.answer_other_request {
            answered.committed_assignment_ref = test_ref("other-committed-assignment");
        }
        Ok(QuarantineAuthorization {
            request: answered,
            authorization_ref: test_ref(&format!("authorization-{}", request.basis.bound_ref())),
        })
    }
}
