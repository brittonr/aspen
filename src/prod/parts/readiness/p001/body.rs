
pub fn backup_restore_drill_value(input: &BackupRestoreDrillInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("backup restore drill name", input.drill_name)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("ledger", input.ledger_refs),
        ("redb", input.redb_refs),
        ("chunk", input.chunk_refs),
        ("identity", input.identity_refs),
        ("retention pin", input.retention_pin_refs),
        ("source gate", input.source_gate_refs),
        ("restore verification", input.restore_verification_refs),
        ("tamper denial", input.tamper_denial_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    Ok(record("prod-ops-backup-restore-drill-v1", vec![
        string(PROD_OPS_BACKUP_RESTORE_DRILL_SCHEMA),
        decision_field(input.decision),
        record("drill", vec![string(input.drill_name)]),
        refs_field("ledgers", input.ledger_refs)?,
        refs_field("redb-stores", input.redb_refs)?,
        refs_field("chunks", input.chunk_refs)?,
        refs_field("identity-metadata", input.identity_refs)?,
        refs_field("retention-pins", input.retention_pin_refs)?,
        refs_field("source-gates", input.source_gate_refs)?,
        refs_field("restore-verification", input.restore_verification_refs)?,
        refs_field("tamper-denials", input.tamper_denial_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value(
                "ledger-redb-chunk-identity-bound",
                pass_check(
                    input.ledger_refs.is_empty()
                        || input.redb_refs.is_empty()
                        || input.chunk_refs.is_empty()
                        || input.identity_refs.is_empty(),
                ),
            ),
            check_value(
                "retention-source-gate-bound",
                pass_check(input.retention_pin_refs.is_empty() || input.source_gate_refs.is_empty()),
            ),
            check_value("tampered-backup-denies-restore", pass_check(input.tamper_denial_refs.is_empty())),
            check_value("restore-verifies-before-operation", pass_check(input.restore_verification_refs.is_empty())),
        ]),
    ]))
}

pub fn upgrade_rollback_drill_value(input: &UpgradeRollbackDrillInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("upgrade rollback plan", input.plan_name)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("migration", input.migration_refs),
        ("smoke", input.smoke_refs),
        ("rollback eligibility", input.rollback_eligibility_refs),
        ("irreversible exclusion", input.irreversible_exclusion_refs),
        ("post rollback", input.post_rollback_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    Ok(record("prod-ops-upgrade-rollback-drill-v1", vec![
        string(PROD_OPS_UPGRADE_ROLLBACK_DRILL_SCHEMA),
        decision_field(input.decision),
        record("plan", vec![string(input.plan_name)]),
        refs_field("migrations", input.migration_refs)?,
        refs_field("smoke-or-dogfood", input.smoke_refs)?,
        refs_field("rollback-eligibility", input.rollback_eligibility_refs)?,
        refs_field("irreversible-exclusions", input.irreversible_exclusion_refs)?,
        refs_field("post-rollback-verification", input.post_rollback_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("migration-receipts-bound", pass_check(input.migration_refs.is_empty())),
            check_value("copied-state-smoke-bound", pass_check(input.smoke_refs.is_empty())),
            check_value("rollback-eligibility-bound", pass_check(input.rollback_eligibility_refs.is_empty())),
            check_value("irreversible-operations-excluded", pass_check(input.irreversible_exclusion_refs.is_empty())),
        ]),
    ]))
}

