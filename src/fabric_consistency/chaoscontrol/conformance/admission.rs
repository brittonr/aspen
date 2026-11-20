//! Pure, bounded admission of retained ChaosControl facts. This is not execution attestation.

include!("admission/schema/model.rs");

pub fn expected_json(bytes: &[u8]) -> Result<ExpectedCohort, ImportIssue> {
    let expected = bounds::decode(bytes, MAX_EXPECTED_BYTES)?;
    checks::policy(&expected)?;
    Ok(expected)
}

pub fn import_json(expected: &ExpectedCohort, bytes: &[u8]) -> Result<Imported, ImportIssue> {
    let plan = checks::policy(expected)?;
    let bundle = bounds::decode(bytes, MAX_BUNDLE_BYTES.min(plan.profile.bounds.maximum_evidence_bytes))?;
    import_admitted(expected, &bundle, plan)
}

// r[impl molten.consensus.chaoscontrol_chain_observation]
pub fn import(expected: &ExpectedCohort, bundle: &Bundle) -> Result<Imported, ImportIssue> {
    let plan = checks::policy(expected)?;
    import_admitted(expected, bundle, plan)
}

// r[impl molten.testing.chaoscontrol_smr_evidence]
fn import_admitted(
    expected: &ExpectedCohort,
    bundle: &Bundle,
    plan: chaoscontrol_smr::SmrWorkloadPlan,
) -> Result<Imported, ImportIssue> {
    bounds::encoded(bundle, MAX_BUNDLE_BYTES.min(plan.profile.bounds.maximum_evidence_bytes))?;
    checks::identity(expected, bundle)?;
    let facts = checks::facts(expected, &plan, &bundle.observations, &bundle.proposals)?;
    let verified = evidence::history(expected, bundle, &plan, &facts)?;
    checks::stabilization(expected, &facts, bundle.stabilization.as_ref())?;
    // r[impl molten.consensus.chaoscontrol_liveness]
    let liveness =
        chaoscontrol_smr::evaluate_liveness(&plan, bundle.stabilization.as_ref()).map_err(|_| ImportIssue::Liveness)?;
    if liveness != bundle.liveness {
        return Err(ImportIssue::Liveness);
    }
    let fault_refs = checks::faults(expected, &plan, bundle, &facts.records)?;
    let replay_refs = replay::validate(expected, &plan, bundle, &facts, &verified.history)?;
    let oracle = oracle::Evaluator {
        expected,
        bundle,
        history: &verified.history,
        facts: &facts,
        liveness: &liveness,
    }
    .evaluate(&verified.cohort)?;
    if oracle != bundle.oracle {
        return Err(ImportIssue::Oracle);
    }
    let scheduler_refs = facts
        .records
        .iter()
        .map(|r| r.scheduler_ref.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let receipt = chaoscontrol_smr::observation::ProtocolReceipt::build(
        &expected.protocol,
        &verified.cohort,
        &oracle,
        None,
        scheduler_refs,
        fault_refs,
        replay_refs,
    )
    .map_err(|_| ImportIssue::Receipt)?;
    if receipt != bundle.receipt {
        return Err(ImportIssue::Receipt);
    }
    evidence::Retained {
        expected,
        bundle,
        plan: &plan,
        facts: &facts,
        verified: &verified,
        liveness: &liveness,
        oracle: &oracle,
        receipt: &receipt,
    }
    .imported()
}

struct Intake<'a> {
    sources: &'a [chaoscontrol_smr::observation::guest::GuestSource],
    execution_ref: &'a str,
    records: &'a [chaoscontrol_smr::observation::ProtocolObservation],
    drains: &'a [chaoscontrol_smr::observation::Drain],
}
impl chaoscontrol_smr::observation::guest::MultiNodeGuestIntake for Intake<'_> {
    fn sources(&self) -> &[chaoscontrol_smr::observation::guest::GuestSource] {
        self.sources
    }
    fn execution_ref(&self) -> &str {
        self.execution_ref
    }
    fn observations(&self) -> &[chaoscontrol_smr::observation::ProtocolObservation] {
        self.records
    }
    fn drains(&self) -> &[chaoscontrol_smr::observation::Drain] {
        self.drains
    }
}

/// Versioned typed-JSON payload inside a canonical packed Preserves record.
/// This is a Molten identity, never a substitute for a producer-domain reference.
pub fn artifact_ref<T: serde::Serialize>(label: &'static str, value: &T) -> Result<String, ImportIssue> {
    crate::preserves_rail::canonical_hash(&typed_artifact(label, value)?).map_err(|_| ImportIssue::Bounds)
}

fn typed_artifact<T: serde::Serialize>(label: &'static str, value: &T) -> Result<preserves::IOValue, ImportIssue> {
    bounds::encoded(value, MAX_BUNDLE_BYTES)?;
    let json = serde_json::to_string(value).map_err(|_| ImportIssue::Decode)?;
    Ok(crate::preserves_rail::record(label, vec![crate::preserves_rail::string(json)]))
}

pub fn operation_ref(client_session: &str, sequence: u64) -> Result<String, ImportIssue> {
    if client_session.is_empty() || client_session.len() > 256 || sequence == 0 {
        return Err(ImportIssue::Policy);
    }
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("molten-chaoscontrol-operation-v1", vec![
        crate::preserves_rail::string(client_session),
        crate::preserves_rail::string(sequence.to_string()),
    ]))
    .map_err(|_| ImportIssue::Bounds)
}

#[path = "admission/bounds.rs"]
mod bounds;
#[path = "admission/checks.rs"]
mod checks;
#[path = "admission/evidence.rs"]
mod evidence;
#[path = "admission/oracle.rs"]
mod oracle;
#[path = "admission/replay.rs"]
mod replay;
#[cfg(test)]
#[path = "admissiontests.rs"]
mod tests;
