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

fn admit_initialization(predecessor: Option<&crate::fabric_membership::CurrentAssignment>, assignment: &crate::fabric_membership::RoleAssignment) -> crate::fabric::FabricPortResult<()> {
    if let Some(predecessor) = predecessor {
        let issues = crate::fabric_membership::validate_assignment_successor(&predecessor.assignment, &predecessor.assignment_ref, assignment);
        if !issues.is_empty() {
            return Err(crate::fabric::FabricPortError::storage(format!("assignment successor denied: {issues:?}")));
        }
    } else if assignment.predecessor_assignment_ref.is_some() || assignment.predecessor_epoch.is_some() {
        return Err(crate::fabric::FabricPortError::storage("initial role assignment cannot name an absent predecessor"));
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
