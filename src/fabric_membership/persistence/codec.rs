//! Versioned canonical Preserves storage projection for the existing assignment owner.

type Value = preserves::IOValue;
const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;
const ASSIGNMENT_RECORD: &str = "fabric-role-assignment-state-v1";

#[derive(Debug, Clone)]
pub(super) struct AssignmentRecord {
    pub assignment: crate::fabric_membership::RoleAssignment,
    pending: Option<(crate::fabric_membership::AssignmentTransition, String)>,
}

/// One owner state change: its result, the new assignment record bytes, and the quarantine
/// reconciliation record (with its bytes) when the change enters `Quarantined`.
pub(super) struct Resolution<T> {
    pub value: T,
    pub bytes: Vec<u8>,
    pub reconciliation: Option<(crate::fabric_membership::QuarantineReconciliation, Vec<u8>)>,
}

impl AssignmentRecord {
    pub fn propose(proposal: &crate::fabric_membership::AssignmentProposal) -> crate::fabric::FabricPortResult<Self> {
        let assignment = crate::fabric_membership::propose_assignment(proposal)
            .map_err(|issues| invalid(format!("invalid proposal: {issues:?}")))?;
        Ok(Self {
            assignment,
            pending: None,
        })
    }

    pub fn current(&self) -> crate::fabric::FabricPortResult<crate::fabric_membership::CurrentAssignment> {
        if self.pending.is_some() {
            return Err(crate::fabric::FabricPortError::uncertain(
                "assignment has unresolved intent; role effects must not be replayed",
            ));
        }
        Ok(crate::fabric_membership::CurrentAssignment {
            assignment_ref: crate::preserves_rail::canonical_hash(&assignment_value(&self.assignment))
                .map_err(crate::fabric::FabricPortError::from)?,
            assignment: self.assignment.clone(),
        })
    }

    pub fn prepare(
        &mut self,
        expected: &crate::fabric_membership::RoleAssignment,
        transition: &crate::fabric_membership::AssignmentTransition,
    ) -> crate::fabric::FabricPortResult<(String, Vec<u8>)> {
        if self.pending.is_some() {
            return Err(crate::fabric::FabricPortError::uncertain(
                "assignment intent already pending; reconciliation required",
            ));
        }
        if &self.assignment != expected {
            return Err(crate::fabric::FabricPortError::storage(
                "stale expected assignment; intent cannot overwrite current state",
            ));
        }
        validate_transition(expected, transition)?;
        let intent_ref = intent_reference(expected, transition)?;
        let bytes = encode_state(&self.assignment, Some((transition, &intent_ref)))?;
        self.pending = Some((transition.clone(), intent_ref.clone()));
        Ok((intent_ref, bytes))
    }

    pub fn finish(
        &mut self,
        transition: &crate::fabric_membership::AssignmentTransition,
        intent_ref: &str,
        effect_ref: Option<&str>,
    ) -> crate::fabric::FabricPortResult<Resolution<String>> {
        let Some((pending, expected_intent)) = &self.pending else {
            return Err(crate::fabric::FabricPortError::uncertain("commit has no pending intent"));
        };
        if pending != transition || expected_intent != intent_ref {
            return Err(crate::fabric::FabricPortError::uncertain(
                "commit does not match exact pending assignment intent",
            ));
        }
        validate_transition(&self.assignment, transition)?;
        let needs_effect = transition.kind.has_role_effect();
        if needs_effect != effect_ref.is_some() || effect_ref.is_some_and(|value| !valid_ref(value)) {
            return Err(crate::fabric::FabricPortError::uncertain(
                "assignment commit lacks exact role-effect evidence",
            ));
        }
        let value = crate::preserves_rail::record("fabric-assignment-owner-commit-v1", vec![
            crate::preserves_rail::string(intent_ref),
            crate::preserves_rail::sequence(effect_ref.into_iter().map(crate::preserves_rail::string).collect()),
            assignment_value(&transition.next),
        ]);
        let commit_ref = crate::preserves_rail::canonical_hash(&value).map_err(crate::fabric::FabricPortError::from)?;
        let bytes = encode_state(&transition.next, None)?;
        // A committed quarantine records its observed quarantine role effect as reconciliation.
        let reconciliation = match (transition.kind, effect_ref) {
            (crate::fabric_membership::AssignmentCommandKind::Quarantine, Some(effect_ref)) => {
                Some(quarantine_reconciliation(
                    &transition.next,
                    crate::fabric_membership::QuarantineReconciliationBasis::CommittedEffect {
                        intent_ref: intent_ref.to_string(),
                    },
                    effect_ref,
                )?)
            }
            _ => None,
        };
        self.assignment = transition.next.clone();
        self.pending = None;
        Ok(Resolution {
            value: commit_ref,
            bytes,
            reconciliation,
        })
    }

    /// The unresolved intent, if any, for recovery inspection.
    pub fn pending_intent(
        &self,
    ) -> crate::fabric::FabricPortResult<Option<crate::fabric_membership::PendingAssignmentIntent>> {
        let Some((transition, intent_ref)) = &self.pending else {
            return Ok(None);
        };
        Ok(Some(crate::fabric_membership::PendingAssignmentIntent {
            committed_ref: crate::preserves_rail::canonical_hash(&assignment_value(&self.assignment))
                .map_err(crate::fabric::FabricPortError::from)?,
            committed: self.assignment.clone(),
            transition: transition.clone(),
            intent_ref: intent_ref.clone(),
        }))
    }

