
fn run_verify(
    registry: &std::path::Path,
    ledger_root: &std::path::Path,
    dir: &std::path::Path,
    bundle_dir: &std::path::Path,
    bundle_ref: &str,
) -> CliResult<()> {
    let verify_path = dir.join("retention-bundle-verify.preserves");
    let verify_output = molten_cmd()
        .args(["test", "retention", "bundle-verify", "--bundle"])
        .arg(bundle_dir)
        .args(["--receipt-out"])
        .arg(&verify_path)
        .output()?;
    assert_success(&verify_output, "retention bundle verify");
    assert!(stderr(&verify_output).contains("retention bundle verify ref="));
    let verify_value = read_preserves(&verify_path)?;
    let verify = molten::retention::parse_candidate_bundle_verify(&verify_value)?;
    assert_eq!(molten::ledger::artifact_kind(&verify_value), "retention-candidate-bundle-verify");
    assert_eq!(verify.decision, "pass");
    assert_eq!(verify.bundle_ref, bundle_ref);
    assert_eq!(verify.file_refs.len(), 6);
    assert!(verify.diagnostics.is_empty());
    let verify_import = molten_cmd()
        .args(["test", "ledger", "import"])
        .arg(&verify_path)
        .args(["--ledger"])
        .arg(ledger_root)
        .output()?;
    assert_success(&verify_import, "ledger import retention bundle verify");
    let verify_search = molten_cmd()
        .args(["test", "catalog", "search", "--registry"])
        .arg(registry)
        .args(["--ledger"])
        .arg(ledger_root)
        .args([
            "--ledger-kind",
            "retention-candidate-bundle-verify",
            "--text",
            "retention-candidate:bundle-verify",
        ])
        .output()?;
    assert_success(&verify_search, "catalog search retention bundle verify");
    let verify_search_stdout = stdout(&verify_search);
    assert!(verify_search_stdout.contains("retention-candidate:bundle-verify"));
    assert!(verify_search_stdout.contains(&verify.verify_ref));
    Ok(())
}

fn run_tamper(
    dir: &std::path::Path,
    bundle_dir: &std::path::Path,
    fixture: &RetentionGcCatalogFixture,
) -> CliResult<()> {
    let tampered_plan_path = bundle_dir
        .join("artifacts/gc-plans")
        .join(format!("{}.preserves", fixture.plan_ref.replace(':', "_")));
    std::fs::write(
        &tampered_plan_path,
        molten::preserves_rail::to_text(&molten::preserves_rail::record("tampered", vec![
            molten::preserves_rail::string("plan"),
        ]))?,
    )?;
    let tampered_path = dir.join("retention-bundle-verify-tampered.preserves");
    let tampered_output = molten_cmd()
        .args(["test", "retention", "bundle-verify", "--bundle"])
        .arg(bundle_dir)
        .args(["--receipt-out"])
        .arg(&tampered_path)
        .output()?;
    assert_success(&tampered_output, "retention bundle verify tampered");
    let tampered = molten::retention::parse_candidate_bundle_verify(&read_preserves(&tampered_path)?)?;
    assert_eq!(tampered.decision, "deny");
    assert!(
        tampered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("retention-bundle-tampered-file:gc-plans"))
    );
    Ok(())
}

