
/// Computes the dependency closure of `artifact_refs` and requires it to be complete and equal to the refs.
fn require_exact_closure_refs(root: &CapabilityArtifactRoot, artifact_refs: &[String]) -> Result<Vec<String>> {
    let (closure_refs, missing_refs) = compute_closure_refs(root, artifact_refs)?;
    if !missing_refs.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "release snapshot cannot bind missing closure refs: {}",
            missing_refs.join(", ")
        )));
    }
    if closure_refs != artifact_refs {
        return Err(MoltenError::invalid_harness(format!(
            "release snapshot artifact refs must equal dependency closure: expected {}, got {}",
            closure_refs.join(", "),
            artifact_refs.join(", ")
        )));
    }
    Ok(closure_refs)
}

pub fn release_snapshot_value(input: &ReleaseSnapshotValueInput) -> Result<IoValue> {
    validate_release_snapshot_value_input(input)?;
    Ok(record("artifact-release-snapshot-v1", vec![
        string(crate::preserves_rail::ARTIFACT_RELEASE_SNAPSHOT_SCHEMA),
        record("namespace", vec![string(&input.namespace_scope)]),
        record("snapshot", vec![string(&input.snapshot_id)]),
        refs_record("artifacts", &sorted_unique(&input.artifact_refs)),
        record("artifact-set", vec![optional_ref_value(input.artifact_set_ref.as_deref())]),
        record("dependency", vec![
            record("closure-digest", vec![string(&input.dependency_closure_digest)]),
            record("index", vec![string(&input.dependency_index_ref)]),
        ]),
        record("documentation", vec![
            refs_record("docs", &sorted_unique(&input.doc_refs)),
            refs_record("transcripts", &sorted_unique(&input.transcript_refs)),
            refs_record("expected-receipts", &sorted_unique(&input.expected_receipt_refs)),
        ]),
        record("evidence", vec![
            refs_record("policy", &sorted_unique(&input.policy_refs)),
            refs_record("provenance", &sorted_unique(&input.provenance_refs)),
            refs_record("source-gates", &sorted_unique(&input.source_gate_refs)),
            refs_record("resources", &sorted_unique(&input.resource_refs)),
        ]),
        record("compatibility", vec![
            refs_record("compatibility", &sorted_unique(&input.compatibility_refs)),
            refs_record("migrations", &sorted_unique(&input.migration_refs)),
        ]),
        record("lifecycle", vec![
            refs_record("upgrade-sessions", &sorted_unique(&input.upgrade_session_refs)),
            refs_record("rollback", &sorted_unique(&input.rollback_refs)),
            refs_record("cutover", &sorted_unique(&input.cutover_refs)),
        ]),
        record("caveats", vec![strings_sequence(&sorted_unique(&input.caveats))]),
        record("non-claims", vec![strings_sequence(&sorted_unique(&input.non_claims))]),
        record("redaction", vec![optional_ref_value(input.redaction_profile_ref.as_deref())]),
        record("signatures", vec![
            record("subject", vec![string(&input.signature_subject_ref)]),
            refs_record("signatures", &sorted_unique(&input.signature_refs)),
        ]),
        refs_record("stale-evidence", &sorted_unique(&input.stale_evidence_refs)),
        checks_value(&[
            "immutable-snapshot-artifact",
            "exact-artifact-refs",
            "dependency-closure-digest-bound",
            "dependency-index-bound",
            "channel-names-are-non-authority",
            "caveats-and-non-claims-rendered",
        ]),
    ]))
}

