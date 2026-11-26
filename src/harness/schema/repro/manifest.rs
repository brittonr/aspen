use preserves::ValueImpl;

pub(super) fn report_artifact_refs_value(
    report: &super::model::Report,
    gate_receipt_ref: Option<&str>,
    redaction_refs: Option<(&str, &str)>,
) -> crate::error::Result<preserves::IOValue> {
    Ok(super::bundling::artifact_refs_owned_value(&report_artifact_refs(
        report,
        gate_receipt_ref,
        redaction_refs,
    )?))
}

pub(super) fn report_artifact_refs(
    report: &super::model::Report,
    gate_receipt_ref: Option<&str>,
    redaction_refs: Option<(&str, &str)>,
) -> crate::error::Result<Vec<(String, String)>> {
    let policy_gate = report
        .policy_gate
        .as_ref()
        .ok_or_else(|| crate::error::Failure::invalid_harness("report repro bundle missing policy gate evidence"))?;
    let capability_gate = report.capability_gate.as_ref().ok_or_else(|| {
        crate::error::Failure::invalid_harness("report repro bundle missing capability gate evidence")
    })?;
    let budget_gate = report
        .budget_gate
        .as_ref()
        .ok_or_else(|| crate::error::Failure::invalid_harness("report repro bundle missing budget gate evidence"))?;
    let executor_preflights = report.executor_preflights.as_ref().ok_or_else(|| {
        crate::error::Failure::invalid_harness("report repro bundle missing executor preflight evidence")
    })?;
    let authority_refs = report_authority_aggregate_refs(report)?;
    let mut refs = vec![
        ("report".to_string(), report.report_ref.clone()),
        ("suite".to_string(), report.suite_ref.clone()),
        ("initial-state".to_string(), report.initial_state_hash.clone()),
        ("final-state".to_string(), report.final_state_hash.clone()),
        (
            "actor-registry".to_string(),
            crate::preserves_rail::canonical_hash(&super::roster::actor_registry_value(&report.actors))?,
        ),
        (
            "executor-preflights".to_string(),
            crate::preserves_rail::canonical_hash(&executor_preflights.value)?,
        ),
        (
            "effect-log".to_string(),
            crate::preserves_rail::canonical_hash(&super::tape::effect_log_value(&report.effect_log))?,
        ),
        ("policy".to_string(), policy_gate.policy_ref.clone()),
        ("policy-gate".to_string(), crate::preserves_rail::canonical_hash(&policy_gate.value)?),
        ("policy-nickel-source".to_string(), policy_gate.nickel_source_ref.clone()),
        ("policy-nickel-export".to_string(), policy_gate.nickel_export_ref.clone()),
        ("policy-basalt-preflight".to_string(), policy_gate.basalt_preflight_ref.clone()),
        ("budget".to_string(), budget_gate.budget_ref.clone()),
        ("budget-gate".to_string(), crate::preserves_rail::canonical_hash(&budget_gate.value)?),
        ("budget-nickel-source".to_string(), budget_gate.nickel_source_ref.clone()),
        ("budget-nickel-export".to_string(), budget_gate.nickel_export_ref.clone()),
        ("budget-basalt-preflight".to_string(), budget_gate.basalt_preflight_ref.clone()),
        ("capabilities".to_string(), capability_gate.capability_ref.clone()),
        ("capability-gate".to_string(), crate::preserves_rail::canonical_hash(&capability_gate.value)?),
        ("capability-authority-preflight".to_string(), capability_gate.authority_preflight_ref.clone()),
        ("ucan-proofset".to_string(), capability_gate.proofset_ref.clone()),
        ("ucan-verification-receipts".to_string(), authority_refs.ucan_verification_receipts_ref),
        ("derived-grants".to_string(), authority_refs.derived_grants_ref),
        ("basalt-enforcement-receipts".to_string(), authority_refs.authority_receipts_ref),
        ("authority-requests".to_string(), authority_refs.request_refs_ref),
    ];
    if let Some(gate_receipt_ref) = gate_receipt_ref {
        refs.push(("gate-receipt".to_string(), gate_receipt_ref.to_string()));
    }
    if let Some((redaction_policy_ref, redaction_gate_ref)) = redaction_refs {
        refs.push(("redaction-policy".to_string(), redaction_policy_ref.to_string()));
        refs.push(("redaction-gate".to_string(), redaction_gate_ref.to_string()));
    }
    Ok(refs)
}

