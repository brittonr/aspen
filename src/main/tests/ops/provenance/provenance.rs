#[test]
fn cli_provenance_commands_work() {
    let dir = temp_dir("provenance-cli");
    let artifact_ref = cli_synthetic_ref("provenance-artifact").expect("artifact ref");
    reviewed_provenance_passes(&dir, &artifact_ref);
    sandbox_provenance_denies(&dir);
    let build = write_and_verify_build_record(&dir, &artifact_ref);
    reproducible_provenance_passes(&dir, &artifact_ref, &build);
    build_mismatch_denies(build);
}

struct ProvenanceBuildFixture {
    build_record: PathBuf,
    build_pass: PathBuf,
    actual_ref: String,
}

fn reviewed_provenance_passes(dir: &Path, artifact_ref: &str) {
    let fixture_out = dir.join("reviewed.preserves");
    run_provenance_command(ProvenanceCommand::Fixture {
        artifact_ref: artifact_ref.to_string(),
        out: Some(fixture_out.clone()),
    })
    .expect("write reviewed provenance fixture");
    run_provenance_command(ProvenanceCommand::Show {
        artifact: fixture_out.clone(),
    })
    .expect("show provenance fixture");
    let pass_receipt = dir.join("provenance-pass.preserves");
    evaluate_provenance_record("install", artifact_ref, fixture_out, pass_receipt.clone());
    assert_provenance_summary(&pass_receipt, "decision=pass");
}

fn sandbox_provenance_denies(dir: &Path) {
    let sandbox_ref = cli_synthetic_ref("provenance-sandbox-artifact").expect("sandbox ref");
    let sandbox_out = dir.join("sandbox.preserves");
    run_provenance_command(ProvenanceCommand::Record {
        artifact_ref: sandbox_ref.clone(),
        trust_state: molten::provenance::TRUST_STATE_SANDBOX_ONLY.to_string(),
        source_refs: vec![cli_synthetic_ref("provenance-source").expect("source ref")],
        dependency_closure_ref: cli_synthetic_ref("provenance-deps").expect("deps ref"),
        toolchain_refs: vec![cli_synthetic_ref("provenance-toolchain").expect("toolchain ref")],
        builder_ref: cli_synthetic_ref("provenance-builder").expect("builder ref"),
        review_refs: Vec::new(),
        test_refs: Vec::new(),
        source_gate_refs: Vec::new(),
        policy_refs: vec![cli_synthetic_ref("provenance-policy").expect("policy ref")],
        build_record_refs: Vec::new(),
        out: Some(sandbox_out.clone()),
    })
    .expect("write sandbox provenance record");
    let deny_receipt = dir.join("provenance-deny.preserves");
    evaluate_provenance_record("run", &sandbox_ref, sandbox_out, deny_receipt.clone());
    assert_provenance_summary(&deny_receipt, "decision=deny");
}

fn write_and_verify_build_record(dir: &Path, artifact_ref: &str) -> ProvenanceBuildFixture {
    let build_record = dir.join("build-record.preserves");
    run_provenance_command(ProvenanceCommand::BuildRecord {
        expected_artifact_ref: artifact_ref.to_string(),
        source_refs: vec![cli_synthetic_ref("provenance-build-source").expect("build source ref")],
        dependency_closure_ref: cli_synthetic_ref("provenance-build-deps").expect("build deps ref"),
        toolchain_refs: vec![cli_synthetic_ref("provenance-build-toolchain").expect("build toolchain ref")],
        build_params: vec!["target=x86_64-linux".to_string()],
        builder_ref: cli_synthetic_ref("provenance-build-builder").expect("build builder ref"),
        nix_derivation_refs: vec![cli_synthetic_ref("provenance-build-derivation").expect("build derivation ref")],
        policy_refs: vec![cli_synthetic_ref("provenance-build-policy").expect("build policy ref")],
        evidence_refs: vec![cli_synthetic_ref("provenance-build-evidence").expect("build evidence ref")],
        out: Some(build_record.clone()),
    })
    .expect("write provenance build record");
    run_provenance_command(ProvenanceCommand::Show {
        artifact: build_record.clone(),
    })
    .expect("show provenance build record");
    let build_pass = dir.join("build-pass.preserves");
    verify_build_record(&build_record, artifact_ref, &build_pass);
    assert_provenance_summary(&build_pass, "decision=pass");
    ProvenanceBuildFixture {
        build_record,
        build_pass,
        actual_ref: cli_synthetic_ref("provenance-actual-artifact").expect("actual ref"),
    }
}

