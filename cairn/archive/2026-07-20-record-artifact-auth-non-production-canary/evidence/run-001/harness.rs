use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use molten::fabric_crypto_identity::CanonicalCryptoProfile;
use molten::fabric_crypto_identity::CanonicalSignatureDomain;
use molten::fabric_crypto_identity::IrohEd25519FileAdapter;
use molten::fabric_crypto_identity::KeyBackendClass;
use molten::fabric_crypto_identity::KeyPurpose;
use molten::fabric_crypto_identity::KeyRotationRequest;
use molten::fabric_crypto_identity::MoltenArtifactAuthOperationalReceipt;
use molten::fabric_crypto_identity::MoltenArtifactAuthShellInput;
use molten::fabric_crypto_identity::MoltenArtifactAuthStatementInput;
use molten::fabric_crypto_identity::ResolvedProductionKey;
use molten::fabric_crypto_identity::RotationOverlapPolicy;
use molten::fabric_crypto_identity::SIGNATURE_DOMAIN_SCHEMA;
use molten::fabric_crypto_identity::SignatureDomain;
use molten::fabric_crypto_identity::VerificationDecisionKind;
use molten::fabric_crypto_identity::VerificationRequest;
use molten::fabric_crypto_identity::artifact_auth_operational_receipt_path;
use molten::fabric_crypto_identity::build_artifact_auth_operational_receipt;
use molten::fabric_crypto_identity::canonical_crypto_profile;
use molten::fabric_crypto_identity::canonical_signature_domain;
use molten::fabric_crypto_identity::evaluate_artifact_auth_shell_dual_run;
use molten::fabric_crypto_identity::production_ed25519_profile;
use molten::fabric_crypto_identity::replay_artifact_auth_operational_receipt;
use molten::fabric_crypto_identity::sign_artifact_auth_for_dual_run;
use molten::fabric_crypto_identity::sign_evidence_payload;
use molten::fabric_crypto_identity::write_artifact_auth_operational_receipt;
use molten::node_state::NodeStateRoot;
use serde::Serialize;

const PRODUCER_ID: &str = "molten";
const KEY_ID: &str = "evidence-signing-key";
const RECEIPT_FILE: &str = "operational-receipt.json";

struct OperationalMaterial {
    request: VerificationRequest,
    signing_policy_ref: String,
    receipt: MoltenArtifactAuthOperationalReceipt,
}

impl OperationalMaterial {
    fn input<'a>(
        &'a self,
        adapter: &'a IrohEd25519FileAdapter<'a>,
        key: &'a ResolvedProductionKey,
    ) -> MoltenArtifactAuthShellInput<'a> {
        MoltenArtifactAuthShellInput {
            statement: MoltenArtifactAuthStatementInput {
                profile: &adapter.profile().profile,
                request: &self.request,
                producer_id: PRODUCER_ID,
                key_id: KEY_ID,
                currentness_ref: &key.handle.handle.currentness_evidence_ref,
            },
            handle: &key.handle.handle,
            signing_policy_ref: &self.signing_policy_ref,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args().collect::<Vec<_>>();
    if arguments.len() != 4 {
        return Err(
            "usage: artifact_auth_canary <capture|replay|rotate|post-rotation-status> <state-dir> <evidence-dir>"
                .into(),
        );
    }
    let state_dir = PathBuf::from(&arguments[2]);
    let evidence_dir = PathBuf::from(&arguments[3]);
    std::fs::create_dir_all(&evidence_dir)?;
    match arguments[1].as_str() {
        "capture" => capture(&state_dir, &evidence_dir),
        "replay" => replay(&state_dir, &evidence_dir),
        "rotate" => rotate(&state_dir, &evidence_dir),
        "post-rotation-status" => post_rotation_status(&state_dir, &evidence_dir),
        mode => Err(format!("unknown canary mode: {mode}").into()),
    }
}

fn capture(state_dir: &Path, evidence_dir: &Path) -> Result<(), Box<dyn Error>> {
    if state_dir.exists() {
        return Err("Molten canary state already exists".into());
    }
    let root = NodeStateRoot::open(state_dir)?;
    root.create_layout()?;
    let secrets = root.secrets()?;
    let receipts = root.receipts()?;
    let adapter = adapter(&secrets)?;
    let generation_policy_ref = reference("generation-policy");
    let key = adapter.resolve_or_generate(KeyPurpose::EvidenceSigning, &generation_policy_ref, true)?;
    let material = operational_material(&adapter, &key)?;
    let path = write_artifact_auth_operational_receipt(&receipts, &material.receipt)?;
    write_json(&evidence_dir.join(RECEIPT_FILE), &material.receipt)?;
    write_json(
        &evidence_dir.join("capture-summary.json"),
        &serde_json::json!({
            "schema": "molten-artifact-auth-canary-summary-v1",
            "phase": "capture",
            "result": "pass",
            "node_receipt_path": path.as_path().display().to_string(),
            "statement_ref": material.receipt.statement_ref,
            "receipt_blake3": material.receipt.receipt_blake3,
            "key_generation": key.handle.handle.generation,
            "legacy_authoritative": material.receipt.legacy_authoritative,
            "standalone_authority_admitted": material.receipt.standalone_authority_admitted,
            "rollback_available": material.receipt.rollback_available,
            "non_claim": "non-production node-state canary evidence does not grant membership, capability, federation, transport, lifecycle, or release authority"
        }),
    )?;
    Ok(())
}

