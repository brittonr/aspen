//! Explicit host-owned facts. Guest references never populate current admission.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub run_ref: String,
    pub handler_profile_ref: String,
    pub turn_ref: String,
    pub boundary_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_calls: u64,
    pub max_output_bytes: u64,
}

pub struct CurrentInput<'a> {
    pub generation: u64,
    pub incarnation: u64,
    pub effect_manifest: &'a preserves::IOValue,
    pub handler_profile: &'a preserves::IOValue,
    pub grant: &'a super::super::super::admission::ImportGrant,
    pub capability_refs: &'a [String],
    pub limits: Limits,
    pub authority: super::authority::Input<'a>,
}

pub(super) fn scope_value(scope: &Scope) -> preserves::IOValue {
    crate::preserves_rail::record("component-host-scope-v1", vec![
        crate::preserves_rail::string(&scope.run_ref),
        crate::preserves_rail::string(&scope.handler_profile_ref),
        crate::preserves_rail::string(&scope.turn_ref),
        crate::preserves_rail::string(&scope.boundary_ref),
    ])
}

pub(super) fn validate_scope(scope: &Scope) -> super::Result<()> {
    for reference in [
        &scope.run_ref,
        &scope.handler_profile_ref,
        &scope.turn_ref,
        &scope.boundary_ref,
    ] {
        crate::preserves_rail::validate_content_ref(reference).map_err(super::super::convert)?;
    }
    Ok(())
}

pub fn limits_ref(limits: Limits) -> super::Result<String> {
    if limits.max_calls == 0
        || limits.max_calls > 1_024
        || limits.max_output_bytes == 0
        || limits.max_output_bytes > super::super::HOST_PAYLOAD_BYTES
    {
        return Err(super::super::invalid("component host limits are outside the finite cohort"));
    }
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("component-host-limits-v1", vec![
        crate::preserves_rail::u64_value(limits.max_calls),
        crate::preserves_rail::u64_value(limits.max_output_bytes),
    ]))
    .map_err(super::super::convert)
}

pub fn recording_ref(scope: &Scope, generation: u64, incarnation: u64) -> super::Result<String> {
    validate_scope(scope)?;
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("component-host-recording-v1", vec![
        scope_value(scope),
        crate::preserves_rail::u64_value(generation),
        crate::preserves_rail::u64_value(incarnation),
    ]))
    .map_err(super::super::convert)
}

pub(super) fn admit(
    request: &super::super::HostRequest,
    scope: &Scope,
    current: &CurrentInput<'_>,
) -> super::Result<()> {
    if request.generation() != current.generation || request.incarnation() != current.incarnation {
        return Err(super::super::invalid("stale component host generation or incarnation"));
    }
    if request.grant() != current.grant
        || current.grant.policy_ref != super::authority::policy_ref(current.authority.policy_json)?
        || current.grant.authority_ref
            != super::authority::credential_ref(
                current.authority.credential.compact_token,
                current.authority.trusted_issuer,
                current.authority.expected_holder,
            )?
        || current.grant.resource_ref != limits_ref(current.limits)?
        || current.grant.recorded_effect_ref != recording_ref(scope, current.generation, current.incarnation)?
    {
        return Err(super::super::invalid("component host grant differs from current measured inputs"));
    }
    let profile = crate::effects::parse_handler_profile(current.handler_profile).map_err(super::super::convert)?;
    let manifest = crate::effects::parse_effect_manifest(current.effect_manifest).map_err(super::super::convert)?;
    if profile.profile_ref != scope.handler_profile_ref
        || profile.policy_ref != current.grant.policy_ref
        || profile.capability_context_ref != current.grant.authority_ref
        || !profile.resource_refs.contains(&current.grant.resource_ref)
        || !manifest.policy_refs.contains(&current.grant.policy_ref)
        || !request.effect().capability_refs.iter().all(|reference| current.capability_refs.contains(reference))
    {
        return Err(super::super::invalid(
            "component host artifact, handler, policy, capabilities or resources are stale",
        ));
    }
    let binding = crate::effects::admit_effect_request(
        current.effect_manifest,
        current.handler_profile,
        &request.effect().value,
        &[],
    )
    .map_err(super::super::convert)?;
    if binding.decision != "pass" {
        return Err(super::super::invalid("component host request is not admitted by the installed effect manifest"));
    }
    Ok(())
}
