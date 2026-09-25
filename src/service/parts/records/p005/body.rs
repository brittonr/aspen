
fn validate_manifest_input(input: &ServiceManifestInput) -> Result<()> {
    validate_service_id(&input.service_id, "service manifest service id")?;
    require_ref(&input.owner_authority_ref, "service manifest owner authority ref")?;
    require_ref(&input.target_ref, "service manifest target ref")?;
    validate_service_ids(&input.dependencies, "service dependency")?;
    validate_refs(&input.provided_assertion_refs, "provided assertion ref")?;
    require_ref(&input.restart_policy_ref, "service restart policy ref")?;
    validate_refs(&input.policy_refs, "service policy ref")?;
    validate_refs(&input.resource_refs, "service resource ref")?;
    validate_refs(&input.effect_profile_refs, "service effect profile ref")?;
    require_non_empty_refs(&input.policy_refs, "service policy refs")?;
    require_non_empty_refs(&input.resource_refs, "service resource refs")?;
    require_non_empty_refs(&input.effect_profile_refs, "service effect profile refs")
}