fn replay(state_dir: &Path, evidence_dir: &Path) -> Result<(), Box<dyn Error>> {
    let root = NodeStateRoot::open_existing(state_dir)?;
    let secrets = root.secrets()?;
    let receipts = root.receipts()?;
    let adapter = adapter(&secrets)?;
    let generation_policy_ref = reference("generation-policy");
    let key = adapter.resolve_or_generate(KeyPurpose::EvidenceSigning, &generation_policy_ref, false)?;
    let material = operational_material(&adapter, &key)?;
    let path = artifact_auth_operational_receipt_path(&material.receipt.statement_ref)?;
    let report = replay_artifact_auth_operational_receipt(&adapter, &receipts, &path, &material.input(&adapter, &key))?;
    write_json(
        &evidence_dir.join("replay-summary.json"),
        &serde_json::json!({
            "schema": "molten-artifact-auth-canary-summary-v1",
            "phase": "restart-replay",
            "result": "pass",
            "statement_ref": material.receipt.statement_ref,
            "standalone_passed": report.dual_run.standalone.as_ref().is_some_and(|decision| decision.passed),
            "legacy_authoritative": report.dual_run.compatibility.legacy_authoritative,
            "standalone_authority_admitted": report.dual_run.compatibility.standalone_authority_admitted,
            "rollback_available": report.dual_run.compatibility.rollback_available
        }),
    )?;
    Ok(())
}

fn rotate(state_dir: &Path, evidence_dir: &Path) -> Result<(), Box<dyn Error>> {
    let root = NodeStateRoot::open_existing(state_dir)?;
    let secrets = root.secrets()?;
    let receipts = root.receipts()?;
    let adapter = adapter(&secrets)?;
    let generation_policy_ref = reference("generation-policy");
    let key = adapter.resolve_or_generate(KeyPurpose::EvidenceSigning, &generation_policy_ref, false)?;
    let material = operational_material(&adapter, &key)?;
    let path = artifact_auth_operational_receipt_path(&material.receipt.statement_ref)?;
    let next_generation = key.handle.handle.generation.checked_add(1).ok_or("key generation overflow")?;
    let rotation = KeyRotationRequest {
        operation_id: "artifact-auth-canary-rotation".to_owned(),
        profile_ref: adapter.profile().profile.profile_ref.clone(),
        purpose: KeyPurpose::EvidenceSigning,
        backend_class: KeyBackendClass::CapabilityFile,
        backend_ref: key.handle.handle.backend_ref.clone(),
        old_handle_ref: key.handle.handle.handle_ref.clone(),
        old_public_key_ref: key.handle.handle.public_key_ref.clone(),
        old_generation: key.handle.handle.generation,
        new_generation: next_generation,
        policy_ref: reference("rotation-policy"),
        activation_boundary_ref: reference("rotation-activation-boundary"),
        overlap: RotationOverlapPolicy::None,
        revocation_evidence_ref: Some(reference("rotation-revocation-evidence")),
    };
    adapter.rotate(&rotation)?;
    let error = replay_artifact_auth_operational_receipt(&adapter, &receipts, &path, &material.input(&adapter, &key))
        .expect_err("rotated capability-file key must deny old receipt replay");
    write_json(
        &evidence_dir.join("rotation-summary.json"),
        &serde_json::json!({
            "schema": "molten-artifact-auth-canary-summary-v1",
            "phase": "rotation",
            "result": "expected-denial",
            "old_generation": key.handle.handle.generation,
            "new_generation": next_generation,
            "error": error.to_string(),
            "standalone_authority_admitted": false,
            "rollback_available": true
        }),
    )?;
    Ok(())
}

