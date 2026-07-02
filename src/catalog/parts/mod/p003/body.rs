
fn registry_summary(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    artifact: crate::artifacts::ArtifactRecord,
    visibility: &VisibilityInput,
) -> Result<Summary> {
    let payload_ref = payload_identity(&artifact.payload);
    let mut name_refs = Vec::new();
    for pointer in crate::artifacts::list_name_pointers(registry_root)? {
        if pointer.artifact_ref == artifact.artifact_ref {
            push_bounded(&mut name_refs, pointer.pointer_ref, MAX_CATALOG_REFS, "catalog name refs")?;
        }
    }
    let dependent_refs = direct_dependents(registry_root, &artifact.artifact_ref)?;
    let mut classifications = Vec::new();
    push_bounded(&mut classifications, "registry-artifact".to_string(), MAX_CATALOG_REFS, "catalog classifications")?;
    push_bounded(
        &mut classifications,
        format!("artifact-kind:{}", artifact.kind),
        MAX_CATALOG_REFS,
        "catalog classifications",
    )?;
    if let Ok(payload) = crate::artifacts::read_payload(registry_root, &artifact.artifact_ref) {
        for classification in known_classifications(&payload) {
            push_bounded(&mut classifications, classification, MAX_CATALOG_REFS, "catalog classifications")?;
        }
    }
    if let Some(ledger_root) = ledger_root
        && let Ok(value) = crate::ledger::read_artifact(ledger_root, &artifact.artifact_ref)
    {
        push_bounded(
            &mut classifications,
            format!("ledger-kind:{}", crate::ledger::artifact_kind(&value)),
            MAX_CATALOG_REFS,
            "catalog classifications",
        )?;
    }
    let value = build_summary_value(&SummaryValueInput {
        artifact_ref: &artifact.artifact_ref,
        artifact_kind: &artifact.kind,
        payload_ref: &payload_ref,
        name_refs: &name_refs,
        schema_refs: &artifact.schema_refs,
        dependency_refs: &artifact.dependency_refs,
        dependent_refs: &dependent_refs,
        effect_manifest_ref: artifact.effect_manifest_ref.as_deref(),
        policy_refs: &artifact.policy_refs,
        evidence_refs: &artifact.evidence_refs,
        classifications: &classifications,
        visibility_decision: "visible",
        redaction_profile_ref: visibility.redaction_profile_ref.as_deref(),
    })?;
    Ok(Summary {
        artifact_ref: artifact.artifact_ref,
        artifact_kind: artifact.kind,
        payload_ref,
        name_refs,
        schema_refs: artifact.schema_refs,
        dependency_refs: artifact.dependency_refs,
        dependent_refs,
        effect_manifest_ref: artifact.effect_manifest_ref,
        policy_refs: artifact.policy_refs,
        evidence_refs: artifact.evidence_refs,
        classifications,
        visibility_decision: "visible".to_string(),
        value,
    })
}

fn ledger_summary(
    registry_root: &Path,
    ledger_root: &Path,
    artifact_ref: &str,
    value: IoValue,
    visibility: &VisibilityInput,
) -> Result<Summary> {
    let kind = crate::ledger::artifact_kind(&value).to_string();
    let mut classifications = Vec::new();
    push_bounded(&mut classifications, "ledger-artifact".to_string(), MAX_CATALOG_REFS, "catalog classifications")?;
    push_bounded(&mut classifications, format!("ledger-kind:{kind}"), MAX_CATALOG_REFS, "catalog classifications")?;
    for classification in known_classifications(&value) {
        push_bounded(&mut classifications, classification, MAX_CATALOG_REFS, "catalog classifications")?;
    }
    let dependent_refs = crate::artifacts::impact_refs(registry_root, &[artifact_ref.to_string()]).unwrap_or_default();
    let mut name_refs = Vec::new();
    for pointer in crate::artifacts::list_name_pointers(registry_root).unwrap_or_default() {
        if pointer.artifact_ref == artifact_ref {
            push_bounded(&mut name_refs, pointer.pointer_ref, MAX_CATALOG_REFS, "catalog name refs")?;
        }
    }
    let value = build_summary_value(&SummaryValueInput {
        artifact_ref,
        artifact_kind: &kind,
        payload_ref: artifact_ref,
        name_refs: &name_refs,
        schema_refs: &[],
        dependency_refs: &[],
        dependent_refs: &dependent_refs,
        effect_manifest_ref: None,
        policy_refs: &[],
        evidence_refs: &[],
        classifications: &classifications,
        visibility_decision: "visible",
        redaction_profile_ref: visibility.redaction_profile_ref.as_deref(),
    })?;
    let _ = ledger_root;
    Ok(Summary {
        artifact_ref: artifact_ref.to_string(),
        artifact_kind: kind,
        payload_ref: artifact_ref.to_string(),
        name_refs,
        schema_refs: Vec::new(),
        dependency_refs: Vec::new(),
        dependent_refs,
        effect_manifest_ref: None,
        policy_refs: Vec::new(),
        evidence_refs: Vec::new(),
        classifications,
        visibility_decision: "visible".to_string(),
        value,
    })
}

fn known_classifications(value: &IoValue) -> Vec<String> {
    known_classifications_result(value).unwrap_or_default()
}

type ClassificationProbe = fn(&IoValue) -> Result<Option<Vec<String>>>;

const CLASSIFICATION_PROBES: &[ClassificationProbe] = &[
    direct_labels,
    release_labels,
    retention_core_labels,
    retention_plan_apply_labels,
    retention_execute_audit_labels,
    candidate_labels,
    retention_tail_labels,
    lifecycle_labels,
    provenance_labels,
    octet_evidence_labels,
    octet_baseline_labels,
    octet_gate_labels,
];

