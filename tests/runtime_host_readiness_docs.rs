fn read_repo_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[test]
fn runtime_host_readiness_doc_is_discoverable() {
    let Some(readme) = read_repo_file("README.md") else {
        return;
    };
    let doc = std::fs::read_to_string("docs/runtime-host-readiness.md")
        .expect("runtime host readiness doc should exist when README is present");

    assert!(readme.contains("docs/runtime-host-readiness.md"));
    assert!(readme.contains("Runtime Host Readiness"));
    assert!(doc.contains("# Runtime Host Readiness"));
}

#[test]
fn runtime_host_readiness_doc_tracks_microvm_e2e_contract() {
    let Some(doc) = read_repo_file("docs/runtime-host-readiness.md") else {
        return;
    };
    let test = std::fs::read_to_string("nix/tests/vm-snapshot-e2e.nix")
        .expect("vm snapshot e2e test should exist when docs are present");

    assert!(doc.contains("runtime-host-microvm-ci-vm"));
    assert!(doc.contains("aspen-spawned-execution"));
    assert!(doc.contains("checks.x86_64-linux.vm-snapshot-e2e-test"));
    assert!(doc.contains("--option sandbox false"));
    assert!(doc.contains("ASPEN_CI_NET_CONFIG"));
    assert!(doc.contains("worker registered with cluster"));
    assert!(doc.contains("CI job completed via snapshot-restored VM"));
    assert!(doc.contains("All stress test jobs completed"));
    assert!(doc.contains("[REDACTED]"));

    assert!(test.contains("ASPEN_CI_NET_CONFIG"));
    assert!(test.contains("worker registered with cluster"));
    assert!(test.contains("CI job completed via snapshot-restored VM"));
    assert!(test.contains("All stress test jobs completed"));
    assert!(test.contains("timeout_secs"));
}