pub fn observability_slo_value(input: &ObservabilitySloInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("observability snapshot name", input.snapshot_name)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_refs("adapter health", input.adapter_health_refs, input.decision)?;
    require_pass_refs("control loop", input.control_loop_refs, input.decision)?;
    require_pass_refs("source gate freshness", input.source_gate_freshness_refs, input.decision)?;
    require_pass_refs("live transport", input.live_transport_refs, input.decision)?;
    require_pass_metric_bound("queue depth", input.queue_depth, input.max_queue_depth, input.decision)?;
    Ok(record("prod-ops-observability-slo-v1", vec![
        string(PROD_OPS_OBSERVABILITY_SLO_SCHEMA),
        decision_field(input.decision),
        record("snapshot", vec![string(input.snapshot_name)]),
        refs_field("adapter-health", input.adapter_health_refs)?,
        record("queue-depth", vec![u64_value(input.queue_depth)]),
        record("max-queue-depth", vec![u64_value(input.max_queue_depth)]),
        refs_field("control-loop", input.control_loop_refs)?,
        refs_field("resource-pressure", input.resource_pressure_refs)?,
        refs_field("retention-drift", input.retention_drift_refs)?,
        refs_field("source-gate-freshness", input.source_gate_freshness_refs)?,
        refs_field("live-transport", input.live_transport_refs)?,
        refs_field("import-export-failures", input.import_export_failure_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("adapter-health-bound", pass_check(input.adapter_health_refs.is_empty())),
            check_value("queue-depth-within-slo", pass_check(input.queue_depth > input.max_queue_depth)),
            check_value("control-loop-liveness-bound", pass_check(input.control_loop_refs.is_empty())),
            check_value("logs-auxiliary-receipts-canonical", "pass"),
        ]),
    ]))
}

pub fn runbook_check_value(input: &RunbookCheckInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("runbook name", input.runbook_name)?;
    validate_text_field("runbook operation", input.operation)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_refs("canonical artifact", input.canonical_artifact_refs, input.decision)?;
    require_pass_refs("denial fixture", input.denial_fixture_refs, input.decision)?;
    Ok(record("prod-ops-runbook-check-v1", vec![
        string(PROD_OPS_RUNBOOK_CHECK_SCHEMA),
        decision_field(input.decision),
        record("runbook", vec![string(input.runbook_name)]),
        record("operation", vec![string(input.operation)]),
        refs_field("canonical-artifacts", input.canonical_artifact_refs)?,
        refs_field("denial-fixtures", input.denial_fixture_refs)?,
        refs_field("auxiliary-logs", input.auxiliary_log_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("canonical-receipts-not-terminal-output", pass_check(input.canonical_artifact_refs.is_empty())),
            check_value("denial-path-covered", pass_check(input.denial_fixture_refs.is_empty())),
            check_value("logs-auxiliary-only", "pass"),
        ]),
    ]))
}

pub fn threat_model_value(input: &ThreatModelInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("threat model name", input.model_name)?;
    validate_text_slice("threat entry", input.threat_entries)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_texts("threat entry", input.threat_entries, input.decision)?;
    require_pass_refs("mapped gate", input.mapped_gate_refs, input.decision)?;
    require_pass_coverage(
        "threat model mapping",
        &[
            input.mapped_gate_refs,
            input.drill_refs,
            input.negative_suite_refs,
            input.unresolved_risk_refs,
        ],
        input.decision,
    )?;
    if is_pass(input.decision) && !input.unresolved_risk_refs.is_empty() {
        require_non_empty_refs("pilot consequence", input.pilot_consequence_refs)?;
    }
    Ok(record("prod-security-threat-model-v1", vec![
        string(PROD_SECURITY_THREAT_MODEL_SCHEMA),
        decision_field(input.decision),
        record("model", vec![string(input.model_name)]),
        texts_field("threats", input.threat_entries)?,
        refs_field("mapped-gates", input.mapped_gate_refs)?,
        refs_field("drills", input.drill_refs)?,
        refs_field("negative-suites", input.negative_suite_refs)?,
        refs_field("unresolved-risks", input.unresolved_risk_refs)?,
        refs_field("pilot-consequences", input.pilot_consequence_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("threats-named", pass_check(input.threat_entries.is_empty())),
            check_value(
                "gate-drill-or-risk-mapped",
                pass_check(
                    input.mapped_gate_refs.is_empty()
                        && input.drill_refs.is_empty()
                        && input.negative_suite_refs.is_empty()
                        && input.unresolved_risk_refs.is_empty(),
                ),
            ),
            check_value(
                "unresolved-risks-have-pilot-consequences",
                pass_check(!input.unresolved_risk_refs.is_empty() && input.pilot_consequence_refs.is_empty()),
            ),
        ]),
    ]))
}