fn known_classifications_result(value: &IoValue) -> Result<Vec<String>> {
    for probe in CLASSIFICATION_PROBES {
        if let Some(classifications) = (*probe)(value)? {
            return Ok(classifications);
        }
    }
    Ok(Vec::new())
}

fn direct_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Ok(receipt) = crate::artifacts::parse_artifact_receipt(value) {
        return Ok(Some(vec![
            "artifact-receipt:registry".to_string(),
            format!("receipt-operation:{}", receipt.operation),
            format!("receipt-decision:{}", receipt.decision),
        ]));
    }
    if let Ok(receipt) = crate::transcripts::parse_transcript_run_receipt(value) {
        return Ok(Some(vec![
            "transcript:run-receipt".to_string(),
            format!("transcript-status:{}", receipt.decision),
            format!("transcript-mode:{}", receipt.mode),
        ]));
    }
    if let Some(fields) = value.collect_simple_record("deterministic-replay-verify-v1", Some(7)) {
        return deterministic_replay_verify_gate_classifications(&fields).map(Some);
    }
    if let Some(fields) = value.collect_simple_record("deterministic-replay-verify-v1", Some(13)) {
        return deterministic_replay_verify_fixture_classifications(&fields).map(Some);
    }
    if let Some(fields) = value.collect_simple_record("deterministic-first-divergence-v1", Some(9)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA,
            "deterministic first divergence",
        )?;
        let kind = record_string(&fields[1], "kind")?;
        let actor_id = record_string(&fields[3], "actor-id")?;
        let handler_profile_ref = record_string(&fields[5], "handler-profile-ref")?;
        let expected_ref = record_string(&fields[6], "expected-ref")?;
        let actual_ref = record_string(&fields[7], "actual-ref")?;
        return Ok(Some(vec![
            "deterministic-replay:first-divergence".to_string(),
            format!("replay-divergence:{kind}"),
            format!("replay-actor:{actor_id}"),
            format!("replay-handler-profile:{handler_profile_ref}"),
            format!("replay-expected-ref:{expected_ref}"),
            format!("replay-actual-ref:{actual_ref}"),
        ]));
    }
    if let Some(fields) = value.collect_simple_record("deterministic-replay-rollup-v1", Some(10)) {
        return deterministic_replay_rollup_classifications(&fields).map(Some);
    }
    if let Some(fields) = value.collect_simple_record("deterministic-replay-index-v1", Some(15)) {
        return deterministic_replay_index_classifications(&fields).map(Some);
    }
    Ok(None)
}

fn release_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Ok(receipt) = crate::operator_dogfood::parse_release_gate_receipt(value) {
        let mut classifications = vec![
            "deterministic-replay:release-binding".to_string(),
            format!("release-dogfood-decision:{}", receipt.decision),
        ];
        for reference in receipt.replay_index_refs {
            classifications.push(format!("release-replay-index:{reference}"));
        }
        return Ok(Some(classifications));
    }
    if let Ok(evidence) = crate::operator_dogfood::parse_nix_dogfood_evidence(value) {
        return Ok(Some(vec![
            "deterministic-replay:release-binding".to_string(),
            format!("release-dogfood-replay-verify:{}", evidence.replay_verify_ref),
            format!("release-dogfood-replay-index:{}", evidence.replay_index_ref),
            format!("release-dogfood-release-gate:{}", evidence.release_gate_ref),
        ]));
    }
    if let Ok(receipt) = crate::operator_dogfood::parse_nix_dogfood_verify_receipt(value) {
        return Ok(Some(vec![
            "deterministic-replay:release-binding".to_string(),
            format!("release-dogfood-decision:{}", receipt.decision),
            format!("release-dogfood-replay-verify:{}", receipt.replay_verify_ref),
            format!("release-dogfood-replay-index:{}", receipt.replay_index_ref),
        ]));
    }
    if let Ok(bundle) = crate::operator_dogfood::parse_release_evidence_bundle(value) {
        return Ok(Some(vec![
            "deterministic-replay:release-binding".to_string(),
            format!("release-dogfood-replay-verify:{}", bundle.replay_verify_ref),
            format!("release-dogfood-replay-index:{}", bundle.replay_index_ref),
            format!("release-dogfood-release-gate:{}", bundle.release_gate_ref),
        ]));
    }
    if let Ok(receipt) = crate::operator_dogfood::parse_release_evidence_bundle_verify_receipt(value) {
        return Ok(Some(vec![
            "deterministic-replay:release-binding".to_string(),
            format!("release-dogfood-decision:{}", receipt.decision),
            format!("release-dogfood-replay-verify:{}", receipt.replay_verify_ref),
            format!("release-dogfood-replay-index:{}", receipt.replay_index_ref),
        ]));
    }
    Ok(None)
}

fn retention_core_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Ok(profile) = crate::retention::parse_class_profile(value) {
        return Ok(Some(vec![
            "retention:class".to_string(),
            format!("retention-class:{}", profile.class_name),
            format!("retention-policies:{}", profile.policy_refs.len()),
        ]));
    }
    if let Ok(pin) = crate::retention::parse_pin(value) {
        return Ok(Some(vec![
            "retention:pin".to_string(),
            format!("retention-object:{}", pin.object_ref),
            format!("retention-class:{}", pin.retention_class),
            format!("retention-source:{}", pin.source),
        ]));
    }
    if let Ok(index) = crate::retention::parse_reference_index(value) {
        return Ok(Some(vec![
            "retention:index".to_string(),
            format!("retention-object:{}", index.object_ref),
            format!("retention-pins:{}", index.pin_refs.len()),
            format!("retention-complete:{}", index.is_complete),
        ]));
    }
    Ok(None)
}