pub fn parse_release_snapshot_value(value: &IoValue) -> Result<ReleaseSnapshot> {
    let fields = value
        .collect_simple_record("artifact-release-snapshot-v1", Some(RELEASE_SNAPSHOT_RECORD_ARITY))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-release-snapshot-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::ARTIFACT_RELEASE_SNAPSHOT_SCHEMA, "release snapshot")?;
    let checks = parse_checks(&fields[15])?;
    require_check(&checks, "immutable-snapshot-artifact", "release snapshot")?;
    let dependency_value = value_to_iovalue(&fields[5]);
    let documentation_value = value_to_iovalue(&fields[6]);
    let evidence_value = value_to_iovalue(&fields[7]);
    let compatibility_value = value_to_iovalue(&fields[8]);
    let lifecycle_value = value_to_iovalue(&fields[9]);
    let signature_value = value_to_iovalue(&fields[13]);
    let dependency_fields = simple_record(&dependency_value, "dependency", 2)?;
    let documentation_fields = simple_record(&documentation_value, "documentation", 3)?;
    let evidence_fields = simple_record(&evidence_value, "evidence", 4)?;
    let compatibility_fields = simple_record(&compatibility_value, "compatibility", 2)?;
    let lifecycle_fields = simple_record(&lifecycle_value, "lifecycle", 3)?;
    let signature_fields = simple_record(&signature_value, "signatures", 2)?;
    Ok(ReleaseSnapshot {
        snapshot_ref: canonical_hash(value)?,
        namespace_scope: record_string(&fields[1], "namespace")?,
        snapshot_id: record_string(&fields[2], "snapshot")?,
        artifact_refs: sorted_unique(&record_ref_sequence(&fields[3], "artifacts")?),
        artifact_set_ref: record_optional_ref(&fields[4], "artifact-set")?,
        dependency_closure_digest: record_ref(&dependency_fields[0], "closure-digest")?,
        dependency_index_ref: record_ref(&dependency_fields[1], "index")?,
        doc_refs: sorted_unique(&record_ref_sequence(&documentation_fields[0], "docs")?),
        transcript_refs: sorted_unique(&record_ref_sequence(&documentation_fields[1], "transcripts")?),
        expected_receipt_refs: sorted_unique(&record_ref_sequence(&documentation_fields[2], "expected-receipts")?),
        policy_refs: sorted_unique(&record_ref_sequence(&evidence_fields[0], "policy")?),
        provenance_refs: sorted_unique(&record_ref_sequence(&evidence_fields[1], "provenance")?),
        source_gate_refs: sorted_unique(&record_ref_sequence(&evidence_fields[2], "source-gates")?),
        resource_refs: sorted_unique(&record_ref_sequence(&evidence_fields[3], "resources")?),
        compatibility_refs: sorted_unique(&record_ref_sequence(&compatibility_fields[0], "compatibility")?),
        migration_refs: sorted_unique(&record_ref_sequence(&compatibility_fields[1], "migrations")?),
        upgrade_session_refs: sorted_unique(&record_ref_sequence(&lifecycle_fields[0], "upgrade-sessions")?),
        rollback_refs: sorted_unique(&record_ref_sequence(&lifecycle_fields[1], "rollback")?),
        cutover_refs: sorted_unique(&record_ref_sequence(&lifecycle_fields[2], "cutover")?),
        caveats: sorted_unique(&record_strings(&fields[10], "caveats")?),
        non_claims: sorted_unique(&record_strings(&fields[11], "non-claims")?),
        redaction_profile_ref: record_optional_ref(&fields[12], "redaction")?,
        signature_subject_ref: record_ref(&signature_fields[0], "subject")?,
        signature_refs: sorted_unique(&record_ref_sequence(&signature_fields[1], "signatures")?),
        stale_evidence_refs: sorted_unique(&record_ref_sequence(&fields[14], "stale-evidence")?),
        value: value.clone(),
    })
}

pub fn install_release_snapshot(root: &Path, input: &ReleaseSnapshotInstallInput) -> Result<ReleaseSnapshotInstall> {
    let root = open_capability_artifact_root(root)?;
    install_release_snapshot_with_root(&root, input)
}

pub fn install_release_snapshot_with_root(
    root: &CapabilityArtifactRoot,
    input: &ReleaseSnapshotInstallInput,
) -> Result<ReleaseSnapshotInstall> {
    validate_ref(&input.installer_ref, "release snapshot installer ref")?;
    if input.capability_refs.is_empty() {
        return Err(MoltenError::invalid_harness("release snapshot install requires at least one capability ref"));
    }
    validate_refs(&input.capability_refs, "release snapshot capability ref")?;
    let snapshot_input = release_snapshot_value_input_with_root(root, &input.snapshot)?;
    let payload = release_snapshot_value(&snapshot_input)?;
    let evidence_refs = release_snapshot_install_evidence_refs(&snapshot_input)?;
    let install = install_artifact_with_root(root, &ArtifactInstallInput {
        kind: RELEASE_SNAPSHOT_ARTIFACT_KIND.to_string(),
        payload: payload.clone(),
        schema_refs: Vec::new(),
        dependency_refs: snapshot_input.artifact_refs.clone(),
        effect_manifest_ref: None,
        policy_refs: snapshot_input.policy_refs.clone(),
        evidence_refs,
        installer_ref: input.installer_ref.clone(),
        capability_refs: input.capability_refs.clone(),
    })?;
    Ok(ReleaseSnapshotInstall {
        artifact_ref: install.artifact_ref.clone(),
        snapshot: parse_release_snapshot_value(&payload)?,
        install,
    })
}

// r[impl molten.release_snapshots.closure_integrity]
pub fn verify_release_snapshot(root: &Path, input: &ReleaseSnapshotVerifyInput) -> Result<ReleaseSnapshotVerifyReceipt> {
    let root = open_capability_artifact_root(root)?;
    verify_release_snapshot_with_root(&root, input)
}

