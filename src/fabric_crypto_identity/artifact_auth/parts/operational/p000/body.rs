use super::*;

const BLAKE3_HEX_CHARS: usize = 64;
const OPERATIONAL_RECEIPT_SCHEMA: &str = "molten-artifact-auth-operational-receipt-v1";
const OPERATIONAL_RECEIPT_CONSUMER: &str = "molten";
const OPERATIONAL_RECEIPT_DIR: &str = "artifact-auth";
const OPERATIONAL_RECEIPT_EXTENSION: &str = "json";
const OPERATIONAL_RECEIPT_NON_CLAIM: &str = "persisted replay proves exact standalone bytes and local capability-file key state only; it does not grant membership, capability, federation, transport, storage, lifecycle, signing-policy, release, or runtime authority";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoltenArtifactAuthOperationalReceipt {
    pub schema: String,
    pub consumer: String,
    pub statement_ref: String,
    pub public_key_ref: String,
    pub signature_ref: String,
    pub signing_policy_ref: String,
    pub key_handle_ref: String,
    pub key_purpose: String,
    pub key_generation: u64,
    pub currentness_ref: String,
    pub signed: SignedArtifactAuthStatement,
    pub standalone_passed: bool,
    pub compatibility_case_explained: bool,
    pub legacy_authoritative: bool,
    pub standalone_authority_admitted: bool,
    pub rollback_available: bool,
    pub non_claim: String,
    pub receipt_blake3: String,
}

#[derive(serde::Serialize)]
struct MoltenArtifactAuthReceiptMaterial<'a> {
    schema: &'a str,
    consumer: &'a str,
    statement_ref: &'a str,
    public_key_ref: &'a str,
    signature_ref: &'a str,
    signing_policy_ref: &'a str,
    key_handle_ref: &'a str,
    key_purpose: &'a str,
    key_generation: u64,
    currentness_ref: &'a str,
    signed: &'a SignedArtifactAuthStatement,
    standalone_passed: bool,
    compatibility_case_explained: bool,
    legacy_authoritative: bool,
    standalone_authority_admitted: bool,
    rollback_available: bool,
    non_claim: &'a str,
}

// r[impl molten.artifact_auth_operational_receipt.identity]
pub fn build_artifact_auth_operational_receipt(
    input: &MoltenArtifactAuthShellInput<'_>,
    signed: &SignedArtifactAuthStatement,
    report: &MoltenArtifactAuthShellReport,
) -> crate::error::Result<MoltenArtifactAuthOperationalReceipt> {
    let is_standalone_passed = report.dual_run.standalone.as_ref().is_some_and(|decision| decision.passed);
    let compatibility = &report.dual_run.compatibility;
    if report.cryptographic_failure_code.is_some() || !is_standalone_passed || !compatibility.case_explained {
        return Err(crate::error::MoltenError::invalid_harness(
            "artifact-auth operational receipt requires passing explained standalone evidence",
        ));
    }
    if report.statement_ref != signed.statement_ref
        || report.public_key_ref != signed.public_key_ref
        || report.signature_ref != signed.signature_ref
        || report.signature_hex != signed.signature_hex
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "artifact-auth operational receipt shell carrier drift",
        ));
    }
    let mut receipt = MoltenArtifactAuthOperationalReceipt {
        schema: OPERATIONAL_RECEIPT_SCHEMA.to_string(),
        consumer: OPERATIONAL_RECEIPT_CONSUMER.to_string(),
        statement_ref: signed.statement_ref.clone(),
        public_key_ref: signed.public_key_ref.clone(),
        signature_ref: signed.signature_ref.clone(),
        signing_policy_ref: input.signing_policy_ref.to_string(),
        key_handle_ref: input.handle.handle_ref.clone(),
        key_purpose: input.handle.purpose.as_str().to_string(),
        key_generation: input.handle.generation,
        currentness_ref: input.handle.currentness_evidence_ref.clone(),
        signed: signed.clone(),
        standalone_passed: is_standalone_passed,
        compatibility_case_explained: compatibility.case_explained,
        legacy_authoritative: compatibility.legacy_authoritative,
        standalone_authority_admitted: compatibility.standalone_authority_admitted,
        rollback_available: compatibility.rollback_available,
        non_claim: OPERATIONAL_RECEIPT_NON_CLAIM.to_string(),
        receipt_blake3: String::new(),
    };
    receipt.receipt_blake3 = operational_receipt_identity(&receipt)?;
    validate_artifact_auth_operational_receipt(&receipt)?;
    debug_assert!(receipt.legacy_authoritative);
    debug_assert!(!receipt.standalone_authority_admitted);
    Ok(receipt)
}

