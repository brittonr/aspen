//! Identity and plan admission for external live reliability evidence.

pub(super) fn validate_identity(expected: &super::Expected, evidence: &super::Observed) -> Result<(), super::Issue> {
    let profile = expected.admitted.profile();
    if expected.endpoints != profile.endpoints.iter().cloned().collect()
        || expected.max_events as u64 > u64::from(profile.evidence.max_events)
        || expected.max_witness_bytes as u64 > profile.evidence.max_bytes
        || expected.max_recovery_ms > profile.recovery.timeout_ms
        || expected.identities.get("generator") != Some(&profile.generator_ref)
        || expected.identities.get("adapter") != Some(&profile.adapter_ref)
        || expected.identities.get("model") != Some(&profile.model_ref)
    {
        return Err(super::Issue::Identity);
    }
    if !(1..=4096).contains(&expected.max_events)
        || !(1..=8_388_608).contains(&expected.max_witness_bytes)
        || !(1..=120_000).contains(&expected.max_recovery_ms)
        || expected.endpoints.is_empty()
        || expected.endpoints.len() > 64
        || evidence.faults.len() > 64
    {
        return Err(super::Issue::Bounds);
    }
    if expected.identities != evidence.identities
        || expected.identities.len() > 32
        || super::IDENTITIES
            .iter()
            .any(|key| !expected.identities.get(*key).is_some_and(|v| crate::live_reliability::checker::reference(v)))
        || evidence.profile_ref != expected.admitted.profile_ref()
        || expected.identities.get("profile") != Some(&evidence.profile_ref)
        || expected.identities.get("plan") != Some(&evidence.plan_ref)
    {
        return Err(super::Issue::Identity);
    }
    let plan = serde_json::to_value(&expected.plan).map_err(|_| super::Issue::Plan)?;
    if crate::live_reliability::checker::canonical_ref(&plan).map_err(|_| super::Issue::Plan)? != evidence.plan_ref {
        return Err(super::Issue::Plan);
    }
    Ok(())
}
