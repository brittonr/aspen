
fn store_retention_cli_admission(input: RetentionAdmissionInput<'_>) -> CliResult<String> {
    Ok(molten::retention::store_evidence_admission(
        &input.candidate.root,
        &molten::retention::EvidenceAdmissionInput {
            kind: input.kind,
            decision: "pass",
            requester_ref: &input.candidate.requester_ref,
            object_ref: &input.candidate.object_ref,
            object_kind: &input.candidate.object_kind,
            retention_class: &input.candidate.retention_class,
            action: &input.candidate.action,
            bound_refs: &[test_ref(&format!("{}-{}", input.candidate.object_ref, input.label))?],
            retained_refs: &[],
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        },
    )?
    .admission_ref)
}

fn run_gc_plan_cli(
    candidate: &RetentionCliCandidate,
    subsystem: &str,
    out: &std::path::Path,
) -> CliResult<molten::retention::GcPlan> {
    let mut command = molten_cmd();
    command
        .args(["test", "retention", "gc-plan", "--root"])
        .arg(&candidate.root)
        .args(["--subsystem", subsystem, "--object-ref"])
        .arg(&candidate.object_ref)
        .args(["--object-kind"])
        .arg(&candidate.object_kind)
        .args(["--retention-class"])
        .arg(&candidate.retention_class)
        .args(["--action"])
        .arg(&candidate.action)
        .args(["--out"])
        .arg(out);
    add_retention_args(&mut command, candidate);
    let output = command.output()?;
    assert_success(&output, "retention gc-plan regression fixture");
    let plan = molten::retention::parse_gc_plan(&read_preserves(out)?)?;
    assert_eq!(plan.decision, "pass");
    Ok(plan)
}

fn add_retention_args(command: &mut std::process::Command, candidate: &RetentionCliCandidate) {
    command
        .args(["--retention-requester"])
        .arg(&candidate.requester_ref)
        .args(["--retention-policy-ref"])
        .arg(&candidate.policy_ref)
        .args(["--retention-authority-ref"])
        .arg(&candidate.authority_ref)
        .args(["--retention-evidence-ref"])
        .arg(&candidate.support_ref)
        .args(["--retention-reference-index-ref"])
        .arg(&candidate.index_ref)
        .args(["--retention-reference-index-complete"]);
}

fn test_ref(label: &str) -> CliResult<String> {
    Ok(molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("cli-test-ref", vec![
        molten::preserves_rail::string(label),
    ]))?)
}

fn molten_cmd() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_molten"));
    command.current_dir(manifest_dir());
    command
}

/// The variable naming the independently provisioned source trust; each signed test captures it.
const SOURCE_TRUST_ENV: &str = "MOLTEN_TEST_SOURCE_TRUST";

fn source_gated_node_command(
    source_trust: Option<&std::ffi::OsStr>,
    root: &std::path::Path,
) -> CliResult<std::process::Command> {
    signed_molten_command(source_trust, &[root])
}

/// Every root that performs startup admission receives the same independently selected trust.
fn signed_molten_command(
    source_trust: Option<&std::ffi::OsStr>,
    roots: &[&std::path::Path],
) -> CliResult<std::process::Command> {
    let selected = source_trust.ok_or_else(|| {
        std::io::Error::other("real startup tests require independently provisioned MOLTEN_TEST_SOURCE_TRUST")
    })?;
    let bytes = std::fs::read(selected)?;
    let trust: serde_json::Value = serde_json::from_slice(&bytes)?;
    let output = trust.get("output").and_then(serde_json::Value::as_str)
        .ok_or_else(|| std::io::Error::other("real source trust has no producer output"))?;
    for root in roots {
        std::fs::write(root.join("source-gate-trust.json"), &bytes)?;
    }
    let mut command = std::process::Command::new(std::path::Path::new(output).join("bin/molten"));
    command.current_dir(manifest_dir());
    Ok(command)
}

struct StartArgs<'a> {
    source_trust: Option<&'a std::ffi::OsStr>,
    root: &'a std::path::Path,
    config: &'a std::path::Path,
    startup: &'a std::path::Path,
}

fn start_case(args: StartArgs<'_>) -> CliResult<()> {
    let init = molten_cmd()
        .args(["node", "init", "--state-root"])
        .arg(args.root)
        .args(["--node-id", "node:cli", "--config-out"])
        .arg(args.config)
        .output()?;
    assert_success(&init, "node init");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(args.config)?), "node-config");

    let run = source_gated_node_command(args.source_trust, args.root)?
        .args(["node", "run", "--state-root"])
        .arg(args.root)
        .args(["--startup-out"])
        .arg(args.startup)
        .output()?;
    assert_success(&run, "node run");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(args.startup)?), "node-startup-receipt");
    Ok(())
}

struct OpArgs<'a> {
    name: &'a str,
    out: &'a std::path::Path,
    authority_ref: &'a str,
    policy_ref: &'a str,
    resource_ref: &'a str,
    label: &'a str,
}

fn write_op(args: OpArgs<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["node", "control-request", "--operation"])
        .arg(args.name)
        .args(["--authority"])
        .arg(args.authority_ref)
        .args(["--policy"])
        .arg(args.policy_ref)
        .args(["--resource"])
        .arg(args.resource_ref)
        .args(["--out"])
        .arg(args.out)
        .output()?;
    assert_success(&output, args.label);
    Ok(())
}

fn submit_op(
    root: &std::path::Path,
    request: &std::path::Path,
    receipt: &std::path::Path,
    label: &str,
) -> CliResult<()> {
    let output = molten_cmd()
        .args(["node", "control-submit", "--state-root"])
        .arg(root)
        .arg(request)
        .args(["--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, label);
    Ok(())
}

fn dispatch_op(root: &std::path::Path, receipt: &std::path::Path, label: &str) -> CliResult<()> {
    let output = molten_cmd()
        .args(["node", "control-dispatch", "--state-root"])
        .arg(root)
        .args(["--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, label);
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-receipt");
    Ok(())
}
