//! Native activation behind the placement role lifecycle.

use crate::addressable_actor::native::SigningPort;
use crate::fabric_membership::*;

/// Native activation behind the placement lifecycle. `prepare` (preview and signing) executes
/// nothing; `rewake` is the role effect.
pub(super) trait NativeActivation {
    fn prepare(&mut self, service_id: &str, tick: u64) -> crate::error::Result<()>;
    fn rewake(
        &mut self,
        service_id: &str,
        tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord>;
}

pub(super) struct Native<'a> {
    pub(super) owner: &'a mut crate::node_service_host::installed::Owner,
    pub(super) signing: &'a mut dyn SigningPort,
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
pub(super) struct Lifecycle<'a> {
    native: &'a mut dyn NativeActivation,
    tick: u64,
    refusals: Vec<(String, String, String)>,
}

impl<'a> Lifecycle<'a> {
    pub(super) fn new(native: &'a mut dyn NativeActivation, tick: u64) -> Self {
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
