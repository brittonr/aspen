
fn stale_plan_case(dir: &std::path::Path) -> CliResult<()> {
    let root = dir.join("stale-plan-root");
    let candidate = setup_retention_cli_candidate(CandidateInput {
        root: &root,
        label: "stale-plan",
        object_ref: test_ref("retention-stale-object")?,
        object_kind: "artifact",
        retention_class: molten::retention::CLASS_PUBLIC_ARTIFACT,
        action: molten::retention::ACTION_DELETE,
    })?;
    let plan = run_gc_plan_cli(&candidate, "ledger-gc", &dir.join("stale-plan.preserves"))?;
    molten::retention::pin_object(&root, molten::retention::PinInput {
        object_ref: candidate.object_ref.clone(),
        object_kind: candidate.object_kind.clone(),
        retention_class: candidate.retention_class.clone(),
        source: molten::retention::SOURCE_OPERATOR_HOLD.to_string(),
        reason: "negative CLI stale plan".to_string(),
        owner_ref: candidate.requester_ref.clone(),
        expiry_ref: None,
        policy_refs: vec![candidate.policy_ref.clone()],
        evidence_refs: vec![candidate.support_ref.clone()],
        has_authority: true,
    })?;
    let apply_path = dir.join("stale-apply.preserves");
    let output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&root)
        .args(["--plan-ref"])
        .arg(&plan.plan_ref)
        .args(["--receipt-out"])
        .arg(&apply_path)
        .output()?;
    assert_success(&output, "retention apply stale plan ref");
    let receipt = molten::retention::parse_gc_apply(&read_preserves(&apply_path)?)?;
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.retention_receipt_ref.is_none());
    assert!(receipt.tombstone_ref.is_none());
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-apply-plan-drift"));
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "active-pins-present"));
    Ok(())
}

fn missing_apply_case(dir: &std::path::Path) -> CliResult<()> {
    let root = dir.join("missing-apply-ledger");
    let artifact =
        molten::ledger::import_artifact(&root, &molten::preserves_rail::parse_text("<artifact \"missing-apply\">")?)?;
    let candidate = setup_retention_cli_candidate(CandidateInput {
        root: &root,
        label: "missing-apply",
        object_ref: artifact.artifact_ref.clone(),
        object_kind: &artifact.artifact_kind,
        retention_class: molten::retention::CLASS_PUBLIC_ARTIFACT,
        action: molten::retention::ACTION_DELETE,
    })?;
    let receipt = dir.join("missing-apply-ledger-gc.preserves");
    let mut command = molten_cmd();
    command.args(["test", "ledger", "gc", "--ledger"]).arg(&root).args(["--receipt-out"]).arg(&receipt);
    add_retention_args(&mut command, &candidate);
    let output = command.output()?;
    assert_success(&output, "ledger gc missing apply ref");
    assert!(stdout(&output).contains("decision=deny"));
    let receipt_text = std::fs::read_to_string(&receipt)?;
    assert!(receipt_text.contains("retention-gc-execute-apply-missing"));
    molten::ledger::read_artifact(&root, &candidate.object_ref)?;
    Ok(())
}

fn wrong_apply_case(dir: &std::path::Path) -> CliResult<()> {
    let root = dir.join("wrong-apply-ledger");
    let artifact =
        molten::ledger::import_artifact(&root, &molten::preserves_rail::parse_text("<artifact \"wrong-apply\">")?)?;
    let candidate = setup_retention_cli_candidate(CandidateInput {
        root: &root,
        label: "wrong-apply",
        object_ref: artifact.artifact_ref.clone(),
        object_kind: &artifact.artifact_kind,
        retention_class: molten::retention::CLASS_PUBLIC_ARTIFACT,
        action: molten::retention::ACTION_DELETE,
    })?;
    let plan = run_gc_plan_cli(&candidate, "chunk-gc", &dir.join("wrong-plan.preserves"))?;
    let apply_path = dir.join("wrong-apply.preserves");
    let apply_output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&root)
        .args(["--plan-ref"])
        .arg(&plan.plan_ref)
        .args(["--receipt-out"])
        .arg(&apply_path)
        .output()?;
    assert_success(&apply_output, "retention apply wrong subsystem plan");
    let apply = molten::retention::parse_gc_apply(&read_preserves(&apply_path)?)?;
    assert_eq!(apply.decision, "pass");
    let receipt = dir.join("wrong-apply-ledger-gc.preserves");
    let mut command = molten_cmd();
    command
        .args(["test", "ledger", "gc", "--ledger"])
        .arg(&root)
        .args(["--apply-ref"])
        .arg(&apply.apply_ref)
        .args(["--receipt-out"])
        .arg(&receipt);
    add_retention_args(&mut command, &candidate);
    let output = command.output()?;
    assert_success(&output, "ledger gc wrong apply ref");
    assert!(stdout(&output).contains("decision=deny"));
    let receipt_text = std::fs::read_to_string(&receipt)?;
    assert!(receipt_text.contains("retention-gc-execute-apply-scope-mismatch"));
    molten::ledger::read_artifact(&root, &candidate.object_ref)?;
    Ok(())
}

