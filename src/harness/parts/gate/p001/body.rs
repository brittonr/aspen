
pub fn repro_bundle_value_with_export_profile(
    report_value: &IoValue,
    command: &[String],
    profile: super::schema::ReproExportProfile,
) -> Result<IoValue> {
    match profile {
        super::schema::ReproExportProfile::DenySensitive => {
            sealed_repro_bundle_value_with_command(report_value, command)
        }
        super::schema::ReproExportProfile::RedactedDiagnostic | super::schema::ReproExportProfile::EncryptedPrivate => {
            super::schema::profiled_repro_bundle_value_with_command(report_value, command, profile)
        }
    }
}

pub fn repro_verify_receipt_value(bundle_value: &IoValue) -> Result<IoValue> {
    let bundle = super::schema::parse_repro_bundle(bundle_value)?;
    if bundle.kind == super::schema::ReproBundleKind::Failure {
        return Err(MoltenError::invalid_harness(format!(
            "failure repro bundle {} wrapping {} is diagnostic-only and cannot be verified as pass evidence",
            bundle.bundle_ref, bundle.artifact_ref
        )));
    }
    if let Some(loss_classification) = bundle.loss_classification.as_deref()
        && loss_classification != "gate-preserving"
    {
        return Err(MoltenError::invalid_harness(format!(
            "{} repro bundle {} is {loss_classification} and cannot be verified as pass evidence",
            bundle.export_profile.as_deref().unwrap_or("profiled"),
            bundle.bundle_ref
        )));
    }
    let embedded_receipt_value = bundle.receipt_value.as_ref().ok_or_else(|| {
        MoltenError::invalid_harness("unsealed report repro bundle cannot satisfy sealed repro verification")
    })?;
    let embedded_receipt = parse_receipt(embedded_receipt_value)?;
    let check = check_value(bundle_value)?;
    if check.artifact_kind != "repro-bundle" || check.artifact_ref != bundle.bundle_ref {
        return Err(MoltenError::invalid_harness("repro verify gate check did not bind bundle artifact"));
    }
    if embedded_receipt.report_ref != check.report_ref || embedded_receipt.suite_ref != check.suite_ref {
        return Err(MoltenError::invalid_harness(
            "repro verify embedded receipt does not match recomputed bundle report refs",
        ));
    }
    Ok(record("repro-verify-receipt-v1", vec![
        string(HARNESS_REPRO_VERIFY_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        tool_value(),
        record("bundle", vec![string(&bundle.bundle_ref)]),
        record("artifact", vec![string(&bundle.artifact_ref)]),
        record("report", vec![string(&check.report_ref)]),
        record("suite", vec![string(&check.suite_ref)]),
        record("gate-receipt", vec![string(&embedded_receipt.receipt_ref)]),
        repro_verify_checks_value(),
    ]))
}

pub fn receipt_value(check: &Check) -> IoValue {
    let mut refs = vec![
        ("artifact", check.artifact_ref.as_str()),
        ("report", check.report_ref.as_str()),
        ("suite", check.suite_ref.as_str()),
        ("executor-preflights", check.executor_preflights_ref.as_str()),
        ("executor-execution-receipts", check.executor_execution_receipts_ref.as_str()),
        ("runtime-predicate-receipts", check.runtime_predicate_receipts_ref.as_str()),
        ("policy", check.policy_ref.as_str()),
        ("policy-gate", check.policy_gate_ref.as_str()),
        ("policy-nickel-source", check.policy_nickel_source_ref.as_str()),
        ("policy-nickel-export", check.policy_nickel_export_ref.as_str()),
        ("policy-basalt-preflight", check.policy_basalt_preflight_ref.as_str()),
        ("budget", check.budget_ref.as_str()),
        ("budget-gate", check.budget_gate_ref.as_str()),
        ("budget-nickel-source", check.budget_nickel_source_ref.as_str()),
        ("budget-nickel-export", check.budget_nickel_export_ref.as_str()),
        ("budget-basalt-preflight", check.budget_basalt_preflight_ref.as_str()),
        ("capabilities", check.capability_ref.as_str()),
        ("capability-gate", check.capability_gate_ref.as_str()),
        ("capability-authority-preflight", check.capability_authority_preflight_ref.as_str()),
        ("ucan-proofset", check.capability_proofset_ref.as_str()),
        ("chain-link", check.chain_evidence.link_ref.as_str()),
        ("chain-anchor", check.chain_evidence.anchor_ref.as_str()),
        ("chain-verify-receipt", check.chain_evidence.verify_receipt_ref.as_str()),
        ("chain-checkpoint", check.chain_evidence.checkpoint_ref.as_str()),
        ("chain-range-predicate", check.chain_evidence.range_predicate_ref.as_str()),
        ("turn-journals", check.turn_journals.aggregate_ref.as_str()),
        ("deterministic-replay-verify", check.deterministic_replay_verify_ref.as_str()),
    ];
    if let Some(redaction_policy_ref) = &check.redaction_policy_ref {
        refs.push(("redaction-policy", redaction_policy_ref.as_str()));
    }
    if let Some(redaction_gate_ref) = &check.redaction_gate_ref {
        refs.push(("redaction-gate", redaction_gate_ref.as_str()));
    }
    record("gate-receipt-v1", vec![
        string(HARNESS_GATE_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("artifact-kind", vec![string(&check.artifact_kind)]),
        record("artifact", vec![string(&check.artifact_ref)]),
        tool_value(),
        artifact_refs_value(&refs),
        validation_value(check),
        replay_value(check),
        checks_value(),
        chain_evidence_value(&check.chain_evidence),
        turn_journals_value(&check.turn_journals),
        string(&check.report_ref),
        string(&check.suite_ref),
        string(&check.final_state_hash),
    ])
}

pub fn parse_receipt(value: &IoValue) -> Result<Receipt> {
    let receipt = simple_record(value, "gate-receipt-v1", 14)?;
    let schema = required_string(&receipt[0], "gate receipt schema")?;
    if schema != HARNESS_GATE_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported gate receipt schema {schema}; expected {HARNESS_GATE_RECEIPT_SCHEMA}"
        )));
    }

    let decision = required_record_string(&receipt[1], "decision", "gate receipt decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported gate receipt decision {decision}")));
    }
    let artifact_kind = required_record_string(&receipt[2], "artifact-kind", "gate receipt artifact kind")?;
    if !matches!(artifact_kind.as_str(), "report" | "repro-bundle") {
        return Err(MoltenError::invalid_harness(format!("unsupported gate receipt artifact kind {artifact_kind}")));
    }
    let artifact_ref = required_record_hash(&receipt[3], "artifact", "gate receipt artifact ref")?;
    validate_tool_record(&receipt[4])?;
    let artifact_refs = parse_artifact_refs(&receipt[5])?;
    require_artifact_ref(&artifact_refs, "artifact", &artifact_ref)?;

    let validation = parse_validation(&receipt[6])?;
    let replay = parse_replay(&receipt[7])?;
    let checks = parse_checks(&receipt[8])?;
    require_all_checks(&checks)?;

    let chain_evidence = parse_chain_evidence(&receipt[9])?;
    let report_ref = required_hash(&receipt[11], "gate receipt report ref")?;
    let suite_ref = required_hash(&receipt[12], "gate receipt suite ref")?;
    let final_state_hash = required_hash(&receipt[13], "gate receipt final state hash")?;
    let turn_journals = parse_turn_journals(&receipt[10], &report_ref, &suite_ref)?;
    require_core_refs(&CoreRefs {
        validation: &validation,
        replay: &replay,
        report: &report_ref,
        suite: &suite_ref,
        final_state: &final_state_hash,
    })?;
    let chain_link = crate::evidence_chain::parse_chain_link(&chain_evidence.link_value)?;
    require_link_context(&chain_link, &report_ref, &suite_ref, &final_state_hash)?;
    require_artifact_ref(&artifact_refs, "report", &report_ref)?;
    require_artifact_ref(&artifact_refs, "suite", &suite_ref)?;
    require_kinds(&artifact_refs, REQUIRED_KINDS)?;
    require_artifact_ref(&artifact_refs, "chain-link", &chain_evidence.link_ref)?;
    require_artifact_ref(&artifact_refs, "chain-anchor", &chain_evidence.anchor_ref)?;
    require_artifact_ref(&artifact_refs, "chain-verify-receipt", &chain_evidence.verify_receipt_ref)?;
    require_artifact_ref(&artifact_refs, "chain-checkpoint", &chain_evidence.checkpoint_ref)?;
    require_artifact_ref(&artifact_refs, "chain-range-predicate", &chain_evidence.range_predicate_ref)?;
    require_artifact_ref(&artifact_refs, "turn-journals", &turn_journals.aggregate_ref)?;
    require_artifact_ref(&artifact_refs, "deterministic-replay-verify", &replay.verify_ref)?;
    if artifact_kind == "repro-bundle" {
        require_kinds(&artifact_refs, REDACTION_KINDS)?;
    }

    Ok(Receipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        artifact_kind,
        artifact_ref,
        report_ref,
        suite_ref,
        final_state_hash,
        checks,
    })
}

