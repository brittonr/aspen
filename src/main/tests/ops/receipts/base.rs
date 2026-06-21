#[test]
fn cli_receipt_commands_work() {
    let dir = temp_dir("receipt-cli");
    let artifact = install_receipt_artifact(&dir);
    let exported = export_receipt_fixture(&dir, &artifact);
    inspect_exported_receipt(&artifact, exported);
}

struct ReceiptArtifactFixture {
    ledger_root: PathBuf,
    artifact_ref: String,
}

struct ExportedReceiptFixture {
    artifact: PathBuf,
    export_receipt: PathBuf,
}

fn install_receipt_artifact(dir: &Path) -> ReceiptArtifactFixture {
    let ledger_root = dir.join("ledger");
    let receipt = dir.join("receipt.preserves");
    let signed = dir.join("signed-receipt.preserves");
    write_file(&receipt, r#"<receipt-test "payload">"#).expect("write receipt payload");
    run_receipt_command(ReceiptCommand::Sign {
        receipt,
        out: signed.clone(),
        signer: "receipt-cli".to_string(),
        purpose: PASS_EVIDENCE_PURPOSE.to_string(),
        trust_root: "local-trust-root".to_string(),
        key: "local-dev-key".to_string(),
        parents: Vec::new(),
    })
    .expect("sign receipt artifact");
    let signed_value = read_preserves_file(&signed).expect("read signed receipt");
    let imported = ledger::import_artifact(&ledger_root, &signed_value).expect("import signed receipt");
    ReceiptArtifactFixture {
        ledger_root,
        artifact_ref: imported.artifact_ref,
    }
}

fn export_receipt_fixture(dir: &Path, fixture: &ReceiptArtifactFixture) -> ExportedReceiptFixture {
    run_receipts_command(ReceiptsCommand::List {
        ledger: fixture.ledger_root.clone(),
    })
    .expect("list receipts");
    run_receipts_command(ReceiptsCommand::Show {
        receipt_ref: fixture.artifact_ref.clone(),
        ledger: fixture.ledger_root.clone(),
    })
    .expect("show receipt");
    run_receipts_command(ReceiptsCommand::Validate {
        receipt_ref: fixture.artifact_ref.clone(),
        ledger: fixture.ledger_root.clone(),
    })
    .expect("validate receipt");
    export_receipt_artifact(dir, fixture)
}

fn export_receipt_artifact(dir: &Path, fixture: &ReceiptArtifactFixture) -> ExportedReceiptFixture {
    let export_out = dir.join("exported-receipt.preserves");
    let export_receipt = dir.join("export-receipt.preserves");
    run_receipts_command(ReceiptsCommand::Export {
        receipt_ref: fixture.artifact_ref.clone(),
        ledger: fixture.ledger_root.clone(),
        out: export_out.clone(),
        receipt_out: Some(export_receipt.clone()),
    })
    .expect("export receipt");
    ExportedReceiptFixture {
        artifact: export_out,
        export_receipt,
    }
}

fn inspect_exported_receipt(fixture: &ReceiptArtifactFixture, exported: ExportedReceiptFixture) {
    let exported_value = read_preserves_file(&exported.artifact).expect("read exported receipt");
    assert_eq!(
        canonical_hash(&exported_value).expect("exported receipt ref"),
        fixture.artifact_ref
    );
    let export_receipt = read_preserves_file(&exported.export_receipt).expect("read export receipt");
    assert!(to_text(&export_receipt)
        .expect("export receipt text")
        .contains("ledger-export-receipt-v1"));
}
