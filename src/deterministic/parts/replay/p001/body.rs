
#[derive(Clone)]
struct ReplayRunParts {
    identity: IoValue,
    identity_ref: String,
    scheduler_ref: String,
    input_ref: String,
    effect_request_ref: String,
    effect_response_ref: String,
    policy_decision_ref: String,
    action_ref: String,
    receipt_ref: String,
    output_ref: String,
    after_state_ref: String,
    turn_journal: IoValue,
    turn_journal_ref: String,
    effect_log: IoValue,
    effect_log_ref: String,
}

pub fn record_fixture_value() -> Result<ReplayFixtureRecord> {
    replay_fixture_record_value(ReplayFixtureVariant::Baseline)
}

pub fn tampered_fixture_record_value(variant: ReplayFixtureVariant) -> Result<ReplayFixtureRecord> {
    if variant == ReplayFixtureVariant::Baseline {
        return Err(crate::error::MoltenError::invalid_harness(
            "replay fixture tamper variant must differ from baseline",
        ));
    }
    replay_fixture_record_value(variant)
}

fn replay_fixture_record_value(variant: ReplayFixtureVariant) -> Result<ReplayFixtureRecord> {
    let parts = run_parts(variant)?;
    fixture_record_from_parts(&parts)
}

fn fixture_record_from_parts(parts: &ReplayRunParts) -> Result<ReplayFixtureRecord> {
    let value = record("deterministic-fixture-record-v1", vec![
        string(DETERMINISTIC_FIXTURE_RECORD_SCHEMA),
        record("identity-ref", vec![string(&parts.identity_ref)]),
        parts.identity.clone(),
        record("effect-log-ref", vec![string(&parts.effect_log_ref)]),
        parts.effect_log.clone(),
        sequence(vec![parts.turn_journal.clone()]),
        record("output-ref", vec![string(&parts.output_ref)]),
        record("final-state-ref", vec![string(&parts.after_state_ref)]),
        sequence(vec![
            string("recorded-responses-bound"),
            string("canonical-journal-order"),
            string("no-ambient-observations"),
        ]),
    ]);
    let record_ref = canonical_hash(&value)?;
    Ok(ReplayFixtureRecord {
        value,
        record_ref,
        identity_ref: parts.identity_ref.clone(),
        effect_log_ref: parts.effect_log_ref.clone(),
        final_state_ref: parts.after_state_ref.clone(),
        output_ref: parts.output_ref.clone(),
    })
}

pub fn replay_snapshot_manifest_bundle(
    chunk_root: &Path,
    variant: ReplayFixtureVariant,
) -> Result<ReplaySnapshotManifestBundle> {
    let expected = run_parts(ReplayFixtureVariant::Baseline)?;
    let actual = run_parts(variant)?;
    let snapshot = record("deterministic-replay-snapshot-v1", vec![
        string("molten.deterministic-replay.snapshot.v1"),
        record("identity-ref", vec![string(&expected.identity_ref)]),
        record("final-state-ref", vec![string(&expected.after_state_ref)]),
        record("turn-journal-ref", vec![string(&expected.turn_journal_ref)]),
        record("effect-log-ref", vec![string(&expected.effect_log_ref)]),
        sequence(vec![string("manifest-backed"), string("partial-debug-fetch")]),
    ]);
    let effect_log_manifest_ref = store_replay_manifest(chunk_root, "replay-effect-log", &expected.effect_log)?;
    let turn_journal_manifest_ref = store_replay_manifest(chunk_root, "replay-turn-journal", &expected.turn_journal)?;
    let snapshot_manifest_ref = store_replay_manifest(chunk_root, "replay-snapshot", &snapshot)?;
    let divergence = first_divergence(&expected, &actual, variant);
    let (first_divergence_manifest_ref, debug_range_receipt_ref) = if divergence == ReplayDivergenceKind::None {
        (None, None)
    } else {
        let divergence_value = first_divergence_value(divergence, &expected, &actual)?;
        let manifest_ref = store_replay_manifest(chunk_root, "replay-first-divergence", &divergence_value)?;
        let range = range_read(chunk_root, &manifest_ref, 0, 32)?;
        (Some(manifest_ref), Some(canonical_hash(&range.receipt_value)?))
    };
    let value = record("deterministic-replay-snapshot-manifests-v1", vec![
        string("molten.deterministic-replay.snapshot-manifests.v1"),
        record("effect-log-manifest-ref", vec![string(&effect_log_manifest_ref)]),
        record("turn-journal-manifest-ref", vec![string(&turn_journal_manifest_ref)]),
        record("snapshot-manifest-ref", vec![string(&snapshot_manifest_ref)]),
        record("first-divergence-manifest-ref", vec![optional_ref_value(first_divergence_manifest_ref.as_deref())]),
        record("debug-range-receipt-ref", vec![optional_ref_value(debug_range_receipt_ref.as_deref())]),
        sequence(vec![
            string("manifest-backed-replay"),
            string("verified-before-load"),
            string("partial-divergence-debug-fetch"),
        ]),
    ]);
    let bundle_ref = canonical_hash(&value)?;
    Ok(ReplaySnapshotManifestBundle {
        value,
        bundle_ref,
        effect_log_manifest_ref,
        turn_journal_manifest_ref,
        snapshot_manifest_ref,
        first_divergence_manifest_ref,
        debug_range_receipt_ref,
    })
}