pub fn check_summary(check: &Check) -> String {
    format!(
        "gate check ok\nartifact_kind={}\nartifact={}\nreport={}\nsuite={}\nfinal_state={}",
        check.artifact_kind, check.artifact_ref, check.report_ref, check.suite_ref, check.final_state_hash
    )
}

pub fn receipt_summary(value: &IoValue) -> Result<String> {
    let receipt = parse_receipt(value)?;
    Ok(format!(
        "gate receipt {}\ndecision={}\nartifact_kind={}\nartifact={}\nreport={}\nsuite={}\nfinal_state={}\nchecks={}",
        receipt.receipt_ref,
        receipt.decision,
        receipt.artifact_kind,
        receipt.artifact_ref,
        receipt.report_ref,
        receipt.suite_ref,
        receipt.final_state_hash,
        receipt.checks.len()
    ))
}

pub fn parse_repro_verify_receipt(value: &IoValue) -> Result<ReproVerifyReceipt> {
    let receipt = simple_record(value, "repro-verify-receipt-v1", 9)?;
    let schema = required_string(&receipt[0], "repro verify receipt schema")?;
    if schema != HARNESS_REPRO_VERIFY_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported repro verify receipt schema {schema}; expected {HARNESS_REPRO_VERIFY_RECEIPT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "repro verify receipt decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported repro verify receipt decision {decision}")));
    }
    validate_tool_record(&receipt[2])?;
    let bundle_ref = required_record_hash(&receipt[3], "bundle", "repro verify bundle ref")?;
    let artifact_ref = required_record_hash(&receipt[4], "artifact", "repro verify artifact ref")?;
    let report_ref = required_record_hash(&receipt[5], "report", "repro verify report ref")?;
    if artifact_ref != report_ref {
        return Err(MoltenError::invalid_harness("repro verify receipt artifact ref does not match report ref"));
    }
    let suite_ref = required_record_hash(&receipt[6], "suite", "repro verify suite ref")?;
    let gate_receipt_ref = required_record_hash(&receipt[7], "gate-receipt", "repro verify gate receipt ref")?;
    let checks = parse_checks(&receipt[8])?;
    require_check(&checks, "sealed-bundle")?;
    require_check(&checks, "embedded-report")?;
    require_check(&checks, "embedded-gate-receipt")?;
    require_check(&checks, "report-validation")?;
    require_check(&checks, "deterministic-replay")?;
    require_check(&checks, "gate-receipt-recomputed")?;
    Ok(ReproVerifyReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        bundle_ref,
        report_ref,
        suite_ref,
        gate_receipt_ref,
        checks,
    })
}

