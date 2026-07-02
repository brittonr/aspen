
#[test]
fn cli_octet_artifacts_imports_raw_artifacts_to_ledger() -> CliResult<()> {
    let dir = temp_dir("cli-octet-artifacts-import")?;
    let artifacts = dir.join("artifacts");
    let ledger_root = dir.join("ledger");
    let receipt = dir.join("octet-artifact-ledger.preserves");
    std::fs::create_dir_all(&artifacts)?;
    write_octet_artifacts(&artifacts)?;

    let imported = molten_cmd()
        .args(["test", "octet", "artifacts", "import", "--artifacts"])
        .arg(&artifacts)
        .args(["--ledger"])
        .arg(&ledger_root)
        .args(["--receipt-out"])
        .arg(&receipt)
        .output()?;

    assert_success(&imported, "octet artifacts import");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&receipt)?), "octet-artifact-ledger-receipt");
    let entries = molten::ledger::list_artifacts(&ledger_root)?;
    let mut kinds = Vec::with_capacity(entries.len());
    for entry in entries {
        let value = molten::ledger::read_artifact(&ledger_root, &entry.artifact_ref)?;
        kinds.push(molten::ledger::artifact_kind(&value).to_string());
    }
    assert!(kinds.iter().any(|kind| kind == "octet-status-artifact"));
    assert!(kinds.iter().any(|kind| kind == "octet-object-corpus-artifact"));
    assert!(kinds.iter().any(|kind| kind == "octet-fingerprint-evidence"));
    Ok(())
}

#[test]
fn cli_octet_gate_writes_canonical_deny_receipt_for_warning_only() -> CliResult<()> {
    let dir = temp_dir("cli-octet-deny")?;
    let receipt = dir.join("octet-gate.preserves");
    write_octet_artifacts(&dir)?;

    let denied = molten_cmd()
        .args(["test", "octet", "gate", "--artifacts"])
        .arg(&dir)
        .args(["--profile", "strict-ci", "--receipt-out"])
        .arg(&receipt)
        .output()?;

    assert_failure(&denied, "warning-only octet gate");
    assert!(stdout(&denied).contains("octet gate receipt blake3:"));
    assert!(stderr(&denied).contains("octet gate denied"));
    let receipt_value = read_preserves(&receipt)?;
    assert_eq!(molten::ledger::artifact_kind(&receipt_value), "octet-gate-receipt");
    let receipt_text = molten::preserves_rail::to_text(&receipt_value)?;
    assert!(receipt_text.contains("<decision \"deny\">"));
    assert!(receipt_text.contains("warning-only"));
    Ok(())
}

#[test]
fn cli_gc_plan_lists_gates_before_mutation() -> CliResult<()> {
    let dir = temp_dir("cli-retention-gc-plan")?;
    let root = dir.join("retention-state");
    let plan_path = dir.join("plan.preserves");
    let apply_path = dir.join("apply.preserves");
    let refs = build_refs(&root)?;
    let plan = run_plan(&root, &refs, &plan_path)?;

    let show = molten_cmd().args(["test", "retention", "show"]).arg(&plan_path).output()?;
    assert_success(&show, "retention show gc-plan");
    assert!(stdout(&show).contains("retention gc plan"));

    let apply = run_apply(&root, &plan, &apply_path)?;
    assert_eq!(apply.plan_ref, plan.plan_ref);
    assert!(apply.retention_receipt_ref.is_some());
    assert!(apply.tombstone_ref.is_some());
    Ok(())
}

struct Refs {
    requester: String,
    object: String,
    peer: String,
    remote: String,
    policy: String,
    authority: String,
    support: String,
    index: String,
    remote_gc: String,
    clearance: String,
}

fn build_refs(root: &std::path::Path) -> CliResult<Refs> {
    let mut refs = Refs {
        requester: test_ref("retention-plan-requester")?,
        object: test_ref("retention-plan-object")?,
        peer: test_ref("retention-plan-peer")?,
        remote: test_ref("retention-plan-remote")?,
        policy: String::new(),
        authority: String::new(),
        support: String::new(),
        index: String::new(),
        remote_gc: String::new(),
        clearance: String::new(),
    };
    refs.policy = admission(root, &refs, molten::retention::ADMISSION_KIND_POLICY, "retention-plan-policy", &[])?;
    refs.authority =
        admission(root, &refs, molten::retention::ADMISSION_KIND_AUTHORITY, "retention-plan-authority", &[])?;
    refs.support =
        admission(root, &refs, molten::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE, "retention-plan-support", &[])?;
    refs.index =
        admission(root, &refs, molten::retention::ADMISSION_KIND_REFERENCE_INDEX, "retention-plan-index", &[])?;
    refs.remote_gc = admission(
        root,
        &refs,
        molten::retention::ADMISSION_KIND_REMOTE_GC,
        "retention-plan-remote-gc",
        std::slice::from_ref(&refs.remote),
    )?;
    refs.clearance = clearance(root, &refs)?;
    Ok(refs)
}

