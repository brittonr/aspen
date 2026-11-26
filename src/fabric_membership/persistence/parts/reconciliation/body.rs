// Quarantine reconciliation records and the evidence and authorization they bind.

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