fn audit_case(dir: &std::path::Path) -> CliResult<()> {
    let root = dir.join("audit-root");
    let missing = molten_cmd()
        .args(["test", "retention", "gc-audit", "--root"])
        .arg(&root)
        .args(["--execution-ref"])
        .arg(test_ref("missing-execution")?)
        .args(["--out"])
        .arg(dir.join("missing-execution-audit.preserves"))
        .output()?;
    assert_failure(&missing, "retention audit missing execution ref");
    let execution = molten::retention::store_gc_execution_gate(molten::retention::GcExecutionGateInput {
        root: &root,
        subsystem: "ledger-gc",
        action: molten::retention::ACTION_DELETE,
        object_ref: &test_ref("denied-execution-object")?,
        object_kind: "artifact",
        retention_class: molten::retention::CLASS_PUBLIC_ARTIFACT,
        apply_ref: None,
    })?;
    let audit_path = dir.join("denied-execution-audit.preserves");
    let output = molten_cmd()
        .args(["test", "retention", "gc-audit", "--root"])
        .arg(&root)
        .args(["--execution-ref"])
        .arg(&execution.execution_ref)
        .args(["--out"])
        .arg(&audit_path)
        .output()?;
    assert_success(&output, "retention audit denied execution ref");
    let audit = molten::retention::parse_gc_audit(&read_preserves(&audit_path)?)?;
    assert_eq!(audit.decision, "deny");
    assert!(audit.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-audit-apply-missing"));
    assert!(audit.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-audit-plan-missing"));
    Ok(())
}

#[test]
fn cli_catalog_discovers_gc_audit_chains() -> CliResult<()> {
    let dir = temp_dir("cli-retention-gc-catalog")?;
    let registry = dir.join("registry");
    let ledger_root = dir.join("ledger");
    let retention_root = dir.join("retention-root");
    let candidate = setup_retention_cli_candidate(CandidateInput {
        root: &retention_root,
        label: "catalog-audit",
        object_ref: test_ref("retention-catalog-audit-object")?,
        object_kind: "artifact",
        retention_class: molten::retention::CLASS_PUBLIC_ARTIFACT,
        action: molten::retention::ACTION_DELETE,
    })?;
    let fixture = setup_gc_catalog_fixture(&candidate, "ledger-gc", &dir)?;

    let (explain_path, explain_ref) = run_explain(&retention_root, &dir, &fixture)?;
    let (bundle_dir, bundle_ref) = run_bundle(&retention_root, &dir, &explain_path, &explain_ref)?;
    check_profile(&registry, &ledger_root, &bundle_dir)?;
    run_verify(&registry, &ledger_root, &dir, &bundle_dir, &bundle_ref)?;
    run_tamper(&dir, &bundle_dir, &fixture)?;
    run_search(&dir, &registry, &ledger_root, &fixture)?;
    run_mcp(&dir, &registry, &ledger_root, &fixture)?;
    Ok(())
}

fn run_explain(
    retention_root: &std::path::Path,
    dir: &std::path::Path,
    fixture: &RetentionGcCatalogFixture,
) -> CliResult<(std::path::PathBuf, String)> {
    let explain_path = dir.join("retention-explain.preserves");
    let explain_output = molten_cmd()
        .args(["test", "retention", "explain", "--root"])
        .arg(retention_root)
        .args(["--object-ref"])
        .arg(&fixture.object_ref)
        .args([
            "--object-kind",
            "artifact",
            "--retention-class",
            molten::retention::CLASS_PUBLIC_ARTIFACT,
            "--action",
            molten::retention::ACTION_DELETE,
            "--subsystem",
            "ledger-gc",
            "--out",
        ])
        .arg(&explain_path)
        .output()?;
    assert_success(&explain_output, "retention explain candidate");
    assert!(stdout(&explain_output).contains("retention explain ref="));
    let explain = molten::retention::parse_candidate_explain(&read_preserves(&explain_path)?)?;
    assert_eq!(explain.object_ref, fixture.object_ref);
    assert_eq!(explain.admission_refs.len(), 4);
    assert_eq!(explain.gc_plan_refs, vec![fixture.plan_ref.clone()]);
    assert_eq!(explain.gc_apply_refs, vec![fixture.apply_ref.clone()]);
    assert_eq!(explain.gc_execution_refs, vec![fixture.execution_ref.clone()]);
    assert_eq!(explain.gc_audit_refs, vec![fixture.audit_ref.clone()]);
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&explain_path)?), "retention-candidate-explain");
    Ok((explain_path, explain.explain_ref))
}