    /// Clears the exact pending intent when its role effect definitely never executed: the
    /// persisted transition has no role effect, or the lifecycle port observes non-execution for
    /// this exact intent.
    pub fn abort(
        &mut self,
        intent_ref: &str,
        lifecycle: &mut dyn crate::fabric_membership::ExtensionRoleLifecyclePort,
    ) -> crate::fabric::FabricPortResult<Resolution<crate::fabric_membership::CurrentAssignment>> {
        let transition = self.exact_pending(intent_ref)?;
        if transition.kind.has_role_effect() {
            match lifecycle.observe_intent(&transition, intent_ref) {
                crate::fabric_membership::RoleEffectObservation::NotExecuted(evidence)
                    if evidence_binds(&evidence, &transition, intent_ref) => {}
                _ => {
                    return Err(crate::fabric::FabricPortError::uncertain(
                        "role lifecycle port observed no definite non-execution for this exact intent; \
                         reconcile the pending intent instead",
                    ));
                }
            }
        }
        let bytes = encode_state(&self.assignment, None)?;
        self.pending = None;
        Ok(Resolution {
            value: self.current()?,
            bytes,
            reconciliation: None,
        })
    }

    /// Resolves the exact pending intent whose role effect may have happened.
    pub fn reconcile(
        &mut self,
        intent_ref: &str,
        reconciliation: crate::fabric_membership::PendingIntentReconciliation<'_>,
    ) -> crate::fabric::FabricPortResult<Resolution<crate::fabric_membership::ReconciledAssignment>> {
        let transition = self.exact_pending(intent_ref)?;
        let resolution = match reconciliation {
            crate::fabric_membership::PendingIntentReconciliation::ConfirmObservedEffect { lifecycle } => {
                let effect_ref = match lifecycle.observe_intent(&transition, intent_ref) {
                    crate::fabric_membership::RoleEffectObservation::Executed(evidence)
                        if transition.kind.has_role_effect() && evidence_binds(&evidence, &transition, intent_ref) =>
                    {
                        evidence.evidence_ref
                    }
                    _ => {
                        return Err(crate::fabric::FabricPortError::uncertain(
                            "role lifecycle port did not observe this exact intent's effect as executed",
                        ));
                    }
                };
                self.finish(&transition, intent_ref, Some(effect_ref.as_str()))?
            }
            crate::fabric_membership::PendingIntentReconciliation::QuarantineUncertain { authority } => {
                let basis = crate::fabric_membership::QuarantineReconciliationBasis::ReconciledIntent {
                    intent_ref: intent_ref.to_string(),
                };
                let authorization_ref = authorize(authority, &self.assignment, basis.clone())?;
                let command = crate::fabric_membership::AssignmentCommand {
                    kind: crate::fabric_membership::AssignmentCommandKind::Quarantine,
                    assignment_id: self.assignment.assignment_id.clone(),
                    service_generation: self.assignment.service_generation,
                    assignment_epoch: self.assignment.assignment_epoch,
                    fencing_token: self.assignment.fencing_token,
                    transition_ref: quarantine_transition_ref(intent_ref)?,
                    uncertain_old_owner: !self.assignment.uncertain_old_owner,
                };
                let quarantine = crate::fabric_membership::apply_assignment_command(&self.assignment, &command)
                    .map_err(|issues| invalid(format!("invalid quarantine reconciliation: {issues:?}")))?;
                let record = quarantine_reconciliation(&quarantine.next, basis, &authorization_ref)?;
                let bytes = encode_state(&quarantine.next, None)?;
                self.assignment = quarantine.next;
                self.pending = None;
                Resolution {
                    value: record.0.reconcile_ref.clone(),
                    bytes,
                    reconciliation: Some(record),
                }
            }
        };
        Ok(Resolution {
            value: crate::fabric_membership::ReconciledAssignment {
                reconcile_ref: resolution.value,
                current: self.current()?,
            },
            bytes: resolution.bytes,
            reconciliation: resolution.reconciliation,
        })
    }

    /// Builds the reconciliation record for a `Quarantined` assignment written before such records
    /// existed, citing the quarantine authority's authorization of its exact quarantine transition.
    /// The caller must already know no record exists.
    pub fn legacy_reconciliation(
        &self,
        authority: &mut dyn crate::fabric_membership::QuarantineAuthorityPort,
    ) -> crate::fabric::FabricPortResult<(crate::fabric_membership::QuarantineReconciliation, Vec<u8>)> {
        if self.pending.is_some() {
            return Err(crate::fabric::FabricPortError::uncertain(
                "assignment has a pending intent; reconcile it instead of migrating",
            ));
        }
        if self.assignment.state != crate::fabric_membership::AssignmentState::Quarantined {
            return Err(crate::fabric::FabricPortError::storage("only a quarantined assignment can be migrated"));
        }
        let transition_ref = self
            .assignment
            .transition_refs
            .last()
            .ok_or_else(|| invalid("quarantined assignment lacks its quarantine transition ref"))?
            .clone();
        let basis = crate::fabric_membership::QuarantineReconciliationBasis::LegacyRecord { transition_ref };
        let authorization_ref = authorize(authority, &self.assignment, basis.clone())?;
        quarantine_reconciliation(&self.assignment, basis, &authorization_ref)
    }

    fn exact_pending(
        &self,
        intent_ref: &str,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::AssignmentTransition> {
        match &self.pending {
            Some((transition, expected)) if expected == intent_ref => Ok(transition.clone()),
            Some(_) => Err(crate::fabric::FabricPortError::storage(
                "recovery does not name the exact pending assignment intent",
            )),
            None => Err(crate::fabric::FabricPortError::storage("assignment has no pending intent to recover")),
        }
    }
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_membership/persistence/parts/codec/body.rs"));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/fabric_membership/persistence/parts/reconciliation/body.rs"
));
