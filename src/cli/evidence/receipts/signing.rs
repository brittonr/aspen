pub(super) struct Sign {
    pub(super) receipt: std::path::PathBuf,
    pub(super) out: std::path::PathBuf,
    pub(super) signer: String,
    pub(super) purpose: String,
    pub(super) trust_root: String,
    pub(super) key: String,
    pub(super) parents: Vec<String>,
}

pub(super) struct Verify {
    pub(super) signed_receipt: std::path::PathBuf,
    pub(super) purpose: String,
    pub(super) trust_root: String,
    pub(super) key: String,
    pub(super) key_ledger: Option<std::path::PathBuf>,
    pub(super) key_ref: Option<String>,
    pub(super) key_id: Option<String>,
    pub(super) signer: Option<String>,
    pub(super) subject_ref: Option<String>,
}

pub(super) fn run_operator_sign(input: Sign) -> molten::error::Result<()> {
    let receipt_value = super::io::read_preserves_file(&input.receipt)?;
    let signed = sign(&input, &receipt_value)?;
    let signed_ref = molten::preserves_rail::canonical_hash(&signed)?;
    let subject_ref = molten::preserves_rail::canonical_hash(&receipt_value)?;
    super::io::write_file(&input.out, &molten::preserves_rail::to_text(&signed)?)?;
    println!(
        "receipts sign ok signed={} subject={} signer={} purpose={} out={} evidence-only=pass",
        signed_ref,
        subject_ref,
        input.signer,
        input.purpose,
        input.out.display()
    );
    Ok(())
}

pub(super) fn run_test_sign(input: Sign) -> molten::error::Result<()> {
    let receipt_value = super::io::read_preserves_file(&input.receipt)?;
    let signed = sign(&input, &receipt_value)?;
    super::io::write_file(&input.out, &molten::preserves_rail::to_text(&signed)?)?;
    println!("signed receipt written to {}", input.out.display());
    Ok(())
}

pub(super) fn run_operator_verify(input: Verify) -> molten::error::Result<()> {
    let signed_value = super::io::read_preserves_file(&input.signed_receipt)?;
    super::ensure_keyring_selector_has_ledger(
        input.key_ledger.as_deref(),
        input.key_ref.as_deref(),
        input.key_id.as_deref(),
    )?;
    if let Some(ledger) = input.key_ledger {
        let keyring = super::load_signed_receipt_keyring(&ledger)?;
        let verified = molten::evidence::verify_signed_receipt_with_keyring_policy(
            &signed_value,
            &molten::evidence::VerifySignedReceiptKeyringPolicy {
                required_purpose: &input.purpose,
                trust_root: &input.trust_root,
                expected_signer: input.signer.as_deref(),
                expected_subject_ref: input.subject_ref.as_deref(),
                required_key_ref: input.key_ref.as_deref(),
                required_key_id: input.key_id.as_deref(),
                keys: &keyring.keys,
                revocations: &keyring.revocations,
            },
        )?;
        println!(
            "receipts verify-signed ok envelope={} subject={} signer={} purpose={} key={} key-id={} keyring=current evidence-only=pass",
            verified.receipt.envelope_ref,
            verified.receipt.subject_ref,
            verified.receipt.signer,
            verified.receipt.purpose,
            verified.key_ref,
            verified.key_id
        );
    } else {
        let verified = molten::evidence::verify_signed_receipt_with_policy(
            &signed_value,
            &molten::evidence::VerifySignedReceiptPolicy {
                required_purpose: &input.purpose,
                trust_root: &input.trust_root,
                key: &input.key,
                expected_signer: input.signer.as_deref(),
                expected_subject_ref: input.subject_ref.as_deref(),
            },
        )?;
        println!(
            "receipts verify-signed ok envelope={} subject={} signer={} purpose={} evidence-only=pass",
            verified.envelope_ref, verified.subject_ref, verified.signer, verified.purpose
        );
    }
    Ok(())
}

pub(super) fn run_test_verify(input: Verify) -> molten::error::Result<()> {
    let signed_value = super::io::read_preserves_file(&input.signed_receipt)?;
    super::ensure_keyring_selector_has_ledger(
        input.key_ledger.as_deref(),
        input.key_ref.as_deref(),
        input.key_id.as_deref(),
    )?;
    if let Some(ledger) = input.key_ledger {
        let keyring = super::load_signed_receipt_keyring(&ledger)?;
        let verified = molten::evidence::verify_signed_receipt_with_keyring_policy(
            &signed_value,
            &molten::evidence::VerifySignedReceiptKeyringPolicy {
                required_purpose: &input.purpose,
                trust_root: &input.trust_root,
                expected_signer: input.signer.as_deref(),
                expected_subject_ref: input.subject_ref.as_deref(),
                required_key_ref: input.key_ref.as_deref(),
                required_key_id: input.key_id.as_deref(),
                keys: &keyring.keys,
                revocations: &keyring.revocations,
            },
        )?;
        println!(
            "signed receipt verify ok envelope={} subject={} signer={} purpose={} key={} key-id={}",
            verified.receipt.envelope_ref,
            verified.receipt.subject_ref,
            verified.receipt.signer,
            verified.receipt.purpose,
            verified.key_ref,
            verified.key_id
        );
    } else {
        let verified = molten::evidence::verify_signed_receipt_with_policy(
            &signed_value,
            &molten::evidence::VerifySignedReceiptPolicy {
                required_purpose: &input.purpose,
                trust_root: &input.trust_root,
                key: &input.key,
                expected_signer: input.signer.as_deref(),
                expected_subject_ref: input.subject_ref.as_deref(),
            },
        )?;
        println!(
            "signed receipt verify ok envelope={} subject={} signer={} purpose={}",
            verified.envelope_ref, verified.subject_ref, verified.signer, verified.purpose
        );
    }
    Ok(())
}

fn sign(input: &Sign, receipt: &preserves::IOValue) -> molten::error::Result<preserves::IOValue> {
    molten::evidence::sign_receipt(&molten::evidence::SignReceiptInput {
        receipt,
        signer: &input.signer,
        purpose: &input.purpose,
        trust_root: &input.trust_root,
        key: &input.key,
        parents: &input.parents,
    })
}
