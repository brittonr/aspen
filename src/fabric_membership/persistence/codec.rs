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

/// The port's evidence names exactly this intent: assignment, command transition, kind and ref.
fn evidence_binds(
    evidence: &crate::fabric_membership::RoleIntentEvidence,
    transition: &crate::fabric_membership::AssignmentTransition,
    intent_ref: &str,
) -> bool {
    evidence.assignment_id == transition.next.assignment_id
        && transition.next.transition_refs.last() == Some(&evidence.transition_ref)
        && evidence.kind == transition.kind
        && evidence.intent_ref == intent_ref
        && valid_ref(&evidence.evidence_ref)
}

/// Asks the quarantine authority to authorize one exact recovery of `committed` and verifies the
/// decision echoes that request.
fn authorize(
    authority: &mut dyn crate::fabric_membership::QuarantineAuthorityPort,
    committed: &crate::fabric_membership::RoleAssignment,
    basis: crate::fabric_membership::QuarantineReconciliationBasis,
) -> crate::fabric::FabricPortResult<String> {
    let request = crate::fabric_membership::QuarantineAuthorizationRequest {
        assignment_id: committed.assignment_id.clone(),
        committed_assignment_ref: crate::preserves_rail::canonical_hash(&assignment_value(committed))
            .map_err(crate::fabric::FabricPortError::from)?,
        basis,
    };
    let authorization = authority.authorize_quarantine(&request)?;
    if authorization.request != request || !valid_ref(&authorization.authorization_ref) {
        return Err(crate::fabric::FabricPortError::malformed(
            "quarantine authorization does not bind the exact recovery request",
        ));
    }
    Ok(authorization.authorization_ref)
}

/// The quarantine transition ref is derived from the intent it resolves, never caller-chosen.
fn quarantine_transition_ref(intent_ref: &str) -> crate::fabric::FabricPortResult<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "fabric-assignment-quarantine-reconcile-transition-v1",
        vec![crate::preserves_rail::string(intent_ref)],
    ))
    .map_err(crate::fabric::FabricPortError::from)
}

const QUARANTINE_RECONCILIATION_RECORD: &str = "fabric-assignment-quarantine-reconciliation-v1";

fn quarantine_reconciliation(
    quarantined: &crate::fabric_membership::RoleAssignment,
    basis: crate::fabric_membership::QuarantineReconciliationBasis,
    evidence_ref: &str,
) -> crate::fabric::FabricPortResult<(crate::fabric_membership::QuarantineReconciliation, Vec<u8>)> {
    let quarantined_assignment_ref = crate::preserves_rail::canonical_hash(&assignment_value(quarantined))
        .map_err(crate::fabric::FabricPortError::from)?;
    reconciliation_from_fields(&quarantined.assignment_id, &quarantined_assignment_ref, basis, evidence_ref)
}

fn reconciliation_from_fields(
    assignment_id: &str,
    quarantined_assignment_ref: &str,
    basis: crate::fabric_membership::QuarantineReconciliationBasis,
    evidence_ref: &str,
) -> crate::fabric::FabricPortResult<(crate::fabric_membership::QuarantineReconciliation, Vec<u8>)> {
    let value = crate::preserves_rail::record(QUARANTINE_RECONCILIATION_RECORD, vec![
        crate::preserves_rail::string(assignment_id),
        crate::preserves_rail::string(quarantined_assignment_ref),
        crate::preserves_rail::string(basis.label()),
        crate::preserves_rail::string(basis.bound_ref()),
        crate::preserves_rail::string(evidence_ref),
    ]);
    let reconcile_ref = crate::preserves_rail::canonical_hash(&value).map_err(crate::fabric::FabricPortError::from)?;
    let bytes = crate::preserves_rail::canonical_bytes(&value).map_err(crate::fabric::FabricPortError::from)?;
    Ok((
        crate::fabric_membership::QuarantineReconciliation {
            assignment_id: assignment_id.to_string(),
            quarantined_assignment_ref: quarantined_assignment_ref.to_string(),
            basis,
            evidence_ref: evidence_ref.to_string(),
            reconcile_ref,
        },
        bytes,
    ))
}

fn reconciliation_basis(
    label: &str,
    bound_ref: String,
) -> crate::fabric::FabricPortResult<crate::fabric_membership::QuarantineReconciliationBasis> {
    match label {
        "committed-effect" => {
            Ok(crate::fabric_membership::QuarantineReconciliationBasis::CommittedEffect { intent_ref: bound_ref })
        }
        "reconciled-intent" => {
            Ok(crate::fabric_membership::QuarantineReconciliationBasis::ReconciledIntent { intent_ref: bound_ref })
        }
        "legacy-record" => Ok(crate::fabric_membership::QuarantineReconciliationBasis::LegacyRecord {
            transition_ref: bound_ref,
        }),
        _ => Err(invalid("unknown quarantine reconciliation basis")),
    }
}