pub fn repro_verify_receipt_summary(value: &IoValue) -> Result<String> {
    let receipt = parse_repro_verify_receipt(value)?;
    Ok(format!(
        "repro verify receipt {}\ndecision={}\nbundle={}\nreport={}\nsuite={}\ngate_receipt={}\nchecks={}",
        receipt.receipt_ref,
        receipt.decision,
        receipt.bundle_ref,
        receipt.report_ref,
        receipt.suite_ref,
        receipt.gate_receipt_ref,
        receipt.checks.len()
    ))
}

fn validate_sealed_report_bundle(report_value: &IoValue, bundle: &super::schema::ReproBundle) -> Result<()> {
    if bundle.redaction_policy_ref.is_none() || bundle.redaction_gate_ref.is_none() {
        return Err(MoltenError::invalid_harness("sealed report repro bundle missing redaction preflight evidence"));
    }
    let embedded_receipt_value = bundle
        .receipt_value
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("sealed report repro bundle missing embedded gate receipt"))?;
    let embedded_receipt_ref = bundle
        .gate_receipt_ref
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("sealed report repro bundle missing gate receipt ref"))?;
    let receipt = parse_receipt(embedded_receipt_value)?;
    if &receipt.receipt_ref != embedded_receipt_ref {
        return Err(MoltenError::invalid_harness(
            "sealed repro bundle gate receipt ref does not match embedded receipt",
        ));
    }
    if receipt.artifact_kind != "report" {
        return Err(MoltenError::invalid_harness(format!(
            "sealed repro bundle must embed a report gate receipt, got {}",
            receipt.artifact_kind
        )));
    }
    if receipt.artifact_ref != bundle.artifact_ref || receipt.report_ref != bundle.artifact_ref {
        return Err(MoltenError::invalid_harness(
            "sealed repro bundle gate receipt does not bind the embedded report ref",
        ));
    }
    let expected_report_check = check_report(report_value, "report".to_string(), None)?;
    let expected_receipt_value = receipt_value(&expected_report_check);
    let expected_receipt_ref = canonical_hash(&expected_receipt_value)?;
    let actual_receipt_ref = canonical_hash(embedded_receipt_value)?;
    if actual_receipt_ref != expected_receipt_ref {
        return Err(MoltenError::invalid_harness(format!(
            "sealed repro bundle embedded gate receipt does not match report: receipt hashes to {actual_receipt_ref}, expected {expected_receipt_ref}"
        )));
    }
    Ok(())
}