fn post_rotation_status(state_dir: &Path, evidence_dir: &Path) -> Result<(), Box<dyn Error>> {
    let root = NodeStateRoot::open_existing(state_dir)?;
    let secrets = root.secrets()?;
    let adapter = adapter(&secrets)?;
    let generation_policy_ref = reference("generation-policy");
    let current = adapter.resolve_or_generate(KeyPurpose::EvidenceSigning, &generation_policy_ref, false)?;
    let receipt = serde_json::from_slice::<MoltenArtifactAuthOperationalReceipt>(&std::fs::read(
        evidence_dir.join(RECEIPT_FILE),
    )?)?;
    if current.handle.handle.generation == receipt.key_generation
        || current.handle.handle.handle_ref == receipt.key_handle_ref
    {
        return Err("fresh-process key state did not retain the completed rotation".into());
    }
    write_json(
        &evidence_dir.join("post-rotation-status-summary.json"),
        &serde_json::json!({
            "schema": "molten-artifact-auth-canary-summary-v1",
            "phase": "fresh-process-post-rotation-status",
            "result": "pass",
            "receipt_generation": receipt.key_generation,
            "current_generation": current.handle.handle.generation,
            "handle_changed": true,
            "standalone_authority_admitted": false,
            "rollback_available": true
        }),
    )?;
    Ok(())
}

fn operational_material(
    adapter: &IrohEd25519FileAdapter<'_>,
    key: &ResolvedProductionKey,
) -> Result<OperationalMaterial, Box<dyn Error>> {
    let signed_domain = domain(
        adapter.profile(),
        KeyPurpose::EvidenceSigning,
        &key.handle.handle.public_key_ref,
        "artifact-auth-canary-payload",
    )?;
    let legacy_sign_policy_ref = reference("legacy-sign-policy");
    let legacy_signature = sign_evidence_payload(adapter, &key.handle.handle, &signed_domain, &legacy_sign_policy_ref)?;
    let verify_policy_ref = reference("legacy-verify-policy");
    let verification = adapter.verify(
        &key.public_key,
        &signed_domain,
        &legacy_signature,
        key.handle.handle.currentness,
        key.handle.handle.generation,
        &verify_policy_ref,
    )?;
    let request = VerificationRequest {
        operation_id: "verify-artifact-auth-canary".to_owned(),
        profile_ref: adapter.profile().profile.profile_ref.clone(),
        expected_domain: signed_domain.domain,
        observed: legacy_signature.metadata,
        cryptographic_verification_passed: verification.decision.kind == VerificationDecisionKind::Accept,
        signer_currentness: key.handle.handle.currentness,
        signer_generation: key.handle.handle.generation,
        policy_ref: verify_policy_ref,
    };
    let signing_policy_ref = reference("artifact-auth-sign-policy");
    let statement = MoltenArtifactAuthStatementInput {
        profile: &adapter.profile().profile,
        request: &request,
        producer_id: PRODUCER_ID,
        key_id: KEY_ID,
        currentness_ref: &key.handle.handle.currentness_evidence_ref,
    };
    let input = MoltenArtifactAuthShellInput {
        statement,
        handle: &key.handle.handle,
        signing_policy_ref: &signing_policy_ref,
    };
    let signed = sign_artifact_auth_for_dual_run(adapter, &input)?;
    let report = evaluate_artifact_auth_shell_dual_run(&input.statement, &signed)?;
    let receipt = build_artifact_auth_operational_receipt(&input, &signed, &report)?;
    Ok(OperationalMaterial {
        request,
        signing_policy_ref,
        receipt,
    })
}

fn adapter<'a>(
    namespace: &'a molten::node_state::NodeStateNamespace,
) -> Result<IrohEd25519FileAdapter<'a>, Box<dyn Error>> {
    Ok(IrohEd25519FileAdapter::new(namespace, profile()?, reference("capability-file-backend"))?)
}

fn profile() -> Result<CanonicalCryptoProfile, Box<dyn Error>> {
    Ok(canonical_crypto_profile(&production_ed25519_profile(
        reference("production-profile"),
        reference("os-csprng-entropy"),
    ))?)
}

fn domain(
    profile: &CanonicalCryptoProfile,
    purpose: KeyPurpose,
    public_key_ref: &str,
    payload_label: &str,
) -> Result<CanonicalSignatureDomain, Box<dyn Error>> {
    Ok(canonical_signature_domain(profile, &SignatureDomain {
        schema: SIGNATURE_DOMAIN_SCHEMA.to_owned(),
        domain_id: format!("{}-domain", purpose.as_str()),
        domain_version: profile.profile.domain_version.clone(),
        purpose,
        payload_schema: "canonical-preserves-payload-v1".to_owned(),
        payload_ref: reference(payload_label),
        signer_public_ref: public_key_ref.to_owned(),
        verifier_context_ref: reference("verifier-context"),
    })?)
}

fn reference(label: &str) -> String {
    molten::codec::content_ref_from_bytes(label.as_bytes())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
