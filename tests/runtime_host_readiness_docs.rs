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

#[test]
fn runtime_host_readiness_doc_tracks_wasm_product_path_contract() {
    let Some(doc) = read_repo_file("docs/runtime-host-readiness.md") else {
        return;
    };
    let manifest = std::fs::read_to_string("test-harness/suites/vm/runtime-host-wasm-gap.ncl")
        .expect("wasm runtime-host harness row should exist when docs are present");
    let test = std::fs::read_to_string("crates/aspen-jobs/tests/wasm_product_path_test.rs")
        .expect("wasm product-path test should exist when docs are present");

    assert!(doc.contains("runtime-host-wasm-product-path"));
    assert!(doc.contains("cargo test -p aspen-jobs --test wasm_product_path_test --features plugins-wasm"));
    assert!(doc.contains("ASPEN_WASM_RUNTIME_HOST_EXECUTED"));
    assert!(doc.contains("ASPEN_WASM_RUNTIME_HOST_PRODUCT_PATH_GUARD"));
    assert!(doc.contains("aspen:runtime-host/wasm-v1"));

    assert!(manifest.contains("runtime-host-wasm-product-path"));
    assert!(manifest.contains("aspen-spawned-execution"));
    assert!(manifest.contains("e2e-registered"));
    assert!(manifest.contains("wasm_product_path_test"));
    assert!(manifest.contains("plugins-wasm"));

    assert!(test.contains("ASPEN_WASM_RUNTIME_HOST_EXECUTED"));
    assert!(test.contains("ASPEN_WASM_RUNTIME_HOST_PRODUCT_PATH_GUARD"));
    assert!(test.contains("JobManager"));
    assert!(test.contains("WorkerPool"));
}

#[test]
fn runtime_host_readiness_doc_tracks_hyperlight_product_path_contract() {
    let Some(doc) = read_repo_file("docs/runtime-host-readiness.md") else {
        return;
    };
    let manifest = std::fs::read_to_string("test-harness/suites/vm/runtime-host-hyperlight-gap.ncl")
        .expect("Hyperlight runtime-host harness row should exist when docs are present");
    let test = std::fs::read_to_string("crates/aspen-jobs/tests/hyperlight_product_path_test.rs")
        .expect("Hyperlight product-path test should exist when docs are present");

    assert!(doc.contains("runtime-host-hyperlight-product-path"));
    assert!(doc.contains("cargo test -p aspen-jobs --test hyperlight_product_path_test --features plugins-vm"));
    assert!(doc.contains("ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED"));
    assert!(doc.contains("ASPEN_HYPERLIGHT_RUNTIME_HOST_PRODUCT_PATH_GUARD"));
    assert!(doc.contains("aspen:runtime-host/hyperlight-v1"));

    assert!(manifest.contains("runtime-host-hyperlight-product-path"));
    assert!(manifest.contains("aspen-spawned-execution"));
    assert!(manifest.contains("e2e-registered"));
    assert!(manifest.contains("hyperlight_product_path_test"));
    assert!(manifest.contains("plugins-vm"));
    assert!(manifest.contains("ignored-only"));

    assert!(test.contains("ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED"));
    assert!(test.contains("ASPEN_HYPERLIGHT_RUNTIME_HOST_PRODUCT_PATH_GUARD"));
    assert!(test.contains("JobManager"));
    assert!(test.contains("WorkerPool"));
    assert!(test.contains("HyperlightWorker"));
}

