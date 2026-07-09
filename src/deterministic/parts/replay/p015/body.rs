fn first_divergence_path_value(divergence: ReplayDivergencePath) -> Result<IoValue> {
    validate_divergence_ref(&divergence.expected_ref)?;
    validate_divergence_ref(&divergence.actual_ref)?;
    validate_content_ref(&divergence.handler_profile_ref)?;
    Ok(record("deterministic-first-divergence-path-v1", vec![
        string(MULTITURN_REPLAY_DIVERGENCE_SCHEMA),
        record("turn-index", vec![u64_value(divergence.turn_index)]),
        record("event-index", vec![u64_value(divergence.event_index)]),
        record("boundary-kind", vec![string(divergence.boundary_kind)]),
        record("actor-id", vec![optional_text(divergence.actor_id)]),
        record("session-id", vec![optional_text(divergence.session_id)]),
        record("vat-id", vec![optional_text(divergence.vat_id)]),
        record("field-path", vec![string(divergence.field_path)]),
        record("expected-ref", vec![string(divergence.expected_ref)]),
        record("actual-ref", vec![string(divergence.actual_ref)]),
        record("handler-profile-ref", vec![string(divergence.handler_profile_ref)]),
        record("redaction-status", vec![string(divergence.redaction_status)]),
        sequence(vec![string("refs-only-no-payloads"), string("evidence-only-no-authority")]),
    ]))
}

fn compare_checks(decision: &str) -> Vec<IoValue> {
    vec![
        record("check", vec![string("ordered-turns-compared"), string(decision)]),
        record("check", vec![string("first-semantic-divergence"), string(decision)]),
        record("check", vec![string("refs-only-redaction"), string("pass")]),
        record("check", vec![string("evidence-only-no-authority"), string("pass")]),
    ]
}

fn optional_text(value: Option<String>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn validate_prefix_manifest(manifest: &ReplayPrefixManifest) -> Result<()> {
    validate_content_ref(&manifest.manifest_ref)?;
    validate_content_ref(&manifest.run_identity_ref)?;
    validate_content_ref(&manifest.summary_root_ref)?;
    validate_ref_list_bound(&manifest.turn_chunk_refs, "turn chunk refs")?;
    validate_ref_list_bound(&manifest.effect_log_chunk_refs, "effect log chunk refs")
}

fn first_prefix_mismatch(expected: &ReplayPrefixManifest, actual: &ReplayPrefixManifest) -> Option<IoValue> {
    if expected.run_identity_ref != actual.run_identity_ref {
        return Some(prefix_mismatch_value("run-identity", &expected.run_identity_ref, &actual.run_identity_ref));
    }
    if expected.summary_root_ref != actual.summary_root_ref {
        return Some(prefix_mismatch_value("summary-root", &expected.summary_root_ref, &actual.summary_root_ref));
    }
    if let Some(value) = first_prefix_vector_mismatch("turn-chunk", &expected.turn_chunk_refs, &actual.turn_chunk_refs) {
        return Some(value);
    }
    first_prefix_vector_mismatch("effect-log-chunk", &expected.effect_log_chunk_refs, &actual.effect_log_chunk_refs)
}

fn first_prefix_vector_mismatch(kind: &str, expected_refs: &[String], actual_refs: &[String]) -> Option<IoValue> {
    let limit = expected_refs.len().max(actual_refs.len());
    for index in 0..limit {
        let expected_ref = expected_refs.get(index).map(String::as_str).unwrap_or("none");
        let actual_ref = actual_refs.get(index).map(String::as_str).unwrap_or("none");
        if expected_ref != actual_ref {
            return Some(prefix_mismatch_value(&format!("{kind}[{index}]"), expected_ref, actual_ref));
        }
    }
    None
}

fn prefix_mismatch_value(kind: &str, expected_ref: &str, actual_ref: &str) -> IoValue {
    record("deterministic-replay-prefix-mismatch-v1", vec![
        record("kind", vec![string(kind)]),
        record("expected-ref", vec![string(expected_ref)]),
        record("actual-ref", vec![string(actual_ref)]),
        record("partial-fetch", vec![string("range-receipt-required")]),
    ])
}
