include!("policy.rs");
include!("facts.rs");

fn reference(value: &str) -> Result<(), super::ImportIssue> {
    crate::preserves_rail::validate_content_ref(value).map_err(|_| super::ImportIssue::Identity)
}

pub(super) fn identity(expected: &super::ExpectedCohort, bundle: &super::Bundle) -> Result<(), super::ImportIssue> {
    if bundle.schema != super::BUNDLE_SCHEMA || bundle.producer_revision != expected.producer_revision {
        return Err(super::ImportIssue::Schema);
    }
    if bundle.evidence_class != expected.evidence_class || bundle.claim != expected.claim {
        return Err(super::ImportIssue::Claim);
    }
    let molten = &bundle.molten;
    if molten.source_ref != expected.molten_source_ref
        || molten.build_ref != expected.molten_build_ref
        || molten.dependency_closure_ref != expected.dependency_closure_ref
        || molten.adapter_ref != expected.adapter_ref
        || molten.observer_ref != expected.observer_ref
        || bundle.sources != expected.sources
    {
        return Err(super::ImportIssue::Identity);
    }
    if bundle.non_claims.len() != super::NON_CLAIMS.len()
        || bundle.non_claims.iter().map(String::as_str).collect::<std::collections::BTreeSet<_>>()
            != super::NON_CLAIMS.into_iter().collect()
    {
        return Err(super::ImportIssue::Claim);
    }
    if bundle.drains.len() > expected.protocol.participants.len() {
        return Err(super::ImportIssue::Bounds);
    }
    reference(&bundle.execution_ref)?;
    Ok(())
}

pub(super) fn stabilization(
    expected: &super::ExpectedCohort,
    facts: &Facts,
    stabilization: Option<&chaoscontrol_smr::StabilizationFacts>,
) -> Result<(), super::ImportIssue> {
    if let Some(s) = stabilization
        && (s.committed_after != facts.committed
            || s.committed_before > s.committed_after
            || s.virtual_progress_end > expected.workload.bounds.maximum_virtual_progress)
    {
        return Err(super::ImportIssue::Liveness);
    }
    Ok(())
}

pub(super) fn faults(
    expected: &super::ExpectedCohort,
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
    bundle: &super::Bundle,
    records: &[chaoscontrol_smr::observation::ProtocolObservation],
) -> Result<Vec<String>, super::ImportIssue> {
    if bundle.faults.len() != expected.faults.len()
        || u64::try_from(bundle.fault_effects.len()).map_err(|_| super::ImportIssue::Bounds)? > super::MAX_EFFECTS
    {
        return Err(super::ImportIssue::Fault);
    }
    chaoscontrol_smr::validate_fault_outcomes(&bundle.faults, &plan.profile.bounds)
        .map_err(|_| super::ImportIssue::Fault)?;
    let mut effects = std::collections::BTreeMap::new();
    for effect in &bundle.fault_effects {
        if effect.before_ref == effect.after_ref || effects.insert(effect.action_id.as_str(), effect).is_some() {
            return Err(super::ImportIssue::Fault);
        }
        reference(&effect.before_ref)?;
        reference(&effect.after_ref)?;
        if !records.iter().any(|r| {
            r.participant_ref == effect.participant_ref
                && r.source_sequence == effect.source_sequence
                && r.boundary_ref == effect.boundary_ref
                && r.scheduler_ref == effect.scheduler_ref
        }) {
            return Err(super::ImportIssue::Fault);
        }
    }
    let mut refs = Vec::with_capacity(bundle.faults.len());
    let mut used = 0;
    for outcome in &bundle.faults {
        let policy =
            expected.faults.iter().find(|p| p.action_id == outcome.action_id).ok_or(super::ImportIssue::Fault)?;
        if policy.class != outcome.class {
            return Err(super::ImportIssue::Fault);
        }
        match outcome.effect_record_ref.as_deref() {
            Some(effect_ref) => {
                let effect = effects.get(outcome.action_id.as_str()).ok_or(super::ImportIssue::Fault)?;
                if !chaoscontrol_smr::fault_supports_effect_claim(outcome)
                    || effect.class != outcome.class
                    || super::artifact_ref("molten-chaoscontrol-fault-effect-v1", effect)? != effect_ref
                {
                    return Err(super::ImportIssue::Fault);
                }
                used += 1;
            }
            None if effects.contains_key(outcome.action_id.as_str()) => return Err(super::ImportIssue::Fault),
            None => {}
        }
        refs.push(super::artifact_ref("molten-chaoscontrol-fault-outcome-v1", outcome)?);
    }
    if used != effects.len() {
        return Err(super::ImportIssue::Fault);
    }
    refs.sort();
    Ok(refs)
}
