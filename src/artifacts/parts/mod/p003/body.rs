
fn name_pointer_value(input: &NamePointerValueInput<'_>) -> Result<IoValue> {
    validate_pointer_kind(input.pointer_kind)?;
    validate_non_empty(input.name, "artifact pointer name")?;
    validate_ref(input.artifact_ref, "artifact pointer artifact ref")?;
    if let Some(previous_ref) = input.previous_ref {
        validate_ref(previous_ref, "artifact pointer previous ref")?;
    }
    validate_refs(input.policy_refs, "artifact pointer policy ref")?;
    validate_ref(input.receipt_ref, "artifact pointer receipt ref")?;
    Ok(record("artifact-name-pointer-v1", vec![
        string(crate::preserves_rail::ARTIFACT_NAME_POINTER_SCHEMA),
        record("kind", vec![string(input.pointer_kind)]),
        record("name", vec![string(input.name)]),
        record("artifact", vec![string(input.artifact_ref)]),
        record("previous", vec![optional_ref_value(input.previous_ref)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("receipt", vec![string(input.receipt_ref)]),
        checks_value(&["names-are-metadata", "artifact-content-immutable"]),
    ]))
}

fn parse_name_pointer_value(value: &IoValue) -> Result<ArtifactNamePointer> {
    let fields = value
        .collect_simple_record("artifact-name-pointer-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-name-pointer-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::ARTIFACT_NAME_POINTER_SCHEMA, "artifact name pointer")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "names-are-metadata", "artifact name pointer")?;
    Ok(ArtifactNamePointer {
        pointer_ref: canonical_hash(value)?,
        pointer_kind: record_string(&fields[1], "kind")?,
        name: record_string(&fields[2], "name")?,
        artifact_ref: record_ref(&fields[3], "artifact")?,
        previous_ref: record_optional_ref(&fields[4], "previous")?,
        policy_refs: record_ref_sequence(&fields[5], "policy")?,
        receipt_ref: record_ref(&fields[6], "receipt")?,
        value: value.clone(),
    })
}

pub fn list_name_pointers(root: &Path) -> Result<Vec<ArtifactNamePointer>> {
    let root = open_capability_artifact_root(root)?;
    list_name_pointers_with_root(&root)
}

pub fn list_name_pointers_with_root(root: &CapabilityArtifactRoot) -> Result<Vec<ArtifactNamePointer>> {
    all_name_pointers(root)
}

fn all_name_pointers(root: &CapabilityArtifactRoot) -> Result<Vec<ArtifactNamePointer>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let names = read_txn.open_table(INDEX_NAMES).map_err(index_error)?;
    let mut pointers = Vec::new();
    for item in names.iter().map_err(index_error)? {
        let (_key, bytes) = item.map_err(index_error)?;
        push_bounded(
            &mut pointers,
            parse_name_pointer_value(&parse_canonical_bytes(bytes.value())?)?,
            MAX_ARTIFACT_POINTERS,
            "artifact name pointers",
        )?;
    }
    Ok(pointers)
}

fn closure_value(roots: &[String], closure_refs: &[String], missing_refs: &[String]) -> Result<IoValue> {
    validate_refs(roots, "artifact closure root")?;
    validate_refs(closure_refs, "artifact closure ref")?;
    validate_refs(missing_refs, "artifact closure missing ref")?;
    Ok(record("artifact-closure-v1", vec![
        string(crate::preserves_rail::ARTIFACT_CLOSURE_SCHEMA),
        refs_record("roots", &sorted_unique(roots)),
        refs_record("closure", closure_refs),
        refs_record("missing", missing_refs),
        checks_value(&["ordered-refs", "closure-hash", "missing-dependency-denial"]),
    ]))
}

struct ReleaseSnapshotSubjectInput<'a> {
    namespace_scope: &'a str,
    snapshot_id: &'a str,
    artifact_refs: &'a [String],
    artifact_set_ref: Option<&'a str>,
    dependency_closure_digest: &'a str,
    dependency_index_ref: &'a str,
    doc_refs: &'a [String],
    transcript_refs: &'a [String],
    expected_receipt_refs: &'a [String],
    policy_refs: &'a [String],
    provenance_refs: &'a [String],
    source_gate_refs: &'a [String],
    resource_refs: &'a [String],
    compatibility_refs: &'a [String],
    migration_refs: &'a [String],
    upgrade_session_refs: &'a [String],
    rollback_refs: &'a [String],
    cutover_refs: &'a [String],
    caveats: &'a [String],
    non_claims: &'a [String],
    redaction_profile_ref: Option<&'a str>,
    stale_evidence_refs: &'a [String],
}

struct ReleaseSnapshotVerifyCore {
    diagnostics: Vec<String>,
    snapshot_artifact_kind: bool,
    exact_member_closure: bool,
    dependency_index_digest: bool,
    signature_subject_bound: bool,
    caveats_rendered: bool,
    fresh_evidence: bool,
    redaction_profile_bound: bool,
    non_authority_boundary: bool,
    required_evidence_bound: bool,
}

fn release_snapshot_subject_ref(input: &ReleaseSnapshotSubjectInput<'_>) -> Result<String> {
    validate_release_snapshot_subject_input(input)?;
    canonical_hash(&record("artifact-release-snapshot-subject-v1", vec![
        record("namespace", vec![string(input.namespace_scope)]),
        record("snapshot", vec![string(input.snapshot_id)]),
        refs_record("artifacts", &sorted_unique(input.artifact_refs)),
        record("artifact-set", vec![optional_ref_value(input.artifact_set_ref)]),
        record("dependency", vec![
            record("closure-digest", vec![string(input.dependency_closure_digest)]),
            record("index", vec![string(input.dependency_index_ref)]),
        ]),
        record("documentation", vec![
            refs_record("docs", &sorted_unique(input.doc_refs)),
            refs_record("transcripts", &sorted_unique(input.transcript_refs)),
            refs_record("expected-receipts", &sorted_unique(input.expected_receipt_refs)),
        ]),
        record("evidence", vec![
            refs_record("policy", &sorted_unique(input.policy_refs)),
            refs_record("provenance", &sorted_unique(input.provenance_refs)),
            refs_record("source-gates", &sorted_unique(input.source_gate_refs)),
            refs_record("resources", &sorted_unique(input.resource_refs)),
        ]),
        record("compatibility", vec![
            refs_record("compatibility", &sorted_unique(input.compatibility_refs)),
            refs_record("migrations", &sorted_unique(input.migration_refs)),
        ]),
        record("lifecycle", vec![
            refs_record("upgrade-sessions", &sorted_unique(input.upgrade_session_refs)),
            refs_record("rollback", &sorted_unique(input.rollback_refs)),
            refs_record("cutover", &sorted_unique(input.cutover_refs)),
        ]),
        record("caveats", vec![strings_sequence(&sorted_unique(input.caveats))]),
        record("non-claims", vec![strings_sequence(&sorted_unique(input.non_claims))]),
        record("redaction", vec![optional_ref_value(input.redaction_profile_ref)]),
        refs_record("stale-evidence", &sorted_unique(input.stale_evidence_refs)),
    ]))
}

fn release_snapshot_subject_input_from_snapshot(snapshot: &ReleaseSnapshot) -> ReleaseSnapshotSubjectInput<'_> {
    ReleaseSnapshotSubjectInput {
        namespace_scope: &snapshot.namespace_scope,
        snapshot_id: &snapshot.snapshot_id,
        artifact_refs: &snapshot.artifact_refs,
        artifact_set_ref: snapshot.artifact_set_ref.as_deref(),
        dependency_closure_digest: &snapshot.dependency_closure_digest,
        dependency_index_ref: &snapshot.dependency_index_ref,
        doc_refs: &snapshot.doc_refs,
        transcript_refs: &snapshot.transcript_refs,
        expected_receipt_refs: &snapshot.expected_receipt_refs,
        policy_refs: &snapshot.policy_refs,
        provenance_refs: &snapshot.provenance_refs,
        source_gate_refs: &snapshot.source_gate_refs,
        resource_refs: &snapshot.resource_refs,
        compatibility_refs: &snapshot.compatibility_refs,
        migration_refs: &snapshot.migration_refs,
        upgrade_session_refs: &snapshot.upgrade_session_refs,
        rollback_refs: &snapshot.rollback_refs,
        cutover_refs: &snapshot.cutover_refs,
        caveats: &snapshot.caveats,
        non_claims: &snapshot.non_claims,
        redaction_profile_ref: snapshot.redaction_profile_ref.as_deref(),
        stale_evidence_refs: &snapshot.stale_evidence_refs,
    }
}

