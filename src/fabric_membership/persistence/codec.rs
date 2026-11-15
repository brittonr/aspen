//! Versioned canonical Preserves storage projection for the existing assignment owner.

type Value = preserves::IOValue;
const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;
const ASSIGNMENT_RECORD: &str = "fabric-role-assignment-state-v1";

#[derive(Debug, Clone)]
pub(super) struct AssignmentRecord {
    pub assignment: crate::fabric_membership::RoleAssignment,
    pending: Option<(crate::fabric_membership::AssignmentTransition, String)>,
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
    ) -> crate::fabric::FabricPortResult<(String, Vec<u8>)> {
        let Some((pending, expected_intent)) = &self.pending else {
            return Err(crate::fabric::FabricPortError::uncertain("commit has no pending intent"));
        };
        if pending != transition || expected_intent != intent_ref {
            return Err(crate::fabric::FabricPortError::uncertain(
                "commit does not match exact pending assignment intent",
            ));
        }
        validate_transition(&self.assignment, transition)?;
        let needs_effect = !matches!(
            transition.kind,
            crate::fabric_membership::AssignmentCommandKind::Reserve
                | crate::fabric_membership::AssignmentCommandKind::Assign
                | crate::fabric_membership::AssignmentCommandKind::Acknowledge
        );
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
        self.assignment = transition.next.clone();
        self.pending = None;
        Ok((commit_ref, bytes))
    }
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