pub fn security_drill_value(input: &SecurityDrillInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_allowed_text("security drill kind", input.drill_kind, SECURITY_DRILL_KINDS)?;
    validate_text_field("security drill scenario", input.scenario)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_refs("pass evidence", input.pass_evidence_refs, input.decision)?;
    require_pass_refs("denial", input.denial_refs, input.decision)?;
    require_pass_refs("cleanup", input.cleanup_refs, input.decision)?;
    Ok(record("prod-security-drill-v1", vec![
        string(PROD_SECURITY_DRILL_SCHEMA),
        decision_field(input.decision),
        record("drill-kind", vec![string(input.drill_kind)]),
        record("scenario", vec![string(input.scenario)]),
        refs_field("pass-evidence", input.pass_evidence_refs)?,
        refs_field("denials", input.denial_refs)?,
        refs_field("cleanup", input.cleanup_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value(
                "revocation-or-attenuation-denies-before-side-effects",
                pass_check(input.denial_refs.is_empty()),
            ),
            check_value("cleanup-actions-bound", pass_check(input.cleanup_refs.is_empty())),
            check_value("drill-receipt-does-not-grant-authority", "pass"),
        ]),
    ]))
}

pub fn redaction_audit_value(input: &RedactionAuditInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("redaction audit name", input.audit_name)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_refs("surface", input.surface_refs, input.decision)?;
    require_pass_refs("redaction", input.redaction_refs, input.decision)?;
    require_pass_refs("plaintext denial", input.plaintext_denial_refs, input.decision)?;
    Ok(record("prod-security-redaction-audit-v1", vec![
        string(PROD_SECURITY_REDACTION_AUDIT_SCHEMA),
        decision_field(input.decision),
        record("audit", vec![string(input.audit_name)]),
        refs_field("surfaces", input.surface_refs)?,
        refs_field("redactions", input.redaction_refs)?,
        refs_field("reveal-gates", input.reveal_gate_refs)?,
        refs_field("plaintext-denials", input.plaintext_denial_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("surfaces-covered", pass_check(input.surface_refs.is_empty())),
            check_value("redaction-or-encryption-bound", pass_check(input.redaction_refs.is_empty())),
            check_value("plaintext-secret-export-denied", pass_check(input.plaintext_denial_refs.is_empty())),
        ]),
    ]))
}

pub fn supply_chain_review_value(input: &SupplyChainReviewInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("supply chain review name", input.review_name)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("release", input.release_refs),
        ("source gate", input.source_gate_refs),
        ("provenance", input.provenance_refs),
        ("build verification", input.build_verify_refs),
        ("signed keyring", input.signed_keyring_refs),
        ("sensitive artifact", input.sensitive_artifact_refs),
        ("mismatch denial", input.mismatch_denial_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    Ok(record("prod-security-supply-chain-review-v1", vec![
        string(PROD_SECURITY_SUPPLY_CHAIN_REVIEW_SCHEMA),
        decision_field(input.decision),
        record("review", vec![string(input.review_name)]),
        refs_field("release", input.release_refs)?,
        refs_field("source-gates", input.source_gate_refs)?,
        refs_field("provenance", input.provenance_refs)?,
        refs_field("build-verification", input.build_verify_refs)?,
        refs_field("signed-keyring", input.signed_keyring_refs)?,
        refs_field("sensitive-artifacts", input.sensitive_artifact_refs)?,
        refs_field("mismatch-denials", input.mismatch_denial_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value(
                "release-source-provenance-build-bound",
                pass_check(
                    input.release_refs.is_empty()
                        || input.source_gate_refs.is_empty()
                        || input.provenance_refs.is_empty()
                        || input.build_verify_refs.is_empty(),
                ),
            ),
            check_value("signed-keyring-currentness-bound", pass_check(input.signed_keyring_refs.is_empty())),
            check_value("stale-sensitive-artifact-denies", pass_check(input.mismatch_denial_refs.is_empty())),
        ]),
    ]))
}
