use std::str::FromStr;

use molten_core::fabric_crypto_identity::MoltenArtifactAuthObservation;
use molten_core::fabric_crypto_identity::MoltenArtifactAuthReport;
use molten_core::fabric_crypto_identity::MoltenArtifactAuthStatementInput;
use molten_core::fabric_crypto_identity::OpaqueKeyHandle;
use molten_core::fabric_crypto_identity::VerificationDecisionKind;
use molten_core::fabric_crypto_identity::evaluate_artifact_auth_dual_run;
use molten_core::fabric_crypto_identity::evaluate_verification;
use molten_core::fabric_crypto_identity::map_artifact_auth_statement;
use serde::Deserialize;
use serde::Serialize;

use super::IrohEd25519FileAdapter;
use crate::error::MoltenError;
use crate::error::Result;
use crate::node_state::MAX_NODE_STATE_FILE_BYTES;
use crate::node_state::NodeStateNamespace;
use crate::node_state::NodeStateNamespaceKind;
use crate::node_state::NodeStatePath;
use crate::preserves_rail::content_ref_from_bytes;

mod operational;

pub use operational::MoltenArtifactAuthOperationalReceipt;
pub use operational::artifact_auth_operational_receipt_path;
pub use operational::build_artifact_auth_operational_receipt;
pub use operational::read_artifact_auth_operational_receipt;
pub use operational::replay_artifact_auth_operational_receipt;
pub use operational::validate_artifact_auth_operational_receipt;
pub use operational::write_artifact_auth_operational_receipt;

const HEX_DIGIT_COUNT: usize = 16;
const HEX_DIGITS: &[u8; HEX_DIGIT_COUNT] = b"0123456789abcdef";
const BITS_PER_NIBBLE: u32 = 4;
const LOW_NIBBLE_MASK: u8 = 0x0f;
const HEX_CHARS_PER_BYTE: usize = 2;

#[derive(Debug, Clone, Copy)]
pub struct MoltenArtifactAuthShellInput<'a> {
    pub statement: MoltenArtifactAuthStatementInput<'a>,
    pub handle: &'a OpaqueKeyHandle,
    pub signing_policy_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedArtifactAuthStatement {
    pub statement_ref: String,
    pub public_key: String,
    pub public_key_ref: String,
    pub signature_ref: String,
    pub signature_hex: String,
    pub signature_bytes: Vec<u8>,
}

#[cfg(test)]
impl SignedArtifactAuthStatement {
    pub(crate) fn replace_signature_bytes_for_test(&mut self, signature_bytes: Vec<u8>) {
        self.signature_ref = content_ref_from_bytes(&signature_bytes);
        self.signature_hex = bytes_to_lower_hex(&signature_bytes);
        self.signature_bytes = signature_bytes;
    }

