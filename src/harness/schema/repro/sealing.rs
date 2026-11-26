use preserves::ValueImpl;

pub(super) fn parse_failure_repro_bundle(
    bundle_value: &preserves::IOValue,
    bundle: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<super::model::ReproBundle> {
    let kind = super::fields::required_record_string(&bundle[1], "bundle-kind", "repro bundle kind")?;
    if kind != "failure" {
        return Err(crate::error::Failure::invalid_harness(format!("expected failure repro bundle kind, got {kind}")));
    }
    super::manifest::validate_tool_record(&bundle[2])?;
    super::manifest::validate_sequence_record(&bundle[3], "command", "repro bundle command")?;
    super::manifest::validate_sequence_record(&bundle[4], "replay-instructions", "repro bundle replay instructions")?;
    let artifact_refs = super::manifest::parse_artifact_refs(&bundle[5])?;
    let failure_ref = super::fields::required_string(&bundle[6], "repro bundle failure ref")?;
    super::manifest::require_artifact_ref(&artifact_refs, "failure", &failure_ref)?;
    let failure_value = crate::preserves_rail::value_to_iovalue(&bundle[7]);
    let failure = super::outcome::parse_failure(&failure_value)?;
    if failure.failure_ref != failure_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "failure repro bundle ref mismatch: bundle has {failure_ref}, embedded failure hashes to {}",
            failure.failure_ref
        )));
    }
    Ok(super::model::ReproBundle {
        bundle_ref: crate::preserves_rail::canonical_hash(bundle_value)?,
        kind: super::model::ReproBundleKind::Failure,
        artifact_ref: failure_ref,
        report_value: None,
        failure_value: Some(failure_value),
        gate_receipt_ref: None,
        receipt_value: None,
        redaction_policy_ref: None,
        redaction_gate_ref: None,
        export_profile: None,
        export_profile_ref: None,
        export_profile_value: None,
        source_report_ref: None,
        source_suite_ref: None,
        redaction_transform_manifest_ref: None,
        redaction_transform_manifest_value: None,
        redaction_transform_receipt_ref: None,
        redaction_transform_receipt_value: None,
        private_bundle_profile_ref: None,
        private_bundle_profile_value: None,
        loss_classification: Some("diagnostic-only".to_string()),
        encrypted_refs: Vec::new(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReproSeal {
    pub(super) gate_receipt_ref: String,
}

pub(super) fn parse_repro_seal(
    value: &preserves::Value<preserves::IOValue>,
    report_ref: &str,
    suite_ref: &str,
    profile: &str,
    replay_status: &str,
) -> crate::error::Result<ReproSeal> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let seal = super::fields::simple_record(&value, "repro-seal", 7)?;
    let schema = super::fields::required_string(&seal[0], "repro seal schema")?;
    if schema != crate::preserves_rail::HARNESS_REPRO_SEAL_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported repro seal schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REPRO_SEAL_SCHEMA
        )));
    }
    let decision = super::fields::required_record_string(&seal[1], "decision", "repro seal decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported repro seal decision {decision}")));
    }
    let gate_receipt_ref =
        super::fields::required_record_hash(&seal[2], "gate-receipt-ref", "repro seal gate receipt ref")?;
    let sealed_report_ref = super::fields::required_record_hash(&seal[3], "report-ref", "repro seal report ref")?;
    if sealed_report_ref != report_ref {
        return Err(crate::error::Failure::invalid_harness("repro seal report ref does not match bundle report ref"));
    }
    let sealed_suite_ref = super::fields::required_record_hash(&seal[4], "suite-ref", "repro seal suite ref")?;
    if sealed_suite_ref != suite_ref {
        return Err(crate::error::Failure::invalid_harness("repro seal suite ref does not match bundle suite ref"));
    }
    let sealed_profile = super::fields::required_record_string(&seal[5], "profile", "repro seal profile")?;
    if sealed_profile != profile {
        return Err(crate::error::Failure::invalid_harness("repro seal profile does not match bundle profile"));
    }
    let sealed_replay_status =
        super::fields::required_record_string(&seal[6], "replay-status", "repro seal replay status")?;
    if sealed_replay_status != replay_status {
        return Err(crate::error::Failure::invalid_harness(
            "repro seal replay status does not match bundle replay status",
        ));
    }
    Ok(ReproSeal { gate_receipt_ref })
}

