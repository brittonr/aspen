const FABRIC_TRANSPORT_CHILD_TIMEOUT_MS: &str = "30000";
const DISTINCT_PROCESS_PAYLOAD_TEXT: &str = "distinct-process-bounded-frame";

fn read_canonical_preserves(path: &std::path::Path) -> CliResult<preserves::IOValue> {
    Ok(molten::preserves_rail::parse_canonical_bytes(&std::fs::read(path)?)?)
}

// r[verify molten.fabric_transport.distinct_process_evidence]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn cli_fabric_transport_runs_distinct_children_and_verifies_offline() -> CliResult<()> {
    let root = temp_dir("cli-fabric-transport-distinct-process")?;
    let run_dir = root.join("run");
    let run = molten_cmd()
        .args(["cluster", "fabric-transport-run", "--run-dir"])
        .arg(&run_dir)
        .args(["--child-timeout-ms", FABRIC_TRANSPORT_CHILD_TIMEOUT_MS])
        .output()?;
    assert_success(&run, "fabric transport distinct-process run");
    assert!(stdout(&run).contains("decision=pass"));
    for artifact in [
        "artifact-index.tsv",
        "endpoint-handoff.preserves",
        "listener-start.preserves",
        "client-start.preserves",
        "listener-terminal.preserves",
        "client-terminal.preserves",
        "cleanup.preserves",
        "parent-run.preserves",
        "verification.preserves",
        "logs/listener.log",
        "logs/client.log",
    ] {
        assert!(run_dir.join(artifact).is_file(), "missing distinct-process artifact {artifact}");
    }
    let listener_start = read_canonical_preserves(&run_dir.join("listener-start.preserves"))?;
    let client_start = read_canonical_preserves(&run_dir.join("client-start.preserves"))?;
    assert_ne!(
        molten::preserves_rail::canonical_hash(&listener_start)?,
        molten::preserves_rail::canonical_hash(&client_start)?
    );
    let parent = read_canonical_preserves(&run_dir.join("parent-run.preserves"))?;
    let parent_text = molten::preserves_rail::to_text(&parent)?;
    assert!(parent_text.contains("parent-observed-distinct-child-handles"));
    assert!(parent_text.contains("same-process-loopback-insufficient"));
    assert!(!parent_text.contains(DISTINCT_PROCESS_PAYLOAD_TEXT));
    assert!(!parent_text.contains("ip:127.0.0.1"));
    assert!(!parent_text.contains("iroh::Connection"));

    let verify = molten_cmd()
        .args(["cluster", "fabric-transport-verify", "--run-dir"])
        .arg(&run_dir)
        .output()?;
    assert_success(&verify, "fabric transport distinct-process offline verification");
    assert!(stdout(&verify).contains("decision=pass"));
    Ok(())
}

// r[verify molten.fabric_transport.distinct_process_evidence]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn cli_fabric_transport_verifier_denies_child_only_or_tampered_parent_observations() -> CliResult<()> {
    let root = temp_dir("cli-fabric-transport-child-only-denial")?;
    let run_dir = root.join("run");
    let run = molten_cmd()
        .args(["cluster", "fabric-transport-run", "--run-dir"])
        .arg(&run_dir)
        .args(["--child-timeout-ms", FABRIC_TRANSPORT_CHILD_TIMEOUT_MS])
        .output()?;
    assert_success(&run, "fabric transport run before parent-observation tamper");

    let client_start = std::fs::read(run_dir.join("client-start.preserves"))?;
    std::fs::write(run_dir.join("listener-start.preserves"), client_start)?;
    let verify = molten_cmd()
        .args(["cluster", "fabric-transport-verify", "--run-dir"])
        .arg(&run_dir)
        .output()?;
    assert_failure(&verify, "fabric transport child-only claim denial");
    let error = stderr(&verify);
    assert!(
        error.contains("parent-start-missing")
            || error.contains("parent-run-artifact-mismatch")
            || error.contains("artifact-index-mismatch")
    );
    Ok(())
}

// r[verify molten.fabric_transport.distinct_process_evidence]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn cli_fabric_transport_spawn_failure_exports_redacted_nonpass_evidence() -> CliResult<()> {
    let root = temp_dir("cli-fabric-transport-spawn-failure")?;
    let run_dir = root.join("run");
    let missing_binary = root.join("missing-molten-binary");
    let run = molten_cmd()
        .args(["cluster", "fabric-transport-run", "--run-dir"])
        .arg(&run_dir)
        .args(["--process-binary"])
        .arg(&missing_binary)
        .args(["--child-timeout-ms", FABRIC_TRANSPORT_CHILD_TIMEOUT_MS])
        .output()?;
    assert_failure(&run, "fabric transport child spawn failure");
    let failure = read_canonical_preserves(&run_dir.join("failure.preserves"))?;
    let failure_text = molten::preserves_rail::to_text(&failure)?;
    assert!(failure_text.contains("deny"));
    assert!(failure_text.contains("owned-child-lifetimes-scope-bound"));
    assert!(failure_text.contains("cleanup-success-not-claimed"));
    assert!(failure_text.contains("failure-does-not-establish-process-separation"));
    assert!(!failure_text.contains("missing-molten-binary"));
    Ok(())
}