fn store_replay_manifest(chunk_root: &Path, object_kind: &str, value: &IoValue) -> Result<String> {
    let bytes = canonical_bytes(value)?;
    Ok(put_bytes(chunk_root, object_kind, &bytes, DEFAULT_FIXED_V1_CHUNK_SIZE)?.manifest_ref)
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

pub fn verify_fixture_value(variant: ReplayFixtureVariant) -> Result<ReplayVerifyReceipt> {
    let expected = run_parts(ReplayFixtureVariant::Baseline)?;
    let actual = run_parts(variant)?;
    validate_replay_run_effects(actual.clone())?;
    let divergence = first_divergence(&expected, &actual, variant);
    replay_verify_receipt(&expected, &actual, divergence)
}

pub fn verify_fixture_record_value(value: &IoValue) -> Result<ReplayVerifyReceipt> {
    let expected = run_parts(ReplayFixtureVariant::Baseline)?;
    let actual = parse_fixture_record_run_parts(value)?;
    validate_replay_run_effects(actual.clone())?;
    let divergence = first_divergence_between_parts(&expected, &actual);
    replay_verify_receipt(&expected, &actual, divergence)
}

fn replay_verify_receipt(
    expected: &ReplayRunParts,
    actual: &ReplayRunParts,
    divergence: ReplayDivergenceKind,
) -> Result<ReplayVerifyReceipt> {
    let first_divergence = if divergence == ReplayDivergenceKind::None {
        None
    } else {
        Some(first_divergence_value(divergence, expected, actual)?)
    };
    let first_divergence_ref = match &first_divergence {
        Some(value) => canonical_hash(value)?,
        None => "none".to_string(),
    };
    let decision = if divergence == ReplayDivergenceKind::None { "pass" } else { "deny" };
    let value = record("deterministic-replay-verify-v1", vec![
        string(DETERMINISTIC_REPLAY_VERIFY_SCHEMA),
        string(decision),
        record("expected-identity-ref", vec![string(&expected.identity_ref)]),
        record("actual-identity-ref", vec![string(&actual.identity_ref)]),
        record("expected-effect-log-ref", vec![string(&expected.effect_log_ref)]),
        record("actual-effect-log-ref", vec![string(&actual.effect_log_ref)]),
        record("expected-output-ref", vec![string(&expected.output_ref)]),
        record("actual-output-ref", vec![string(&actual.output_ref)]),
        record("expected-final-state-ref", vec![string(&expected.after_state_ref)]),
        record("actual-final-state-ref", vec![string(&actual.after_state_ref)]),
        record("divergence", vec![string(divergence.as_str())]),
        record("first-divergence-ref", vec![string(&first_divergence_ref)]),
        sequence(verify_checks(decision, divergence)),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(ReplayVerifyReceipt {
        value,
        receipt_ref,
        decision,
        divergence,
        first_divergence,
    })
}