fn run_search(
    dir: &std::path::Path,
    registry: &std::path::Path,
    ledger_root: &std::path::Path,
    fixture: &RetentionGcCatalogFixture,
) -> CliResult<()> {
    let search_receipt = dir.join("catalog-search-receipt.preserves");
    let search_output = molten_cmd()
        .args(["test", "catalog", "search", "--registry"])
        .arg(registry)
        .args(["--ledger"])
        .arg(ledger_root)
        .args(["--text"])
        .arg(format!("retention-gc-object:{}", fixture.object_ref))
        .args(["--receipt-out"])
        .arg(&search_receipt)
        .output()?;
    assert_success(&search_output, "catalog search retention GC object");
    let search_stdout = stdout(&search_output);
    assert!(search_stdout.contains("retention-gc:plan"));
    assert!(search_stdout.contains("retention-gc:apply"));
    assert!(search_stdout.contains("retention-gc:execute"));
    assert!(search_stdout.contains("retention-gc:audit"));
    assert!(search_stdout.contains(&fixture.plan_ref));
    assert!(search_stdout.contains(&fixture.apply_ref));
    assert!(search_stdout.contains(&fixture.execution_ref));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&search_receipt)?), "catalog-receipt");

    let audit_search = molten_cmd()
        .args(["test", "catalog", "search", "--registry"])
        .arg(registry)
        .args(["--ledger"])
        .arg(ledger_root)
        .args(["--ledger-kind", "retention-gc-audit", "--text", "retention-gc:audit"])
        .output()?;
    assert_success(&audit_search, "catalog search retention GC audit ledger kind");
    let audit_search_stdout = stdout(&audit_search);
    assert!(audit_search_stdout.contains("retention-gc:audit"));
    assert!(audit_search_stdout.contains(&fixture.audit_ref));
    Ok(())
}

fn run_mcp(
    dir: &std::path::Path,
    registry: &std::path::Path,
    ledger_root: &std::path::Path,
    fixture: &RetentionGcCatalogFixture,
) -> CliResult<()> {
    let mcp_request_path = dir.join("retention-gc-search-request.preserves");
    let mcp_response_path = dir.join("retention-gc-search-response.preserves");
    let mcp_receipt_path = dir.join("retention-gc-search-mcp-receipt.preserves");
    let mcp_request = molten::catalog_mcp::mcp_request_value("search_retention_gc", vec![
        molten::preserves_rail::record("stage", vec![molten::preserves_rail::string("audit")]),
        molten::preserves_rail::record("object-ref", vec![molten::preserves_rail::string(&fixture.object_ref)]),
        molten::preserves_rail::record("subsystem", vec![molten::preserves_rail::string("ledger-gc")]),
        molten::preserves_rail::record("execution-ref", vec![molten::preserves_rail::string(&fixture.execution_ref)]),
    ])?;
    std::fs::write(&mcp_request_path, molten::preserves_rail::to_text(&mcp_request)?)?;
    let mcp_output = molten_cmd()
        .args(["test", "catalog", "mcp-call"])
        .arg(&mcp_request_path)
        .args(["--registry"])
        .arg(registry)
        .args(["--ledger"])
        .arg(ledger_root)
        .args(["--out"])
        .arg(&mcp_response_path)
        .args(["--receipt-out"])
        .arg(&mcp_receipt_path)
        .output()?;
    assert_success(&mcp_output, "catalog MCP search_retention_gc");
    let mcp_response = std::fs::read_to_string(&mcp_response_path)?;
    assert!(mcp_response.contains("retention-gc:audit"));
    assert!(mcp_response.contains(&fixture.execution_ref));
    let mcp_receipt = molten::catalog_mcp::parse_mcp_receipt(&read_preserves(&mcp_receipt_path)?)?;
    assert_eq!(mcp_receipt.tool, "search_retention_gc");
    assert_eq!(mcp_receipt.decision, "pass");
    Ok(())
}

struct CandidateInput<'a> {
    root: &'a std::path::Path,
    label: &'a str,
    object_ref: String,
    object_kind: &'a str,
    retention_class: &'a str,
    action: &'a str,
}

struct RetentionCliCandidate {
    root: std::path::PathBuf,
    object_ref: String,
    object_kind: String,
    retention_class: String,
    action: String,
    requester_ref: String,
    policy_ref: String,
    authority_ref: String,
    support_ref: String,
    index_ref: String,
}

struct RetentionAdmissionInput<'a> {
    candidate: &'a RetentionCliCandidate,
    kind: &'a str,
    label: &'a str,
}

struct RetentionGcCatalogFixture {
    object_ref: String,
    plan_ref: String,
    apply_ref: String,
    execution_ref: String,
    audit_ref: String,
}