fn release_snapshot_verify_core(
    root: &CapabilityArtifactRoot,
    artifact: &ArtifactRecord,
    snapshot: &ReleaseSnapshot,
    required_caveats: &[String],
) -> Result<ReleaseSnapshotVerifyCore> {
    let mut diagnostics = Vec::new();
    let is_snapshot_artifact_kind = artifact.kind == RELEASE_SNAPSHOT_ARTIFACT_KIND;
    if !is_snapshot_artifact_kind {
        push_snapshot_diagnostic(&mut diagnostics, format!("release snapshot artifact kind was {}, expected {RELEASE_SNAPSHOT_ARTIFACT_KIND}", artifact.kind))?;
    }

    let expected_members = sorted_unique(&snapshot.artifact_refs);
    let is_exact_member_closure = exact_member_closure(root, artifact, snapshot, &expected_members, &mut diagnostics)?;

    let (dependency_index_digest, index_ref) = match release_snapshot_dependency_index_digest(root, &expected_members) {
        Ok(index_ref) => (index_ref == snapshot.dependency_index_ref, Some(index_ref)),
        Err(error) => {
            push_snapshot_diagnostic(&mut diagnostics, format!("dependency index digest could not be recomputed: {error}"))?;
            (false, None)
        }
    };
    if let Some(index_ref) = index_ref
        && index_ref != snapshot.dependency_index_ref
    {
        push_snapshot_diagnostic(&mut diagnostics, format!("dependency index digest mismatch: got {index_ref}, expected {}", snapshot.dependency_index_ref))?;
    }

    let recomputed_subject = release_snapshot_subject_ref(&release_snapshot_subject_input_from_snapshot(snapshot))?;
    let is_signature_subject_bound = recomputed_subject == snapshot.signature_subject_ref && !snapshot.signature_refs.is_empty();
    if recomputed_subject != snapshot.signature_subject_ref {
        push_snapshot_diagnostic(&mut diagnostics, format!(
                "signature subject mismatch: got {recomputed_subject}, expected {}",
                snapshot.signature_subject_ref
            ))?;
    }
    if snapshot.signature_refs.is_empty() {
        push_snapshot_diagnostic(&mut diagnostics, "release snapshot requires at least one signature ref".to_string())?;
    }

    let is_caveats_rendered = release_snapshot_caveats_rendered(snapshot, required_caveats, &mut diagnostics)?;
    let is_fresh_evidence = release_snapshot_fresh_evidence(snapshot, &mut diagnostics)?;
    let is_redaction_profile_bound = release_snapshot_redaction_bound(snapshot, &mut diagnostics)?;
    let is_required_evidence_bound = release_snapshot_required_evidence_bound(snapshot, &mut diagnostics)?;
    let is_non_authority_boundary = snapshot
        .non_claims
        .iter()
        .any(|claim| claim.contains("authority") || claim.contains("deployment") || claim.contains("execution"));
    if !is_non_authority_boundary {
        push_snapshot_diagnostic(&mut diagnostics, "release snapshot must surface non-claims for authority, deployment, or execution".to_string())?;
    }
    Ok(ReleaseSnapshotVerifyCore {
        diagnostics,
        snapshot_artifact_kind: is_snapshot_artifact_kind,
        exact_member_closure: is_exact_member_closure,
        dependency_index_digest,
        signature_subject_bound: is_signature_subject_bound,
        caveats_rendered: is_caveats_rendered,
        fresh_evidence: is_fresh_evidence,
        redaction_profile_bound: is_redaction_profile_bound,
        non_authority_boundary: is_non_authority_boundary,
        required_evidence_bound: is_required_evidence_bound,
    })
}
