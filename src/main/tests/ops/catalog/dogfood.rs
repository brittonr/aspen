#[test]
fn cli_dogfood_local_node_commands_work() {
    let dir = temp_dir("dogfood-cli");
    let fixture = run_dogfood_local_node(&dir);
    exercise_dogfood_receipts(&fixture);
    let nix = export_nix_dogfood(&fixture);
    verify_stale_nextest_marker(&fixture, &nix);
    verify_tampered_dogfood_report(&fixture, &nix);
    show_dogfood_artifacts(fixture, nix);
}

struct DogfoodCliFixture {
    dir: PathBuf,
    state_root: PathBuf,
    report: PathBuf,
    release_gate: PathBuf,
    replay_verify: PathBuf,
    replay_index: PathBuf,
    report_value: preserves::IOValue,
    report_ref: String,
}

struct NixDogfoodFixture {
    evidence: PathBuf,
    verify: PathBuf,
}

fn run_dogfood_local_node(dir: &Path) -> DogfoodCliFixture {
    let state_root = dir.join("state");
    let report = dir.join("dogfood-report.preserves");
    let release_gate = dir.join("release-gate.preserves");
    let replay_verify = dir.join("replay-verify.preserves");
    let replay_index = dir.join("replay-evidence-index.preserves");
    run_dogfood_command(DogfoodCommand::LocalNode {
        state_root: state_root.clone(),
        out: report.clone(),
        release_gate_out: Some(release_gate.clone()),
        replay_verify_out: Some(replay_verify.clone()),
        replay_index_out: Some(replay_index.clone()),
    })
    .expect("dogfood local node");
    let report_value = read_preserves_file(&report).expect("read dogfood report");
    let parsed = operator_dogfood::parse_dogfood_report(&report_value).expect("parse dogfood report");
    assert_eq!(parsed.decision, "pass");
    assert_dogfood_evidence_files(&release_gate, &replay_verify, &replay_index);
    DogfoodCliFixture {
        dir: dir.to_path_buf(),
        state_root,
        report,
        release_gate,
        replay_verify,
        replay_index,
        report_value,
        report_ref: parsed.report_ref,
    }
}

fn assert_dogfood_evidence_files(release_gate: &Path, replay_verify: &Path, replay_index: &Path) {
    assert!(fs::read_to_string(release_gate)
        .expect("read release gate")
        .contains("release-gate-receipt-v1"));
    assert!(fs::read_to_string(replay_verify)
        .expect("read replay verify")
        .contains("deterministic-replay-verify-v1"));
    assert!(fs::read_to_string(replay_index)
        .expect("read replay index")
        .contains("deterministic-replay-index-v1"));
}

fn exercise_dogfood_receipts(fixture: &DogfoodCliFixture) {
    let ledger_root = fixture.state_root.join("ledger");
    run_receipts_command(ReceiptsCommand::List {
        ledger: ledger_root.clone(),
    })
    .expect("receipts list");
    run_receipts_command(ReceiptsCommand::Show {
        receipt_ref: fixture.report_ref.clone(),
        ledger: ledger_root.clone(),
    })
    .expect("receipts show dogfood report");
    run_receipts_command(ReceiptsCommand::Validate {
        receipt_ref: fixture.report_ref.clone(),
        ledger: ledger_root.clone(),
    })
    .expect("receipts validate dogfood report");
    export_dogfood_report(fixture, ledger_root);
}

fn export_dogfood_report(fixture: &DogfoodCliFixture, ledger_root: PathBuf) {
    let exported_report = fixture.dir.join("exported-dogfood-report.preserves");
    run_receipts_command(ReceiptsCommand::Export {
        receipt_ref: fixture.report_ref.clone(),
        ledger: ledger_root,
        out: exported_report.clone(),
        receipt_out: Some(fixture.dir.join("receipts-export.preserves")),
    })
    .expect("receipts export dogfood report");
    assert_eq!(
        canonical_hash(&read_preserves_file(&exported_report).expect("exported dogfood report"))
            .expect("exported ref"),
        fixture.report_ref
    );
    write_dogfood_summary(fixture);
}