    pub(crate) fn replace_public_key_for_test(&mut self, public_key: iroh::PublicKey) {
        self.public_key_ref = content_ref_from_bytes(public_key.as_bytes());
        self.public_key = public_key.to_string();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoltenArtifactAuthShellReport {
    pub statement_ref: String,
    pub public_key_ref: String,
    pub signature_ref: String,
    pub signature_hex: String,
    pub cryptographic_failure_code: Option<String>,
    pub dual_run: MoltenArtifactAuthReport,
}

// r[impl molten.artifact_auth_shell.exact_verification]
// r[impl molten.artifact_auth_shell.evidence]
pub fn sign_artifact_auth_for_dual_run(
    adapter: &IrohEd25519FileAdapter<'_>,
    input: &MoltenArtifactAuthShellInput<'_>,
) -> Result<SignedArtifactAuthStatement> {
    admit_signing_input(input)?;
    let statement = map_statement(&input.statement)?;
    let signed = adapter.sign_artifact_auth_statement(input.handle, &statement, input.signing_policy_ref)?;
    let public_key = iroh::PublicKey::from_str(&signed.public_key)
        .map_err(|_| MoltenError::invalid_harness("artifact-auth signer returned a malformed public key"))?;
    let statement_bytes = artifact_auth_core::canonical_statement_bytes(&statement)
        .map_err(|_| MoltenError::invalid_harness("artifact-auth statement is not canonicalizable"))?;
    let result = SignedArtifactAuthStatement {
        statement_ref: content_ref_from_bytes(&statement_bytes),
        public_key: signed.public_key,
        public_key_ref: content_ref_from_bytes(public_key.as_bytes()),
        signature_ref: content_ref_from_bytes(&signed.signature_bytes),
        signature_hex: bytes_to_lower_hex(&signed.signature_bytes),
        signature_bytes: signed.signature_bytes,
    };
    debug_assert_eq!(result.signature_bytes.len(), artifact_auth_ed25519::ED25519_SIGNATURE_BYTES);
    debug_assert_eq!(result.signature_hex.len(), result.signature_bytes.len().saturating_mul(HEX_CHARS_PER_BYTE));
    Ok(result)
}

// r[impl molten.artifact_auth_shell.exact_verification]
// r[impl molten.artifact_auth_shell.evidence]
// r[impl molten.artifact_auth_shell.authority]
pub fn evaluate_artifact_auth_shell_dual_run(
    input: &MoltenArtifactAuthStatementInput<'_>,
    signed: &SignedArtifactAuthStatement,
) -> Result<MoltenArtifactAuthShellReport> {
    let statement = map_statement(input)?;
    let statement_bytes = artifact_auth_core::canonical_statement_bytes(&statement)
        .map_err(|_| MoltenError::invalid_harness("artifact-auth statement is not canonicalizable"))?;
    require_carrier_identity("statement", &signed.statement_ref, &content_ref_from_bytes(&statement_bytes))?;
    let public_key = iroh::PublicKey::from_str(&signed.public_key)
        .map_err(|_| MoltenError::invalid_harness("artifact-auth carrier public key is malformed"))?;
    require_carrier_identity("public key", &signed.public_key_ref, &content_ref_from_bytes(public_key.as_bytes()))?;
    require_carrier_identity("signature", &signed.signature_ref, &content_ref_from_bytes(&signed.signature_bytes))?;
    require_carrier_identity("signature hex", &signed.signature_hex, &bytes_to_lower_hex(&signed.signature_bytes))?;
    let cryptographic =
        artifact_auth_ed25519::verify_statement(&statement, public_key.as_bytes(), &signed.signature_bytes);
    let cryptographic_failure_code = cryptographic.failure_code.clone();
    let dual_run = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: input.profile,
        request: input.request,
        producer_id: input.producer_id,
        key_id: input.key_id,
        currentness_ref: input.currentness_ref,
        standalone_cryptographic: cryptographic,
    });
    let result = MoltenArtifactAuthShellReport {
        statement_ref: signed.statement_ref.clone(),
        public_key_ref: signed.public_key_ref.clone(),
        signature_ref: signed.signature_ref.clone(),
        signature_hex: signed.signature_hex.clone(),
        cryptographic_failure_code,
        dual_run,
    };
    debug_assert!(result.dual_run.compatibility.legacy_authoritative);
    debug_assert!(!result.dual_run.compatibility.standalone_authority_admitted);
    Ok(result)
}

fn admit_signing_input(input: &MoltenArtifactAuthShellInput<'_>) -> Result<()> {
    let request = input.statement.request;
    let legacy = evaluate_verification(input.statement.profile, request);
    if legacy.kind != VerificationDecisionKind::Accept {
        return Err(MoltenError::invalid_harness("artifact-auth signing requires an accepted legacy observation"));
    }
    if request.signer_generation != input.handle.generation || request.observed.generation != input.handle.generation {
        return Err(MoltenError::invalid_harness("artifact-auth signing generation does not match the current handle"));
    }
    if request.signer_currentness != input.handle.currentness || !input.handle.currentness.permits_signing() {
        return Err(MoltenError::invalid_harness(
            "artifact-auth signing currentness does not permit the current handle",
        ));
    }
    if input.statement.currentness_ref != input.handle.currentness_evidence_ref {
        return Err(MoltenError::invalid_harness(
            "artifact-auth signing currentness evidence does not match the current handle",
        ));
    }
    debug_assert_eq!(request.signer_generation, input.handle.generation);
    debug_assert_eq!(request.signer_currentness, input.handle.currentness);
    Ok(())
}

fn map_statement(input: &MoltenArtifactAuthStatementInput<'_>) -> Result<artifact_auth_core::ArtifactStatement> {
    map_artifact_auth_statement(input).map_err(|issues| {
        MoltenError::invalid_harness(format!("artifact-auth statement mapping failed: {}", issues.join(",")))
    })
}

fn require_carrier_identity(label: &str, observed: &str, expected: &str) -> Result<()> {
    if observed != expected {
        return Err(MoltenError::invalid_harness(format!("artifact-auth carrier {label} identity mismatch")));
    }
    Ok(())
}

fn bytes_to_lower_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(HEX_CHARS_PER_BYTE));
    for byte in bytes {
        let high = usize::from(byte >> BITS_PER_NIBBLE);
        let low = usize::from(byte & LOW_NIBBLE_MASK);
        encoded.push(char::from(HEX_DIGITS[high]));
        encoded.push(char::from(HEX_DIGITS[low]));
    }
    debug_assert_eq!(encoded.len(), bytes.len().saturating_mul(HEX_CHARS_PER_BYTE));
    encoded
}