// r[impl molten.artifact_auth_operational_receipt.identity]
pub fn validate_artifact_auth_operational_receipt(
    receipt: &MoltenArtifactAuthOperationalReceipt,
) -> crate::error::Result<()> {
    if receipt.schema != OPERATIONAL_RECEIPT_SCHEMA || receipt.consumer != OPERATIONAL_RECEIPT_CONSUMER {
        return Err(crate::error::MoltenError::invalid_harness(
            "artifact-auth operational receipt schema or consumer mismatch",
        ));
    }
    if receipt.key_generation == 0 || receipt.key_purpose.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(
            "artifact-auth operational receipt key state is invalid",
        ));
    }
    for (label, value) in [
        ("statement", receipt.statement_ref.as_str()),
        ("public key", receipt.public_key_ref.as_str()),
        ("signature", receipt.signature_ref.as_str()),
        ("signing policy", receipt.signing_policy_ref.as_str()),
        ("key handle", receipt.key_handle_ref.as_str()),
        ("currentness", receipt.currentness_ref.as_str()),
        ("receipt", receipt.receipt_blake3.as_str()),
    ] {
        require_blake3_ref(label, value)?;
    }
    if receipt.statement_ref != receipt.signed.statement_ref
        || receipt.public_key_ref != receipt.signed.public_key_ref
        || receipt.signature_ref != receipt.signed.signature_ref
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "artifact-auth operational receipt carrier refs drifted",
        ));
    }
    let public_key = iroh::PublicKey::from_str(&receipt.signed.public_key).map_err(|_| {
        crate::error::MoltenError::invalid_harness("artifact-auth operational receipt public key is malformed")
    })?;
    require_carrier_identity(
        "public key",
        &receipt.signed.public_key_ref,
        &crate::preserves_rail::content_ref_from_bytes(public_key.as_bytes()),
    )?;
    require_carrier_identity(
        "signature",
        &receipt.signed.signature_ref,
        &crate::preserves_rail::content_ref_from_bytes(&receipt.signed.signature_bytes),
    )?;
    require_carrier_identity(
        "signature hex",
        &receipt.signed.signature_hex,
        &bytes_to_lower_hex(&receipt.signed.signature_bytes),
    )?;
    if !receipt.standalone_passed
        || !receipt.compatibility_case_explained
        || !receipt.legacy_authoritative
        || receipt.standalone_authority_admitted
        || !receipt.rollback_available
        || receipt.non_claim != OPERATIONAL_RECEIPT_NON_CLAIM
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "artifact-auth operational receipt authority boundary drifted",
        ));
    }
    require_carrier_identity("receipt", &receipt.receipt_blake3, &operational_receipt_identity(receipt)?)?;
    Ok(())
}

// r[impl molten.artifact_auth_operational_receipt.persistence]
pub fn artifact_auth_operational_receipt_path(
    statement_ref: &str,
) -> crate::error::Result<crate::node_state::NodeStatePath> {
    require_blake3_ref("statement", statement_ref)?;
    let digest = statement_ref
        .strip_prefix("blake3:")
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("artifact-auth statement ref prefix is invalid"))?;
    crate::node_state::NodeStatePath::parse(&format!(
        "{OPERATIONAL_RECEIPT_DIR}/{digest}.{OPERATIONAL_RECEIPT_EXTENSION}"
    ))
}

// r[impl molten.artifact_auth_operational_receipt.persistence]
pub fn write_artifact_auth_operational_receipt(
    namespace: &crate::node_state::NodeStateNamespace,
    receipt: &MoltenArtifactAuthOperationalReceipt,
) -> crate::error::Result<crate::node_state::NodeStatePath> {
    require_receipts_namespace(namespace)?;
    validate_artifact_auth_operational_receipt(receipt)?;
    let directory = crate::node_state::NodeStatePath::parse(OPERATIONAL_RECEIPT_DIR)?;
    let path = artifact_auth_operational_receipt_path(&receipt.statement_ref)?;
    namespace.create_dir_all(&directory)?;
    if namespace.try_exists(&path)? {
        let existing = read_artifact_auth_operational_receipt(namespace, &path)?;
        if existing != *receipt {
            return Err(crate::error::MoltenError::invalid_harness(
                "artifact-auth operational receipt replacement denied",
            ));
        }
        return Ok(path);
    }
    let bytes = serde_json::to_vec(receipt).map_err(|error| {
        crate::error::MoltenError::invalid_harness(format!("artifact-auth receipt serialization failed: {error}"))
    })?;
    namespace.write(&path, &bytes)?;
    let reopened = read_artifact_auth_operational_receipt(namespace, &path)?;
    if reopened != *receipt {
        return Err(crate::error::MoltenError::invalid_harness(
            "artifact-auth operational receipt write verification failed",
        ));
    }
    Ok(path)
}

