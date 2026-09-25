
fn revocation_hits_context(revocation: &Revocation, context: &Context, logical_time: u64) -> bool {
    revocation.effective_at <= logical_time
        && (revocation.target_ref == context.context_ref
            || revocation.target_ref == context.subject_ref
            || context.delegation_refs.iter().any(|delegation| delegation == &revocation.target_ref)
            || context.key_refs.iter().any(|key| key == &revocation.target_ref)
            || context.capabilities.iter().any(|capability| {
                capability_ref(&context.subject_ref, capability)
                    .is_ok_and(|capability_ref| capability_ref == revocation.target_ref)
            }))
}

fn capability_allows_current_action(
    capability: &Capability,
    requested_capability: &str,
    requested_operation: &str,
    requested_scope: &str,
) -> bool {
    capability_name_matches(&capability.capability, requested_capability, requested_operation)
        && (capability.scope == requested_scope || capability.scope == "*")
        && attenuation_allows(&capability.attenuation)
}

fn capability_name_matches(capability: &str, requested_capability: &str, requested_operation: &str) -> bool {
    let operation_capability = format!("{requested_capability}:{requested_operation}");
    capability == "*"
        || capability == requested_capability
        || capability == requested_operation
        || capability == operation_capability
}

fn attenuation_allows(attenuation: &str) -> bool {
    matches!(attenuation, "scoped" | "unattenuated" | "*")
}

fn capability_ref(subject_ref: &str, capability: &Capability) -> Result<String> {
    canonical_hash(&record("authority-capability-ref", vec![string(subject_ref), capability_value(capability)]))
}

fn validate_identity_type(identity_type: &str) -> Result<()> {
    match identity_type {
        "principal" | "node" | "actor" | "service" | "session" | "artifact" | "execution" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported authority identity type {other}"))),
    }
}

fn validate_revocation_target(target_kind: &str) -> Result<()> {
    match target_kind {
        "key" | "principal" | "delegation" | "capability" | "live-ref" | "handler-binding" | "session" | "artifact"
        | "authority-context" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported authority revocation target {other}"))),
    }
}

fn validate_capability(capability: &Capability) -> Result<()> {
    validate_non_empty(&capability.capability, "authority capability")?;
    validate_non_empty(&capability.scope, "authority capability scope")?;
    validate_non_empty(&capability.attenuation, "authority capability attenuation")
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}
