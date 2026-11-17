//! Producer-history validation and retained receipt encoding.

pub(super) struct Verified {
    pub source_ref: String,
    pub cohort: chaoscontrol_smr::observation::CohortResult,
    pub history: chaoscontrol_smr::HistoryReport,
}

pub(super) fn history(
    expected: &super::ExpectedCohort,
    bundle: &super::Bundle,
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
    facts: &super::checks::Facts,
) -> Result<Verified, super::ImportIssue> {
    let intake = super::Intake {
        sources: &bundle.sources,
        execution_ref: &bundle.execution_ref,
        records: &facts.records,
        drains: &bundle.drains,
    };
    let source_ref = chaoscontrol_smr::observation::guest::validate_sources(&expected.protocol, &intake)
        .map_err(|_| super::ImportIssue::Source)?;
    let cohort = chaoscontrol_smr::observation::assemble_cohort(
        &expected.protocol,
        &expected.boundary_ref,
        &facts.records,
        &bundle.drains,
    )
    .map_err(|_| super::ImportIssue::Observation)?;
    let history = chaoscontrol_smr::validate_history(plan, &facts.chains).map_err(|_| super::ImportIssue::History)?;
    if source_ref != bundle.source_ref
        || cohort != bundle.cohort
        || history != bundle.history
        || chaoscontrol_smr::history_report_ref(&history).map_err(|_| super::ImportIssue::History)?
            != bundle.history_ref
        || chaoscontrol_smr::observations_ref(&facts.chains).map_err(|_| super::ImportIssue::History)?
            != bundle.observations_ref
    {
        return Err(super::ImportIssue::History);
    }
    // The actual producer guest-history hook is used only when the cohort is complete.
    // On incomplete inputs its error is not reclassified as a consensus failure.
    if cohort.completeness == chaoscontrol_smr::observation::Completeness::Complete {
        let admitted = chaoscontrol_smr::observation::guest::validate_guest_history(
            &expected.protocol,
            &expected.boundary_ref,
            plan,
            &intake,
        )
        .map_err(|_| super::ImportIssue::History)?;
        if admitted.history != history || admitted.sources_ref != source_ref || admitted.cohort_ref != cohort.cohort_ref
        {
            return Err(super::ImportIssue::History);
        }
    }
    Ok(Verified {
        source_ref,
        cohort,
        history,
    })
}

pub(super) struct Retained<'a> {
    pub expected: &'a super::ExpectedCohort,
    pub bundle: &'a super::Bundle,
    pub plan: &'a chaoscontrol_smr::SmrWorkloadPlan,
    pub facts: &'a super::checks::Facts,
    pub verified: &'a Verified,
    pub liveness: &'a chaoscontrol_smr::LivenessReport,
    pub oracle: &'a chaoscontrol_smr::observation::OracleResult,
    pub receipt: &'a chaoscontrol_smr::observation::ProtocolReceipt,
}

impl Retained<'_> {
    pub(super) fn imported(self) -> Result<super::Imported, super::ImportIssue> {
        let Self {
            expected,
            bundle,
            plan,
            facts,
            verified,
            liveness,
            oracle,
            receipt,
        } = self;
        let Verified {
            source_ref,
            cohort,
            history,
        } = verified;
        let protocol_receipt_ref =
            receipt.admit(&expected.protocol, cohort).map_err(|_| super::ImportIssue::Receipt)?;
        let status = if facts.failures.is_empty() {
            super::oracle::status(bundle, oracle, liveness)
        } else {
            "external-failure"
        };
        let status_value = chaoscontrol_smr::observation::protocol_status(
            &expected.protocol,
            cohort,
            &facts.records,
            Some(oracle),
            None,
        )
        .map_err(|_| super::ImportIssue::Receipt)?;
        let binding_refs = bundle
            .observations
            .iter()
            .map(|o| super::super::wire::binding_ref(&o.apply).map_err(|_| super::ImportIssue::Observation))
            .collect::<Result<Vec<_>, _>>()?;
        let expected_value = super::typed_artifact(super::EXPECTED_RECORD_LABEL, expected)?;
        let expected_ref =
            crate::preserves_rail::canonical_hash(&expected_value).map_err(|_| super::ImportIssue::Bounds)?;
        let summary = serde_json::json!({
            "schema": "molten.external-chaoscontrol-conformance.v1",
            "producer_revision": super::super::PRODUCER_REVISION,
            "expected_ref": expected_ref,
            "group_ref": expected.group_ref, "workload_profile_ref": plan.profile_ref,
            "initial_state_ref": expected.workload.initial_state_ref,
            "workload_bounds": expected.workload.bounds, "protocol_bounds": expected.protocol.bounds,
            "protocol_receipt_ref": protocol_receipt_ref,
            "source_ref": source_ref, "execution_ref": bundle.execution_ref,
            "molten": bundle.molten, "history_ref": bundle.history_ref, "observations_ref": bundle.observations_ref,
            "binding_refs": binding_refs, "proposals_ref": super::artifact_ref("molten-chaoscontrol-proposals-v1", &bundle.proposals)?,
            "fault_outcomes": bundle.faults,
            "safety_verdict": history.verdict, "safety_violation_count": history.violations.len(),
            "replay_class": if bundle.replay.is_some() { "bounded-semantic-data-comparison" } else { "not-requested" },
            "replay_comparison": bundle.replay.as_ref().map(|r| &r.comparison),
            "stabilization": bundle.stabilization, "liveness": liveness, "terminal_status": bundle.terminal_status,
            "status": status, "cohort_status": status_value, "oracle": oracle,
            "consumer_failures": facts.failures,
            "evidence_class": "instrumentation", "claim": "bounded-instrumentation", "authority": false,
            "scope": "exact retained finite facts; import is not execution attestation",
            "non_claims": super::NON_CLAIMS,
        });
        let value = crate::preserves_rail::record(super::RECEIPT_RECORD_LABEL, vec![
            crate::live_reliability::checker::canonical_value(&summary).map_err(|_| super::ImportIssue::Bounds)?,
            expected_value,
            super::typed_artifact(super::BUNDLE_RECORD_LABEL, bundle)?,
        ]);
        encode_receipt(value, status)
    }
}

fn encode_receipt(value: preserves::IOValue, status: &str) -> Result<super::Imported, super::ImportIssue> {
    let packed = crate::codec::canonical_bytes(&value).map_err(|_| super::ImportIssue::Bounds)?;
    if u64::try_from(packed.len()).map_err(|_| super::ImportIssue::Bounds)? > super::MAX_RECEIPT_BYTES {
        return Err(super::ImportIssue::Bounds);
    }
    crate::preserves_rail::packed_bounds::preflight(&packed, crate::preserves_rail::packed_bounds::MAX_VALUE_TOKENS)
        .map_err(|_| super::ImportIssue::Bounds)?;
    let receipt_ref = crate::preserves_rail::content_ref_from_bytes(&packed);
    Ok(super::Imported {
        receipt_ref,
        value,
        status: status.into(),
        evidence_class: "instrumentation".into(),
    })
}