fn run_bundle(
    retention_root: &std::path::Path,
    dir: &std::path::Path,
    explain_path: &std::path::Path,
    explain_ref: &str,
) -> CliResult<(std::path::PathBuf, String)> {
    let bundle_dir = dir.join("retention-bundle");
    let bundle_output = molten_cmd()
        .args(["test", "retention", "bundle-export", "--root"])
        .arg(retention_root)
        .args(["--explain"])
        .arg(explain_path)
        .args(["--out"])
        .arg(&bundle_dir)
        .args(["--profile", "public"])
        .output()?;
    assert_success(&bundle_output, "retention bundle export");
    assert!(stderr(&bundle_output).contains("retention bundle ref="));
    let bundle_value = read_preserves(&bundle_dir.join("bundle.preserves"))?;
    let bundle = molten::retention::parse_candidate_bundle(&bundle_value)?;
    assert_eq!(molten::ledger::artifact_kind(&bundle_value), "retention-candidate-bundle");
    assert_eq!(bundle.explain_ref, explain_ref);
    assert_eq!(bundle.artifact_refs.len(), 6);
    assert!(bundle.diagnostics.is_empty());
    assert!(bundle_dir.join("explain.preserves").exists());
    assert!(bundle_dir.join("artifacts/gc-plans").exists());
    assert!(bundle_dir.join("artifacts/gc-audits").exists());
    Ok((bundle_dir, bundle.bundle_ref))
}

fn check_profile(
    registry: &std::path::Path,
    ledger_root: &std::path::Path,
    bundle_dir: &std::path::Path,
) -> CliResult<()> {
    let bundle_profile = molten::retention::parse_candidate_bundle_profile(&read_preserves(
        &bundle_dir.join("bundle-profile.preserves"),
    )?)?;
    assert_eq!(bundle_profile.profile, "public");
    assert_eq!(bundle_profile.decision, "pass");
    assert!(bundle_profile.marker_refs.is_empty());
    let bundle_profile_path = bundle_dir.join("bundle-profile.preserves");
    let profile_import = molten_cmd()
        .args(["test", "ledger", "import"])
        .arg(&bundle_profile_path)
        .args(["--ledger"])
        .arg(ledger_root)
        .output()?;
    assert_success(&profile_import, "ledger import retention bundle profile");
    let profile_search = molten_cmd()
        .args(["test", "catalog", "search", "--registry"])
        .arg(registry)
        .args(["--ledger"])
        .arg(ledger_root)
        .args([
            "--ledger-kind",
            "retention-candidate-bundle-profile",
            "--text",
            "retention-candidate:bundle-profile",
        ])
        .output()?;
    assert_success(&profile_search, "catalog search retention bundle profile");
    assert!(stdout(&profile_search).contains("retention-candidate:bundle-profile"));
    Ok(())
}
