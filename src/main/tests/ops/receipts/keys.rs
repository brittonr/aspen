#[test]
fn cli_receipt_key_commands_work() {
    let dir = temp_dir("receipt-keys-cli");
    let fixture = receipt_signature_fixture(&dir);
    sign_receipt_fixture(&fixture);
    verify_receipt_signature(&fixture);
    wrong_signature_key_denies(&fixture);
    wrong_signature_subject_denies(&fixture);
}

struct ReceiptSignatureFixture {
    receipt: PathBuf,
    signed: PathBuf,
    signer: String,
    purpose: String,
    trust_root: String,
    key: String,
    subject_ref: String,
}

fn receipt_signature_fixture(dir: &Path) -> ReceiptSignatureFixture {
    let receipt = dir.join("receipt.preserves");
    write_file(&receipt, r#"<receipt-test "payload">"#).expect("write receipt");
    let receipt_value = read_preserves_file(&receipt).expect("read receipt");
    ReceiptSignatureFixture {
        receipt,
        signed: dir.join("signed-receipt.preserves"),
        signer: "node-a".to_string(),
        purpose: PASS_EVIDENCE_PURPOSE.to_string(),
        trust_root: "local-trust-root".to_string(),
        key: "local-dev-key".to_string(),
        subject_ref: canonical_hash(&receipt_value).expect("receipt ref"),
    }
}

fn sign_receipt_fixture(fixture: &ReceiptSignatureFixture) {
    run_receipt_command(ReceiptCommand::Sign {
        receipt: fixture.receipt.clone(),
        out: fixture.signed.clone(),
        signer: fixture.signer.clone(),
        purpose: fixture.purpose.clone(),
        trust_root: fixture.trust_root.clone(),
        key: fixture.key.clone(),
        parents: Vec::new(),
    })
    .expect("sign receipt");
    assert_eq!(
        ledger::artifact_kind(&read_preserves_file(&fixture.signed).expect("read signed receipt")),
        "signed-receipt"
    );
}

fn verify_receipt_signature(fixture: &ReceiptSignatureFixture) {
    run_receipt_command(ReceiptCommand::Verify {
        signed_receipt: fixture.signed.clone(),
        purpose: fixture.purpose.clone(),
        trust_root: fixture.trust_root.clone(),
        key: fixture.key.clone(),
        key_ledger: None,
        key_ref: None,
        key_id: None,
        signer: Some(fixture.signer.clone()),
        subject_ref: Some(fixture.subject_ref.clone()),
    })
    .expect("verify signed receipt");
}

fn wrong_signature_key_denies(fixture: &ReceiptSignatureFixture) {
    let denied = run_receipt_command(ReceiptCommand::Verify {
        signed_receipt: fixture.signed.clone(),
        purpose: fixture.purpose.clone(),
        trust_root: fixture.trust_root.clone(),
        key: "wrong-key".to_string(),
        key_ledger: None,
        key_ref: None,
        key_id: None,
        signer: Some(fixture.signer.clone()),
        subject_ref: Some(fixture.subject_ref.clone()),
    })
    .expect_err("wrong key should fail verification");
    assert!(denied.to_string().contains("signature"));
}

fn wrong_signature_subject_denies(fixture: &ReceiptSignatureFixture) {
    let denied = run_receipt_command(ReceiptCommand::Verify {
        signed_receipt: fixture.signed.clone(),
        purpose: fixture.purpose.clone(),
        trust_root: fixture.trust_root.clone(),
        key: fixture.key.clone(),
        key_ledger: None,
        key_ref: None,
        key_id: None,
        signer: Some(fixture.signer.clone()),
        subject_ref: Some(cli_synthetic_ref("wrong-receipt-subject").expect("wrong subject ref")),
    })
    .expect_err("wrong subject should fail verification");
    assert!(denied.to_string().contains("subject"));
}