fn admission(
    root: &std::path::Path,
    refs: &Refs,
    kind: &str,
    label: &str,
    remote_refs: &[String],
) -> CliResult<String> {
    Ok(molten::retention::store_evidence_admission(root, &molten::retention::EvidenceAdmissionInput {
        kind,
        decision: "pass",
        requester_ref: &refs.requester,
        object_ref: &refs.object,
        object_kind: "chunk",
        retention_class: molten::retention::CLASS_DURABLE_VALUE,
        action: molten::retention::ACTION_DELETE,
        bound_refs: &[test_ref(label)?],
        retained_refs: &[],
        remote_refs,
        is_reference_index_complete: true,
        is_current: true,
        revoked_refs: &[],
        diagnostics: &[],
    })?
    .admission_ref)
}

fn clearance(root: &std::path::Path, refs: &Refs) -> CliResult<String> {
    Ok(molten::retention::store_remote_gc_clearance(root, &molten::retention::RemoteGcClearanceInput {
        decision: "pass",
        requester_ref: &refs.requester,
        peer_ref: &refs.peer,
        object_ref: &refs.object,
        object_kind: "chunk",
        retention_class: molten::retention::CLASS_DURABLE_VALUE,
        action: molten::retention::ACTION_DELETE,
        remote_ref: &refs.remote,
        policy_ref: &refs.policy,
        authority_ref: &refs.authority,
        evidence_refs: std::slice::from_ref(&refs.support),
        retained_refs: &[],
        is_current: true,
        revoked_refs: &[],
        diagnostics: &[],
    })?
    .clearance_ref)
}

fn run_plan(root: &std::path::Path, refs: &Refs, out: &std::path::Path) -> CliResult<molten::retention::GcPlan> {
    let mut command = molten_cmd();
    command
        .args(["test", "retention", "gc-plan", "--root"])
        .arg(root)
        .args(["--subsystem", "ledger-gc", "--object-ref"])
        .arg(&refs.object)
        .args([
            "--object-kind",
            "chunk",
            "--retention-class",
            molten::retention::CLASS_DURABLE_VALUE,
            "--action",
            "delete",
        ]);
    add_refs(&mut command, refs);
    command.args(["--out"]).arg(out);
    let output = command.output()?;
    assert_success(&output, "retention gc-plan");
    assert!(stdout(&output).contains("retention gc plan ref="));
    let value = read_preserves(out)?;
    assert_eq!(molten::ledger::artifact_kind(&value), "retention-gc-plan");
    let plan = molten::retention::parse_gc_plan(&value)?;
    assert_eq!(plan.decision, "pass");
    assert!(plan.gates.iter().any(|gate| gate.name == "remote-clearance" && gate.decision == "pass"));
    Ok(plan)
}

fn add_refs(command: &mut std::process::Command, refs: &Refs) {
    command
        .args(["--retention-requester"])
        .arg(&refs.requester)
        .args(["--retention-policy-ref"])
        .arg(&refs.policy)
        .args(["--retention-authority-ref"])
        .arg(&refs.authority)
        .args(["--retention-evidence-ref"])
        .arg(&refs.support)
        .args(["--retention-remote-peer-ref"])
        .arg(&refs.peer)
        .args(["--retention-remote-ref"])
        .arg(&refs.remote)
        .args(["--retention-reference-index-ref"])
        .arg(&refs.index)
        .args(["--retention-remote-gc-ref"])
        .arg(&refs.remote_gc)
        .args(["--retention-remote-clearance-ref"])
        .arg(&refs.clearance)
        .args(["--retention-reference-index-complete"]);
}

fn run_apply(
    root: &std::path::Path,
    plan: &molten::retention::GcPlan,
    out: &std::path::Path,
) -> CliResult<molten::retention::GcApply> {
    let output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(root)
        .args(["--plan-ref"])
        .arg(&plan.plan_ref)
        .args(["--receipt-out"])
        .arg(out)
        .output()?;
    assert_success(&output, "retention gc-apply-plan");
    assert!(stdout(&output).contains("retention gc apply ref="));
    let value = read_preserves(out)?;
    assert_eq!(molten::ledger::artifact_kind(&value), "retention-gc-apply");
    let apply = molten::retention::parse_gc_apply(&value)?;
    assert_eq!(apply.decision, "pass");
    Ok(apply)
}

#[test]
fn cli_gc_negative_regression_matrix() -> CliResult<()> {
    let dir = temp_dir("cli-retention-gc-negative")?;
    missing_plan_case(&dir)?;
    stale_plan_case(&dir)?;
    missing_apply_case(&dir)?;
    wrong_apply_case(&dir)?;
    audit_case(&dir)?;
    Ok(())
}

fn missing_plan_case(dir: &std::path::Path) -> CliResult<()> {
    let root = dir.join("missing-plan-root");
    let missing_plan_ref = test_ref("retention-missing-plan")?;
    let output = molten_cmd()
        .args(["test", "retention", "gc-apply-plan", "--root"])
        .arg(&root)
        .args(["--plan-ref"])
        .arg(&missing_plan_ref)
        .args(["--receipt-out"])
        .arg(dir.join("missing-plan-apply.preserves"))
        .output()?;
    assert_failure(&output, "retention apply missing plan ref");
    Ok(())
}