/// Decodes a stored reconciliation record and recomputes its identity; the record must belong to
/// `assignment_id` and re-encode to exactly the stored bytes.
pub(super) fn decode_reconciliation(
    bytes: &[u8],
    assignment_id: &str,
) -> crate::fabric::FabricPortResult<crate::fabric_membership::QuarantineReconciliation> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(invalid("reconciliation record exceeds 4 MiB bound"));
    }
    let value = crate::preserves_rail::parse_canonical_bytes(bytes).map_err(crate::fabric::FabricPortError::from)?;
    let values = fields(&value, QUARANTINE_RECONCILIATION_RECORD, 5)?;
    let basis = reconciliation_basis(&text(&values[2])?, text(&values[3])?)?;
    let (record, canonical) =
        reconciliation_from_fields(&text(&values[0])?, &text(&values[1])?, basis, &text(&values[4])?)?;
    if record.assignment_id != assignment_id || canonical != bytes {
        return Err(invalid("stored quarantine reconciliation does not match its key"));
    }
    Ok(record)
}

fn validate_transition(
    current: &crate::fabric_membership::RoleAssignment,
    transition: &crate::fabric_membership::AssignmentTransition,
) -> crate::fabric::FabricPortResult<()> {
    let next = &transition.next;
    let transition_ref = next.transition_refs.last().ok_or_else(|| invalid("transition lacks command reference"))?;
    let command = crate::fabric_membership::AssignmentCommand {
        kind: transition.kind,
        assignment_id: next.assignment_id.clone(),
        service_generation: next.service_generation,
        assignment_epoch: next.assignment_epoch,
        fencing_token: next.fencing_token,
        transition_ref: transition_ref.clone(),
        uncertain_old_owner: next.uncertain_old_owner && !current.uncertain_old_owner,
    };
    let expected = crate::fabric_membership::apply_assignment_command(current, &command)
        .map_err(|issues| invalid(format!("invalid assignment transition: {issues:?}")))?;
    if &expected != transition {
        return Err(invalid("transition differs from pure assignment command"));
    }
    Ok(())
}

fn intent_reference(
    current: &crate::fabric_membership::RoleAssignment,
    transition: &crate::fabric_membership::AssignmentTransition,
) -> crate::fabric::FabricPortResult<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("fabric-assignment-owner-intent-v1", vec![
        assignment_value(current),
        crate::preserves_rail::string(transition.kind.as_str()),
        assignment_value(&transition.next),
    ]))
    .map_err(crate::fabric::FabricPortError::from)
}

pub(super) fn encode_record(state: &AssignmentRecord) -> crate::fabric::FabricPortResult<Vec<u8>> {
    encode_state(
        &state.assignment,
        state.pending.as_ref().map(|(transition, reference)| (transition, reference.as_str())),
    )
}

fn encode_state(
    assignment: &crate::fabric_membership::RoleAssignment,
    pending: Option<(&crate::fabric_membership::AssignmentTransition, &str)>,
) -> crate::fabric::FabricPortResult<Vec<u8>> {
    let pending = pending.map(|(transition, intent_ref)| {
        crate::preserves_rail::record("pending-assignment-intent-v1", vec![
            crate::preserves_rail::string(transition.kind.as_str()),
            assignment_value(&transition.next),
            crate::preserves_rail::string(intent_ref),
        ])
    });
    let value = crate::preserves_rail::record("fabric-assignment-owner-state-v1", vec![
        assignment_value(assignment),
        crate::preserves_rail::sequence(pending.into_iter().collect()),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value).map_err(crate::fabric::FabricPortError::from)?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(invalid("assignment record exceeds 4 MiB bound"));
    }
    Ok(bytes)
}

pub(super) fn decode_record(bytes: &[u8]) -> crate::fabric::FabricPortResult<AssignmentRecord> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(invalid("assignment record exceeds 4 MiB bound"));
    }
    let value = crate::preserves_rail::parse_canonical_bytes(bytes).map_err(crate::fabric::FabricPortError::from)?;
    let values = fields(&value, "fabric-assignment-owner-state-v1", 2)?;
    let assignment = decode_assignment(&values[0])?;
    let pending = items(&values[1], 1)?;
    let pending = pending
        .first()
        .map(|value| {
            let fields = fields(value, "pending-assignment-intent-v1", 3)?;
            let kind = command_kind(&text(&fields[0])?)?;
            let next = decode_assignment(&fields[1])?;
            let transition = crate::fabric_membership::AssignmentTransition {
                previous_state: assignment.state,
                next,
                kind,
            };
            validate_transition(&assignment, &transition)?;
            let intent_ref = text(&fields[2])?;
            if intent_reference(&assignment, &transition)? != intent_ref {
                return Err(invalid("stored pending intent identity mismatch"));
            }
            Ok((transition, intent_ref))
        })
        .transpose()?;
    Ok(AssignmentRecord { assignment, pending })
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_membership/persistence/parts/codec/body.rs"));
