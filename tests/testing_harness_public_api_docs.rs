use std::process::Command;

#[test]
fn public_api_boundary_docs_name_structured_diagnostics_and_guard() {
    let docs = include_str!("../docs/testing-harness-public-api.md");

    assert!(docs.contains("InventoryCheckReport"));
    assert!(docs.contains("InventoryCheckDiagnostic"));
    assert!(docs.contains("cargo run -p aspen-testing --bin aspen-test-harness -- check --json"));
    assert!(docs.contains("scripts/test-harness.sh public-api-boundary"));
    assert!(docs.contains("scripts/test-harness.sh quick-confidence"));
    assert!(docs.contains("without pulling runtime-host, patchbay, madsim"));
    assert!(docs.contains("madsim"));
    assert!(docs.contains("Raft"));
}

#[test]
fn public_api_boundary_check_reports_clean_default_graph() {
    let repo_root = env!("CARGO_MANIFEST_DIR");
    let output = Command::new("scripts/test-harness.sh")
        .arg("public-api-boundary")
        .current_dir(repo_root)
        .output()
        .expect("run public API boundary check");

    assert!(
        output.status.success(),
        "boundary check failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    assert!(stdout.contains("\"status\": \"passed\""));
    assert!(stdout.contains("\"leaked_packages\": []"));
    assert!(stdout.contains("aspen-testing-network"));
    assert!(stdout.contains("madsim"));
}