fn write_dogfood_summary(fixture: &DogfoodCliFixture) {
    fs::write(
        fixture.dir.join("dogfood-summary.txt"),
        format!(
            "dogfood local-node decision=pass report={} release-gate={}\n",
            fixture.report_ref,
            canonical_hash(&read_preserves_file(&fixture.release_gate).expect("release gate value"))
                .expect("release ref")
        ),
    )
    .expect("write summary");
}

fn export_nix_dogfood(fixture: &DogfoodCliFixture) -> NixDogfoodFixture {
    fs::write(fixture.dir.join("after-nextest.txt"), "/nix/store/test-molten-nextest\n")
        .expect("write nextest marker");
    let evidence = fixture.dir.join("nix-dogfood-evidence.preserves");
    let verify = fixture.dir.join("nix-dogfood-verify.preserves");
    run_dogfood_command(DogfoodCommand::NixReleaseExport {
        output_path: fixture.dir.clone(),
        out: evidence.clone(),
    })
    .expect("dogfood nix release export");
    run_dogfood_command(DogfoodCommand::NixReleaseVerify {
        output_path: fixture.dir.clone(),
        evidence: evidence.clone(),
        receipt_out: verify.clone(),
    })
    .expect("dogfood nix release verify");
    let value = read_preserves_file(&verify).expect("read nix verify");
    let parsed = operator_dogfood::parse_nix_dogfood_verify_receipt(&value).expect("parse nix verify");
    assert_eq!(parsed.decision, "pass");
    NixDogfoodFixture { evidence, verify }
}

fn verify_stale_nextest_marker(fixture: &DogfoodCliFixture, nix: &NixDogfoodFixture) {
    fs::write(
        fixture.dir.join("after-nextest.txt"),
        "/nix/store/stale-molten-nextest\n",
    )
    .expect("tamper nextest marker");
    let stale_verify = fixture.dir.join("nix-dogfood-verify-stale.preserves");
    run_dogfood_command(DogfoodCommand::NixReleaseVerify {
        output_path: fixture.dir.clone(),
        evidence: nix.evidence.clone(),
        receipt_out: stale_verify.clone(),
    })
    .expect("dogfood nix release verify stale marker");
    let value = read_preserves_file(&stale_verify).expect("read stale nix verify");
    let receipt = operator_dogfood::parse_nix_dogfood_verify_receipt(&value).expect("parse stale nix verify");
    assert_eq!(receipt.decision, "deny");
    assert!(receipt
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("nextest-marker-ref mismatch")));
}

fn verify_tampered_dogfood_report(fixture: &DogfoodCliFixture, nix: &NixDogfoodFixture) {
    fs::write(&fixture.report, "<tampered-dogfood-report>\n").expect("tamper report");
    let tampered_verify = fixture.dir.join("nix-dogfood-verify-tampered.preserves");
    run_dogfood_command(DogfoodCommand::NixReleaseVerify {
        output_path: fixture.dir.clone(),
        evidence: nix.evidence.clone(),
        receipt_out: tampered_verify.clone(),
    })
    .expect("dogfood nix release verify tampered report");
    let value = read_preserves_file(&tampered_verify).expect("read tampered nix verify");
    let receipt = operator_dogfood::parse_nix_dogfood_verify_receipt(&value).expect("parse tampered nix verify");
    assert_eq!(receipt.decision, "deny");
    assert!(receipt
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("Nix dogfood output observation failed")));
    fs::write(&fixture.report, to_text(&fixture.report_value).expect("report text")).expect("restore report");
}

fn show_dogfood_artifacts(fixture: DogfoodCliFixture, nix: NixDogfoodFixture) {
    run_dogfood_command(DogfoodCommand::Show {
        artifact: fixture.report.clone(),
    })
    .expect("dogfood show report");
    run_dogfood_command(DogfoodCommand::Show {
        artifact: fixture.release_gate.clone(),
    })
    .expect("dogfood show gate");
    assert_dogfood_evidence_files(&fixture.release_gate, &fixture.replay_verify, &fixture.replay_index);
    run_dogfood_command(DogfoodCommand::Show { artifact: nix.evidence }).expect("dogfood show nix evidence");
    run_dogfood_command(DogfoodCommand::Show { artifact: nix.verify }).expect("dogfood show nix verify");
}