fn reproducible_provenance_passes(dir: &Path, artifact_ref: &str, build: &ProvenanceBuildFixture) {
    let build_record_ref = canonical_hash(&read_preserves_file(&build.build_record).expect("read build record"))
        .expect("build record ref");
    let reproducible_record = dir.join("reproducible.preserves");
    run_provenance_command(ProvenanceCommand::Record {
        artifact_ref: artifact_ref.to_string(),
        trust_state: molten::provenance::TRUST_STATE_REPRODUCIBLE_VERIFIED.to_string(),
        source_refs: vec![cli_synthetic_ref("provenance-repro-source").expect("repro source ref")],
        dependency_closure_ref: cli_synthetic_ref("provenance-repro-deps").expect("repro deps ref"),
        toolchain_refs: vec![cli_synthetic_ref("provenance-repro-toolchain").expect("repro toolchain ref")],
        builder_ref: cli_synthetic_ref("provenance-repro-builder").expect("repro builder ref"),
        review_refs: Vec::new(),
        test_refs: Vec::new(),
        source_gate_refs: Vec::new(),
        policy_refs: Vec::new(),
        build_record_refs: vec![build_record_ref],
        out: Some(reproducible_record.clone()),
    })
    .expect("write reproducible provenance record");
    let receipt = dir.join("provenance-reproducible-pass.preserves");
    evaluate_provenance_with_build(
        "install",
        artifact_ref,
        reproducible_record,
        build.build_pass.clone(),
        receipt.clone(),
    );
    assert_provenance_summary(&receipt, "decision=pass");
}

fn build_mismatch_denies(build: ProvenanceBuildFixture) {
    let build_deny = build.build_record.with_file_name("build-deny.preserves");
    verify_build_record(&build.build_record, &build.actual_ref, &build_deny);
    assert_provenance_summary(&build_deny, "decision=deny");
}

fn evaluate_provenance_record(operation: &str, artifact_ref: &str, provenance_path: PathBuf, receipt_out: PathBuf) {
    evaluate_provenance_paths(operation, artifact_ref, vec![provenance_path], Vec::new(), receipt_out);
}

fn evaluate_provenance_with_build(
    operation: &str,
    artifact_ref: &str,
    provenance_path: PathBuf,
    build_verification_path: PathBuf,
    receipt_out: PathBuf,
) {
    evaluate_provenance_paths(
        operation,
        artifact_ref,
        vec![provenance_path],
        vec![build_verification_path],
        receipt_out,
    );
}

fn evaluate_provenance_paths(
    operation: &str,
    artifact_ref: &str,
    provenance_paths: Vec<PathBuf>,
    build_verification_paths: Vec<PathBuf>,
    receipt_out: PathBuf,
) {
    run_provenance_command(ProvenanceCommand::Evaluate {
        operation: operation.to_string(),
        profile: "node-control".to_string(),
        artifact_ref: artifact_ref.to_string(),
        provenance_paths,
        build_verification_paths,
        prior_diagnostics: Vec::new(),
        receipt_out: Some(receipt_out),
    })
    .expect("evaluate provenance");
}

fn verify_build_record(build_record: &Path, actual_artifact_ref: &str, receipt_out: &Path) {
    run_provenance_command(ProvenanceCommand::VerifyBuild {
        build_record: build_record.to_path_buf(),
        actual_artifact_ref: actual_artifact_ref.to_string(),
        prior_diagnostics: Vec::new(),
        receipt_out: Some(receipt_out.to_path_buf()),
    })
    .expect("verify provenance build");
}

fn assert_provenance_summary(receipt: &Path, expected: &str) {
    let summary = molten::provenance::provenance_summary(&read_preserves_file(receipt).expect("read provenance receipt"))
        .expect("summarize provenance receipt");
    assert!(summary.contains(expected));
}