// r[impl molten.artifact_auth_operational_receipt.persistence]
pub fn read_artifact_auth_operational_receipt(
    namespace: &crate::node_state::NodeStateNamespace,
    path: &crate::node_state::NodeStatePath,
) -> crate::error::Result<MoltenArtifactAuthOperationalReceipt> {
    require_receipts_namespace(namespace)?;
    let bytes = namespace.read(path, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
    let receipt = serde_json::from_slice::<MoltenArtifactAuthOperationalReceipt>(&bytes).map_err(|error| {
        crate::error::MoltenError::invalid_harness(format!("artifact-auth receipt parsing failed: {error}"))
    })?;
    validate_artifact_auth_operational_receipt(&receipt)?;
    Ok(receipt)
}

// r[impl molten.artifact_auth_operational_receipt.replay]
pub fn replay_artifact_auth_operational_receipt(
    adapter: &super::super::IrohEd25519FileAdapter<'_>,
    namespace: &crate::node_state::NodeStateNamespace,
    path: &crate::node_state::NodeStatePath,
    input: &MoltenArtifactAuthShellInput<'_>,
) -> crate::error::Result<MoltenArtifactAuthShellReport> {
    let receipt = read_artifact_auth_operational_receipt(namespace, path)?;
    let current = adapter.resolve_or_generate(input.handle.purpose, input.signing_policy_ref, false)?;
    if current.handle.handle.handle_ref != receipt.key_handle_ref
        || current.handle.handle.generation != receipt.key_generation
        || current.handle.handle.currentness_evidence_ref != receipt.currentness_ref
        || current.handle.handle != *input.handle
        || receipt.signing_policy_ref != input.signing_policy_ref
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "artifact-auth operational receipt current key state drifted",
        ));
    }
    let report = evaluate_artifact_auth_shell_dual_run(&input.statement, &receipt.signed)?;
    let expected = build_artifact_auth_operational_receipt(input, &receipt.signed, &report)?;
    if expected != receipt {
        return Err(crate::error::MoltenError::invalid_harness("artifact-auth operational receipt replay drifted"));
    }
    debug_assert!(report.dual_run.compatibility.legacy_authoritative);
    debug_assert!(!report.dual_run.compatibility.standalone_authority_admitted);
    Ok(report)
}

fn operational_receipt_identity(receipt: &MoltenArtifactAuthOperationalReceipt) -> crate::error::Result<String> {
    let material = MoltenArtifactAuthReceiptMaterial {
        schema: &receipt.schema,
        consumer: &receipt.consumer,
        statement_ref: &receipt.statement_ref,
        public_key_ref: &receipt.public_key_ref,
        signature_ref: &receipt.signature_ref,
        signing_policy_ref: &receipt.signing_policy_ref,
        key_handle_ref: &receipt.key_handle_ref,
        key_purpose: &receipt.key_purpose,
        key_generation: receipt.key_generation,
        currentness_ref: &receipt.currentness_ref,
        signed: &receipt.signed,
        standalone_passed: receipt.standalone_passed,
        compatibility_case_explained: receipt.compatibility_case_explained,
        legacy_authoritative: receipt.legacy_authoritative,
        standalone_authority_admitted: receipt.standalone_authority_admitted,
        rollback_available: receipt.rollback_available,
        non_claim: &receipt.non_claim,
    };
    let bytes = serde_json::to_vec(&material).map_err(|error| {
        crate::error::MoltenError::invalid_harness(format!("artifact-auth receipt hashing failed: {error}"))
    })?;
    Ok(crate::preserves_rail::content_ref_from_bytes(&bytes))
}

fn require_receipts_namespace(namespace: &crate::node_state::NodeStateNamespace) -> crate::error::Result<()> {
    if namespace.kind() != crate::node_state::NodeStateNamespaceKind::Receipts {
        return Err(crate::error::MoltenError::invalid_harness(
            "artifact-auth operational receipts require receipts namespace",
        ));
    }
    Ok(())
}
