
fn validate_transfer(transfer: &str) -> Result<()> {
    match transfer {
        TRANSFER_LOCAL_ONLY | TRANSFER_ATTENUATED_DELEGATION | TRANSFER_REMOTE_PROXY => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported effect handle transfer policy {transfer}"))),
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value in refs {
        require_ref(value, field)?;
    }
    Ok(())
}

fn validate_unique_refs(refs: &[String], field: &str) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for value in refs {
        require_ref(value, field)?;
        if !seen.insert(value.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate {field} {value}")));
        }
    }
    Ok(())
}

fn validate_operation_subset(parent: &[String], child: &[String]) -> Result<()> {
    validate_operations(child)?;
    for operation in child {
        if !parent.iter().any(|candidate| candidate == operation) {
            return Err(MoltenError::invalid_harness(format!(
                "attenuated effect handle operation {operation} is not in parent operation set"
            )));
        }
    }
    Ok(())
}

fn validate_scope_narrows(parent: &EffectScope, child: &EffectScope) -> Result<()> {
    validate_scope(child)?;
    if parent.run_ref != child.run_ref || parent.session_ref != child.session_ref {
        return Err(MoltenError::invalid_harness("attenuated effect handle cannot widen run/session scope"));
    }
    if let Some(parent_actor) = parent.actor_ref.as_deref()
        && child.actor_ref.as_deref() != Some(parent_actor)
    {
        return Err(MoltenError::invalid_harness("attenuated effect handle cannot escape parent actor scope"));
    }
    if let Some(parent_turn) = parent.turn_ref.as_deref()
        && child.turn_ref.as_deref() != Some(parent_turn)
    {
        return Err(MoltenError::invalid_harness("attenuated effect handle cannot escape parent turn scope"));
    }
    Ok(())
}