struct ReportAuthorityAggregateRefs {
    ucan_verification_receipts_ref: String,
    derived_grants_ref: String,
    authority_receipts_ref: String,
    request_refs_ref: String,
}

fn report_authority_aggregate_refs(
    report: &super::model::Report,
) -> crate::error::Result<ReportAuthorityAggregateRefs> {
    let authorities = report
        .observations
        .iter()
        .flat_map(|observation| &observation.events)
        .filter(|event| super::tape::event_boundary(event) == super::observed::EventBoundary::PolicyDecision)
        .filter_map(|event| {
            super::vetting::parse_admission_decision_event(event)
                .map(|admission| admission.authority)
                .transpose()
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    Ok(ReportAuthorityAggregateRefs {
        ucan_verification_receipts_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "ucan-verification-receipts",
            vec![crate::preserves_rail::sequence(
                authorities
                    .iter()
                    .flat_map(|authority| authority.ucan_verification_receipt_refs.as_slice())
                    .map(crate::preserves_rail::string)
                    .collect(),
            )],
        ))?,
        derived_grants_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "derived-grants",
            vec![crate::preserves_rail::sequence(
                authorities
                    .iter()
                    .flat_map(|authority| authority.derived_grant_refs.as_slice())
                    .map(crate::preserves_rail::string)
                    .collect(),
            )],
        ))?,
        authority_receipts_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "basalt-enforcement-receipts",
            vec![crate::preserves_rail::sequence(
                authorities
                    .iter()
                    .map(|authority| crate::preserves_rail::string(&authority.basalt_enforcement_receipt_ref))
                    .collect(),
            )],
        ))?,
        request_refs_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "authority-requests",
            vec![crate::preserves_rail::sequence(
                authorities.iter().map(|authority| crate::preserves_rail::string(&authority.request_ref)).collect(),
            )],
        ))?,
    })
}

pub(super) fn validate_tool_record(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<()> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let tool = super::fields::simple_record(&value, "tool", 2)?;
    let name = super::fields::required_string(&tool[0], "repro bundle tool name")?;
    if name != "molten" {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported repro bundle tool {name}")));
    }
    let version = super::fields::required_string(&tool[1], "repro bundle tool version")?;
    if version.is_empty() {
        return Err(crate::error::Failure::invalid_harness("repro bundle tool version must not be empty"));
    }
    Ok(())
}

pub(super) fn validate_sequence_record(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    field: &str,
) -> crate::error::Result<()> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = super::fields::simple_record(&value, label, 1)?;
    super::fields::required_sequence(&record[0], field)?;
    Ok(())
}

pub(super) fn parse_artifact_refs(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<Vec<(String, String)>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let artifact_refs = super::fields::simple_record(&value, "artifact-refs", 1)?;
    let ref_values = super::fields::required_sequence(&artifact_refs[0], "repro bundle artifact refs")?;
    let mut refs = Vec::with_capacity(ref_values.len());
    for ref_value in ref_values.iter() {
        let ref_value = crate::preserves_rail::value_to_iovalue(&ref_value);
        let artifact_ref = super::fields::simple_record(&ref_value, "artifact-ref", 2)?;
        refs.push((
            super::fields::required_string(&artifact_ref[0], "artifact ref kind")?,
            super::fields::required_string(&artifact_ref[1], "artifact ref value")?,
        ));
    }
    Ok(refs)
}

pub(super) fn require_artifact_ref(refs: &[(String, String)], kind: &str, expected: &str) -> crate::error::Result<()> {
    if refs.iter().any(|(actual_kind, actual_ref)| actual_kind == kind && actual_ref == expected) {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!(
            "repro bundle artifact refs missing {kind} ref {expected}"
        )))
    }
}
