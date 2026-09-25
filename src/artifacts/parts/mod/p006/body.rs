
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactImpactQueryReceipt {
    pub query_ref: String,
    pub decision: String,
    pub direct_dependents: Vec<String>,
    pub transitive_dependents: Vec<String>,
    pub redacted_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotDraftInput {
    pub namespace_scope: String,
    pub snapshot_id: String,
    pub artifact_refs: Vec<String>,
    pub artifact_set_ref: Option<String>,
    pub doc_refs: Vec<String>,
    pub transcript_refs: Vec<String>,
    pub expected_receipt_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub compatibility_refs: Vec<String>,
    pub migration_refs: Vec<String>,
    pub upgrade_session_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
    pub cutover_refs: Vec<String>,
    pub caveats: Vec<String>,
    pub non_claims: Vec<String>,
    pub redaction_profile_ref: Option<String>,
    pub signature_refs: Vec<String>,
    pub stale_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotValueInput {
    pub namespace_scope: String,
    pub snapshot_id: String,
    pub artifact_refs: Vec<String>,
    pub artifact_set_ref: Option<String>,
    pub dependency_closure_digest: String,
    pub dependency_index_ref: String,
    pub doc_refs: Vec<String>,
    pub transcript_refs: Vec<String>,
    pub expected_receipt_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub compatibility_refs: Vec<String>,
    pub migration_refs: Vec<String>,
    pub upgrade_session_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
    pub cutover_refs: Vec<String>,
    pub caveats: Vec<String>,
    pub non_claims: Vec<String>,
    pub redaction_profile_ref: Option<String>,
    pub signature_subject_ref: String,
    pub signature_refs: Vec<String>,
    pub stale_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshot {
    pub snapshot_ref: String,
    pub namespace_scope: String,
    pub snapshot_id: String,
    pub artifact_refs: Vec<String>,
    pub artifact_set_ref: Option<String>,
    pub dependency_closure_digest: String,
    pub dependency_index_ref: String,
    pub doc_refs: Vec<String>,
    pub transcript_refs: Vec<String>,
    pub expected_receipt_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub compatibility_refs: Vec<String>,
    pub migration_refs: Vec<String>,
    pub upgrade_session_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
    pub cutover_refs: Vec<String>,
    pub caveats: Vec<String>,
    pub non_claims: Vec<String>,
    pub redaction_profile_ref: Option<String>,
    pub signature_subject_ref: String,
    pub signature_refs: Vec<String>,
    pub stale_evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotInstallInput {
    pub snapshot: ReleaseSnapshotDraftInput,
    pub installer_ref: String,
    pub capability_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotInstall {
    pub artifact_ref: String,
    pub snapshot: ReleaseSnapshot,
    pub install: ArtifactInstall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotVerifyInput {
    pub snapshot_ref: String,
    pub required_caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotVerifyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub snapshot_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseChannelUpdateInput {
    pub channel: String,
    pub snapshot_ref: String,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseChannelUpdate {
    pub pointer: ArtifactNamePointer,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseChannelAdmissionInput {
    pub channel_pointer_ref: String,
    pub release_evidence_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseChannelAdmissionReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactClosure {
    pub roots: Vec<String>,
    pub closure_refs: Vec<String>,
    pub missing_refs: Vec<String>,
    pub closure_hash: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactImpact {
    pub seeds: Vec<String>,
    pub impacted_refs: Vec<String>,
    pub impact_hash: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactIndexRebuild {
    pub artifacts: usize,
    pub names: usize,
    pub receipt_value: IoValue,
}

pub fn install_artifact(root: &Path, input: &ArtifactInstallInput) -> Result<ArtifactInstall> {
    let root = open_capability_artifact_root(root)?;
    install_artifact_with_root(&root, input)
}

pub fn install_artifact_with_root(
    root: &CapabilityArtifactRoot,
    input: &ArtifactInstallInput,
) -> Result<ArtifactInstall> {
    validate_install_input(input)?;
    ensure_dirs(root)?;
    let payload = prepare_install_payload(root, &input.payload)?;
    let artifact = build_install_artifact(input, &payload.payload_ref)?;
    let identity_receipt = artifact_identity_receipt(&identity_input_from_artifact(&artifact))?;
    if identity_receipt.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "artifact identity denied: {}",
            identity_receipt.diagnostics.join("; ")
        )));
    }
    let missing_dependencies = missing_dependencies(root, &input.dependency_refs)?;
    let decision = install_decision(&missing_dependencies);
    let refs = install_refs(input, &artifact, &identity_receipt.receipt_ref, payload.chunk_receipt_ref.as_ref())?;
    let diagnostics = install_diagnostics(&missing_dependencies)?;
    let receipt_value = install_receipt_value(&artifact, decision, &refs, &diagnostics, &missing_dependencies)?;
    commit_install(root, &artifact, &payload.payload_bytes, &receipt_value, missing_dependencies.is_empty())?;
    Ok(ArtifactInstall {
        artifact_ref: artifact.artifact_ref.clone(),
        decision: decision.to_string(),
        artifact,
        identity_receipt_ref: identity_receipt.receipt_ref,
        identity_receipt_value: identity_receipt.value,
        missing_dependencies,
        receipt_value,
    })
}

struct InstallPayload {
    payload_bytes: Vec<u8>,
    payload_ref: ArtifactPayloadRef,
    chunk_receipt_ref: Option<String>,
}

fn prepare_install_payload(root: &CapabilityArtifactRoot, payload: &IoValue) -> Result<InstallPayload> {
    let payload_bytes = canonical_bytes(payload)?;
    let payload_value_ref = canonical_hash(payload)?;
    let (payload_ref, chunk_receipt_ref) = if payload_bytes.len() <= INLINE_PAYLOAD_LIMIT {
        (
            ArtifactPayloadRef::Inline {
                value_ref: payload_value_ref,
                length: payload_bytes.len() as u64,
            },
            None,
        )
    } else {
        let put = put_payload_bytes(root, &payload_bytes)?;
        (
            ArtifactPayloadRef::ContentRef {
                manifest_ref: put.manifest_ref,
                length: payload_bytes.len() as u64,
            },
            Some(canonical_hash(&put.receipt_value)?),
        )
    };
    Ok(InstallPayload {
        payload_bytes,
        payload_ref,
        chunk_receipt_ref,
    })
}

fn build_install_artifact(input: &ArtifactInstallInput, payload_ref: &ArtifactPayloadRef) -> Result<ArtifactRecord> {
    let value = artifact_value(ArtifactValueInput {
        kind: &input.kind,
        payload: payload_ref,
        schema_refs: &input.schema_refs,
        dependency_refs: &input.dependency_refs,
        effect_manifest_ref: input.effect_manifest_ref.as_deref(),
        policy_refs: &input.policy_refs,
        evidence_refs: &input.evidence_refs,
    })?;
    parse_artifact_value(&value)
}

fn install_decision(missing_dependencies: &[String]) -> &'static str {
    if missing_dependencies.is_empty() {
        "pass"
    } else {
        "deny"
    }
}