pub fn verify_release_snapshot_with_root(
    root: &CapabilityArtifactRoot,
    input: &ReleaseSnapshotVerifyInput,
) -> Result<ReleaseSnapshotVerifyReceipt> {
    validate_ref(&input.snapshot_ref, "release snapshot ref")?;
    validate_strings(&input.required_caveats, "release snapshot required caveat")?;
    let artifact = read_artifact_with_root(root, &input.snapshot_ref)?;
    let payload = read_payload_with_root(root, &input.snapshot_ref)?;
    let snapshot = parse_release_snapshot_value(&payload)?;
    let core = release_snapshot_verify_core(root, &artifact, &snapshot, &input.required_caveats)?;
    let decision = if core.diagnostics.is_empty() { "pass" } else { "deny" };
    let refs = release_snapshot_verify_refs(&input.snapshot_ref, &snapshot)?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "release-snapshot-verify",
        decision,
        subject_ref: &input.snapshot_ref,
        name: Some(&snapshot.snapshot_id),
        refs: &refs,
        diagnostics: &core.diagnostics,
        checks: &[
            ("snapshot-artifact-kind", pass_fail(core.snapshot_artifact_kind)),
            ("exact-member-closure", pass_fail(core.exact_member_closure)),
            ("dependency-index-digest", pass_fail(core.dependency_index_digest)),
            ("signature-subject-bound", pass_fail(core.signature_subject_bound)),
            ("caveats-rendered", pass_fail(core.caveats_rendered)),
            ("fresh-evidence", pass_fail(core.fresh_evidence)),
            ("redaction-profile-bound", pass_fail(core.redaction_profile_bound)),
            ("non-authority-boundary", pass_fail(core.non_authority_boundary)),
            ("required-evidence-bound", pass_fail(core.required_evidence_bound)),
        ],
    })?;
    store_receipt(root, &receipt_value)?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(ReleaseSnapshotVerifyReceipt {
        receipt_ref,
        decision: decision.to_string(),
        snapshot_ref: input.snapshot_ref.clone(),
        diagnostics: core.diagnostics,
        value: receipt_value,
    })
}

// r[impl molten.release_snapshots.channel_view_non_authority]
pub fn set_release_channel(root: &Path, input: &ReleaseChannelUpdateInput) -> Result<ReleaseChannelUpdate> {
    let root = open_capability_artifact_root(root)?;
    set_release_channel_with_root(&root, input)
}

pub fn set_release_channel_with_root(
    root: &CapabilityArtifactRoot,
    input: &ReleaseChannelUpdateInput,
) -> Result<ReleaseChannelUpdate> {
    validate_release_channel_update_input(input)?;
    let artifact = read_artifact_with_root(root, &input.snapshot_ref)?;
    if artifact.kind != RELEASE_SNAPSHOT_ARTIFACT_KIND {
        return Err(MoltenError::invalid_harness(format!(
            "release channel target {} is {}, expected {RELEASE_SNAPSHOT_ARTIFACT_KIND}",
            input.snapshot_ref, artifact.kind
        )));
    }
    parse_release_snapshot_value(&read_payload_with_root(root, &input.snapshot_ref)?)?;
    let pointer = set_name_pointer_with_root(root, &SetNamePointerInput {
        pointer_kind: "channel",
        name: &input.channel,
        artifact_ref: &input.snapshot_ref,
        policy_refs: &input.policy_refs,
        evidence_refs: &input.evidence_refs,
    })?;
    let refs = release_channel_update_refs(input, &pointer)?;
    let diagnostics = vec!["release channels are mutable non-authority views over immutable snapshot refs".to_string()];
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "release-channel-update",
        decision: "pass",
        subject_ref: &pointer.pointer_ref,
        name: Some(&input.channel),
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &[
            ("channel-is-mutable-view", "pass"),
            ("snapshot-immutable-artifact", "pass"),
            ("channel-is-not-authority", "pass"),
            ("capability-check", "pass"),
            ("policy-check", "pass"),
        ],
    })?;
    store_receipt(root, &receipt_value)?;
    Ok(ReleaseChannelUpdate {
        pointer,
        receipt_ref: canonical_hash(&receipt_value)?,
        receipt_value,
    })
}

pub fn release_channel_admission_receipt(
    input: &ReleaseChannelAdmissionInput,
) -> Result<ReleaseChannelAdmissionReceipt> {
    validate_release_channel_admission_input(input)?;
    let diagnostics = release_channel_admission_diagnostics(input)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let refs = release_channel_admission_refs(input)?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "release-channel-admission",
        decision,
        subject_ref: &input.channel_pointer_ref,
        name: None,
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &[
            ("channel-is-not-authority", "pass"),
            ("release-evidence-bound", pass_fail(!input.release_evidence_refs.is_empty())),
            ("policy-bound", pass_fail(!input.policy_refs.is_empty())),
            ("provenance-bound", pass_fail(!input.provenance_refs.is_empty())),
            ("source-gate-bound", pass_fail(!input.source_gate_refs.is_empty())),
            ("authority-bound", pass_fail(!input.authority_refs.is_empty())),
            ("resource-bound", pass_fail(!input.resource_refs.is_empty())),
        ],
    })?;
    Ok(ReleaseChannelAdmissionReceipt {
        receipt_ref: canonical_hash(&receipt_value)?,
        decision: decision.to_string(),
        diagnostics,
        value: receipt_value,
    })
}

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<ArtifactReceipt> {
    let root = open_capability_artifact_root(root)?;
    read_receipt_with_root(&root, receipt_ref)
}