#[test]
fn runtime_host_readiness_doc_tracks_oci_lowering_product_path_contract() {
    let Some(doc) = read_repo_file("docs/runtime-host-readiness.md") else {
        return;
    };
    let manifest = std::fs::read_to_string("test-harness/suites/vm/runtime-host-oci-lowering-gap.ncl")
        .expect("OCI lowering runtime-host harness row should exist when docs are present");
    let test = std::fs::read_to_string("crates/aspen-jobs/tests/oci_lowering_product_path_test.rs")
        .expect("OCI lowering product-path test should exist when docs are present");

    assert!(doc.contains("runtime-host-oci-lowering-product-path"));
    assert!(doc.contains("cargo test -p aspen-jobs --test oci_lowering_product_path_test --features plugins-wasm"));
    assert!(doc.contains("ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED"));
    assert!(doc.contains("ASPEN_OCI_LOWERING_RUNTIME_HOST_PRODUCT_PATH_GUARD"));
    assert!(doc.contains("aspen:runtime-host/wasm-v1"));
    assert!(doc.contains("ASPEN_WASM_RUNTIME_HOST_EXECUTED"));
    assert!(doc.contains("sha256:"));

    assert!(manifest.contains("runtime-host-oci-lowering-product-path"));
    assert!(manifest.contains("aspen-spawned-execution"));
    assert!(manifest.contains("e2e-registered"));
    assert!(manifest.contains("oci_lowering_product_path_test"));
    assert!(manifest.contains("plugins-wasm"));
    assert!(manifest.contains("immutable-oci-source-identity"));
    assert!(manifest.contains("derived-isolated-target-artifact"));

    assert!(test.contains("ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED"));
    assert!(test.contains("ASPEN_OCI_LOWERING_RUNTIME_HOST_PRODUCT_PATH_GUARD"));
    assert!(test.contains("JobManager"));
    assert!(test.contains("WorkerPool"));
    assert!(test.contains("OciLoweringPlan"));
    assert!(test.contains("WasmComponentWorker"));
}

#[test]
fn runtime_host_readiness_doc_tracks_hermit_uhyve_product_path_contract() {
    let Some(doc) = read_repo_file("docs/runtime-host-readiness.md") else {
        return;
    };
    let manifest = std::fs::read_to_string("test-harness/suites/vm/runtime-host-hermit-gap.ncl")
        .expect("Hermit/Uhyve runtime-host harness row should exist when docs are present");
    let test = format!(
        "{}\n{}",
        std::fs::read_to_string("crates/aspen-jobs/tests/hermit_uhyve_product_path_test.rs")
            .expect("Hermit/Uhyve product-path test should exist when docs are present"),
        std::fs::read_to_string("crates/aspen-jobs/src/vm_executor/hermit_uhyve.rs")
            .expect("Hermit/Uhyve worker should exist when docs are present")
    );

    assert!(doc.contains("runtime-host-hermit-uhyve-product-path"));
    assert!(doc.contains("cargo test -p aspen-jobs --test hermit_uhyve_product_path_test --features plugins-vm"));
    assert!(doc.contains("ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED"));
    assert!(doc.contains("ASPEN_HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD"));
    assert!(doc.contains("aspen:runtime-host/hermit-uhyve-v1"));
    assert!(doc.contains("ASPEN_UHYVE"));
    assert!(doc.contains("ASPEN_HERMIT_UHYVE_IMAGE"));
    assert!(doc.contains("hermit-uhyve-marker"));
    assert!(doc.contains("hermit-uhyve-marker-contract"));
    assert!(doc.contains("fixture-build-is-not-runtime-host-proof"));

    assert!(manifest.contains("runtime-host-hermit-uhyve-product-path"));
    assert!(manifest.contains("aspen-spawned-execution"));
    assert!(manifest.contains("e2e-registered"));
    assert!(manifest.contains("hermit_uhyve_product_path_test"));
    assert!(manifest.contains("plugins-vm"));
    assert!(manifest.contains("ignored-only"));
    assert!(manifest.contains("real-uhyve-runner"));
    assert!(manifest.contains("packages.x86_64-linux.hermit-uhyve-marker"));
    assert!(manifest.contains("checks.x86_64-linux.hermit-uhyve-marker-contract"));

    assert!(test.contains("ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED"));
    assert!(test.contains("ASPEN_HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD"));
    assert!(test.contains("JobManager"));
    assert!(test.contains("WorkerPool"));
    assert!(test.contains("HermitUhyveWorker"));
    assert!(test.contains("proof marker missing"));
}
