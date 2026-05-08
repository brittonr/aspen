use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn quick_confidence_docs_name_command_and_boundaries() {
    let docs = include_str!("../docs/quick-confidence-rail.md");

    assert!(docs.contains("scripts/test-harness.sh quick-confidence"));
    assert!(docs.contains("target/quick-confidence/summary.json"));
    assert!(docs.contains("scripts/test-harness.sh check"));
    assert!(docs.contains("scripts/test-harness.sh runtime-host-acceptance-bundle"));
    assert!(docs.contains("scripts/test-harness.sh public-api-boundary"));
    assert!(docs.contains("cargo test --test operator_receipts_docs -- --nocapture"));
    assert!(docs.contains("cargo test --test runtime_host_readiness_docs -- --nocapture"));
    assert!(docs.contains("openspec validate --all --strict --json"));
    assert!(docs.contains("git diff --check"));
    assert!(docs.contains("does **not** prove"));
    assert!(docs.contains("nix run .#dogfood-local -- full"));
    assert!(docs.contains("KVM/NixOS VM runtime-host execution"));
    assert!(docs.contains("Uhyve/Hermit runtime-host execution"));
    assert!(docs.contains("Hyperlight runtime-host execution"));
}

#[test]
fn quick_confidence_dry_run_summary_is_structured_and_non_proof() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let summary_path = repo_root.join("target").join("quick-confidence-test").join(format!(
        "summary-{}-{}.json",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));

    let output = Command::new("scripts/test-harness.sh")
        .arg("quick-confidence")
        .arg("--dry-run")
        .arg("--json")
        .arg("--summary")
        .arg(&summary_path)
        .current_dir(&repo_root)
        .output()
        .expect("run quick confidence dry-run");

    assert!(
        output.status.success(),
        "dry-run failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    assert!(stdout.contains("aspen.quick-confidence.v1"));
    assert!(stdout.contains("planned"));
    assert!(stdout.contains("runtime-host-acceptance-bundle"));
    assert!(stdout.contains("testing-public-api-boundary"));
    assert!(stdout.contains("non_proof_boundary"));
    assert!(stdout.contains("Hyperlight runtime-host execution proofs"));

    let summary = fs::read_to_string(&summary_path).expect("summary written");
    assert!(summary.contains("\"status\": \"planned\""));
    assert!(summary.contains("\"checks\""));
    assert!(summary.contains("\"skipped_gated_proofs\""));
    assert!(summary.contains("full dogfood/self-hosting acceptance"));
}
