fn role_key(assignment: &crate::fabric_membership::RoleAssignment) -> crate::fabric::FabricPortResult<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("fabric-assignment-role-slot-v1", vec![
        crate::preserves_rail::string(&assignment.extension_id),
        crate::preserves_rail::string(&assignment.service_id),
        crate::preserves_rail::string(&assignment.role_id),
    ]))
    .map_err(crate::fabric::FabricPortError::from)
}

fn require_role(owner: Option<&str>, assignment: &crate::fabric_membership::RoleAssignment) -> crate::fabric::FabricPortResult<()> {
    if owner != Some(assignment.assignment_id.as_str()) {
        return Err(crate::fabric::FabricPortError::storage("assignment is not the current owner of its role slot"));
    }
    Ok(())
}

/// A quarantine succession request together with the fence observed from the fencing authority.
struct QuarantineAdmission<'a> {
    clearance: &'a crate::fabric_membership::QuarantineClearance,
    fence: crate::fabric_membership::FenceObservation,
}

/// Observes, from the fencing authority, the fence enforced for the proposed successor.
fn observe_successor_fence(
    proposal: &crate::fabric_membership::AssignmentProposal,
    fencing: &mut dyn crate::fabric_membership::FencingAuthorityPort,
) -> crate::fabric::FabricPortResult<crate::fabric_membership::FenceObservation> {
    let successor = crate::fabric_membership::propose_assignment(proposal)
        .map_err(|issues| crate::fabric::FabricPortError::malformed(format!("invalid proposal: {issues:?}")))?;
    fencing.observe_fence(&successor)
}

fn admit_initialization(
    predecessor: Option<&crate::fabric_membership::CurrentAssignment>,
    assignment: &crate::fabric_membership::RoleAssignment,
    quarantine: Option<QuarantineAdmission<'_>>,
    reconciliation: Option<&crate::fabric_membership::QuarantineReconciliation>,
) -> crate::fabric::FabricPortResult<()> {
    let issues = match (predecessor, quarantine) {
        (Some(predecessor), None) => crate::fabric_membership::validate_assignment_successor(&predecessor.assignment, &predecessor.assignment_ref, assignment),
        (Some(predecessor), Some(quarantine)) => crate::fabric_membership::validate_quarantined_successor(
            &predecessor.assignment,
            &predecessor.assignment_ref,
            assignment,
            quarantine.clearance,
            reconciliation,
            &quarantine.fence,
        ),
        (None, Some(_)) => {
            return Err(crate::fabric::FabricPortError::storage("quarantine successor requires a quarantined role-slot owner"));
        }
        (None, None) => {
            if assignment.predecessor_assignment_ref.is_some() || assignment.predecessor_epoch.is_some() {
                return Err(crate::fabric::FabricPortError::storage("initial role assignment cannot name an absent predecessor"));
            }
            Vec::new()
        }
    };
    if !issues.is_empty() {
        return Err(crate::fabric::FabricPortError::storage(format!("assignment successor denied: {issues:?}")));
    }
    Ok(())
}

fn require_key(record: &codec::AssignmentRecord, assignment_id: &str) -> crate::fabric::FabricPortResult<()> {
    if record.assignment.assignment_id != assignment_id {
        return Err(crate::fabric::FabricPortError::malformed("stored assignment does not match its key"));
    }
    Ok(())
}

fn storage_error(error: impl std::fmt::Display) -> crate::fabric::FabricPortError {
    crate::fabric::FabricPortError::storage(format!("assignment store: {error}"))
}

fn uncertain_error(error: impl std::fmt::Display) -> crate::fabric::FabricPortError {
    crate::fabric::FabricPortError::uncertain(format!("assignment store outcome uncertain: {error}"))
}