pub(super) fn parse_seal_checks(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let checks_record = super::fields::simple_record(&value, "seal-checks", 1)?;
    let check_values = super::fields::required_sequence(&checks_record[0], "repro seal checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = crate::preserves_rail::value_to_iovalue(&check_value);
        let check = super::fields::simple_record(&check_value, "check", 2)?;
        let name = super::fields::required_string(&check[0], "repro seal check name")?;
        let status = super::fields::required_string(&check[1], "repro seal check status")?;
        if status != "pass" {
            return Err(crate::error::Failure::invalid_harness(format!("repro seal check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

pub(super) fn require_seal_check(checks: &[String], expected: &str) -> crate::error::Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!("repro seal missing {expected} check")))
    }
}

pub(super) struct ReproReportMatchInput<'a> {
    pub(super) report: &'a super::model::Report,
    pub(super) report_ref: &'a str,
    pub(super) suite_ref: &'a str,
    pub(super) initial_state_hash: &'a str,
    pub(super) final_state_hash: &'a str,
    pub(super) replay_status: &'a str,
    pub(super) profile: &'a str,
    pub(super) actors: &'a [super::model::ActorDecl],
    pub(super) effect_log: &'a [super::observed::EffectLogEntry],
    pub(super) suite_value: &'a preserves::IOValue,
}

pub(super) fn require_report_artifact_refs(
    refs: &[(String, String)],
    report: &super::model::Report,
) -> crate::error::Result<()> {
    for (kind, artifact_ref) in super::manifest::report_artifact_refs(report, None, None)? {
        super::manifest::require_artifact_ref(refs, &kind, &artifact_ref)?;
    }
    Ok(())
}

pub(super) fn require_repro_report_matches(input: &ReproReportMatchInput<'_>) -> crate::error::Result<()> {
    if input.report.report_ref != input.report_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "repro bundle report ref mismatch: bundle has {}, embedded report hashes to {}",
            input.report_ref, input.report.report_ref
        )));
    }
    if input.report.suite_ref != input.suite_ref {
        return Err(crate::error::Failure::invalid_harness("repro bundle suite ref does not match embedded report"));
    }
    if input.report.initial_state_hash != input.initial_state_hash
        || input.report.final_state_hash != input.final_state_hash
    {
        return Err(crate::error::Failure::invalid_harness("repro bundle state refs do not match embedded report"));
    }
    if input.report.replay_status != input.replay_status || input.report.profile != input.profile {
        return Err(crate::error::Failure::invalid_harness(
            "repro bundle replay/profile metadata does not match embedded report",
        ));
    }
    if input.report.actors != input.actors {
        return Err(crate::error::Failure::invalid_harness(
            "repro bundle actor registry does not match embedded report",
        ));
    }
    if input.report.effect_log != input.effect_log {
        return Err(crate::error::Failure::invalid_harness("repro bundle effect log does not match embedded report"));
    }
    if &input.report.suite_value != input.suite_value {
        return Err(crate::error::Failure::invalid_harness("repro bundle suite value does not match embedded report"));
    }
    Ok(())
}

pub(super) fn repro_seal_value(report: &super::model::Report, gate_receipt_ref: &str) -> preserves::IOValue {
    crate::preserves_rail::record("repro-seal", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_REPRO_SEAL_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("gate-receipt-ref", vec![crate::preserves_rail::string(gate_receipt_ref)]),
        crate::preserves_rail::record("report-ref", vec![crate::preserves_rail::string(&report.report_ref)]),
        crate::preserves_rail::record("suite-ref", vec![crate::preserves_rail::string(&report.suite_ref)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(&report.profile)]),
        crate::preserves_rail::record("replay-status", vec![crate::preserves_rail::string(&report.replay_status)]),
    ])
}

pub(super) fn sealed_repro_checks_value() -> preserves::IOValue {
    crate::preserves_rail::record("seal-checks", vec![crate::preserves_rail::sequence(
        [
            "sealed-report",
            "embedded-gate-receipt",
            "report-ref-binding",
            "suite-ref-binding",
            "actor-registry-binding",
            "effect-log-binding",
            "policy-gate-ref-binding",
            "capability-gate-ref-binding",
            "budget-gate-ref-binding",
            "redaction-preflight",
            "redaction-gate-ref-binding",
            "no-sensitive-markers",
            "replay-metadata-binding",
        ]
        .iter()
        .map(|name| {
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string(*name),
                crate::preserves_rail::string("pass"),
            ])
        })
        .collect(),
    )])
}