fn setup_gc_catalog_fixture(
    candidate: &RetentionCliCandidate,
    subsystem: &str,
    dir: &std::path::Path,
) -> CliResult<RetentionGcCatalogFixture> {
    let plan_path = dir.join("catalog-retention-plan.preserves");
    let plan = run_gc_plan_cli(candidate, subsystem, &plan_path)?;
    let apply_path = dir.join("catalog-retention-apply.preserves");
    let apply_output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&candidate.root)
        .args(["--plan-ref"])
        .arg(&plan.plan_ref)
        .args(["--receipt-out"])
        .arg(&apply_path)
        .output()?;
    assert_success(&apply_output, "retention gc-apply-plan catalog fixture");
    let apply = molten::retention::parse_gc_apply(&read_preserves(&apply_path)?)?;
    assert_eq!(apply.decision, "pass");
    let execution = molten::retention::store_gc_execution_gate(molten::retention::GcExecutionGateInput {
        root: &candidate.root,
        subsystem,
        action: &candidate.action,
        object_ref: &candidate.object_ref,
        object_kind: &candidate.object_kind,
        retention_class: &candidate.retention_class,
        apply_ref: Some(&apply.apply_ref),
    })?;
    assert_eq!(execution.decision, "pass");
    let execution_path = dir.join("catalog-retention-execution.preserves");
    std::fs::write(&execution_path, molten::preserves_rail::to_text(&execution.value)?)?;
    let audit = molten::retention::audit_gc_execution(molten::retention::GcAuditInput {
        root: &candidate.root,
        execution_ref: &execution.execution_ref,
    })?;
    assert_eq!(audit.decision, "pass");
    let audit_path = dir.join("catalog-retention-audit.preserves");
    std::fs::write(&audit_path, molten::preserves_rail::to_text(&audit.value)?)?;
    let ledger_root = dir.join("ledger");
    for artifact in [&plan_path, &apply_path, &execution_path, &audit_path] {
        let output = molten_cmd()
            .args(["test", "ledger", "import"])
            .arg(artifact)
            .args(["--ledger"])
            .arg(&ledger_root)
            .output()?;
        assert_success(&output, "ledger import retention GC catalog fixture");
    }
    Ok(RetentionGcCatalogFixture {
        object_ref: candidate.object_ref.clone(),
        plan_ref: plan.plan_ref,
        apply_ref: apply.apply_ref,
        execution_ref: execution.execution_ref,
        audit_ref: audit.audit_ref,
    })
}

fn setup_retention_cli_candidate(input: CandidateInput<'_>) -> CliResult<RetentionCliCandidate> {
    let requester_ref = test_ref(&format!("{}-requester", input.label))?;
    let mut candidate = RetentionCliCandidate {
        root: input.root.to_path_buf(),
        object_ref: input.object_ref,
        object_kind: input.object_kind.to_string(),
        retention_class: input.retention_class.to_string(),
        action: input.action.to_string(),
        requester_ref,
        policy_ref: String::new(),
        authority_ref: String::new(),
        support_ref: String::new(),
        index_ref: String::new(),
    };
    candidate.policy_ref = store_retention_cli_admission(RetentionAdmissionInput {
        candidate: &candidate,
        kind: molten::retention::ADMISSION_KIND_POLICY,
        label: "policy",
    })?;
    candidate.authority_ref = store_retention_cli_admission(RetentionAdmissionInput {
        candidate: &candidate,
        kind: molten::retention::ADMISSION_KIND_AUTHORITY,
        label: "authority",
    })?;
    candidate.support_ref = store_retention_cli_admission(RetentionAdmissionInput {
        candidate: &candidate,
        kind: molten::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
        label: "support",
    })?;
    candidate.index_ref = store_retention_cli_admission(RetentionAdmissionInput {
        candidate: &candidate,
        kind: molten::retention::ADMISSION_KIND_REFERENCE_INDEX,
        label: "index",
    })?;
    Ok(candidate)
}
