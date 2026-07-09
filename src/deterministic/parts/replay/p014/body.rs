const MULTITURN_EXPLAIN_SCHEMA: &str = "molten.determinism.multiturn-replay.explain.v1";
const MULTITURN_PREFIX_SCHEMA: &str = "molten.determinism.multiturn-replay.prefix.v1";
const MULTITURN_COMPARISON_FIELD_COUNT: usize = 7;
const MULTITURN_COMPARISON_DECISION_INDEX: usize = 1;
const MULTITURN_COMPARISON_DIVERGENCE_INDEX: usize = 4;
const MULTITURN_COMPARISON_REDACTION_INDEX: usize = 5;

#[derive(Clone, Debug)]
pub struct ReplayExplainReceipt {
    pub value: IoValue,
    pub receipt_ref: String,
    pub decision: String,
    pub first_divergence_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayPrefixManifest {
    pub manifest_ref: String,
    pub run_identity_ref: String,
    pub summary_root_ref: String,
    pub turn_chunk_refs: Vec<String>,
    pub effect_log_chunk_refs: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ReplayPrefixReceipt {
    pub value: IoValue,
    pub receipt_ref: String,
    pub decision: String,
    pub first_mismatch_ref: Option<String>,
}

pub fn explain_replay_comparison_value(value: IoValue) -> Result<ReplayExplainReceipt> {
    let comparison_ref = canonical_hash(&value)?;
    let fields = value
        .collect_simple_record("deterministic-replay-comparison-v1", Some(MULTITURN_COMPARISON_FIELD_COUNT))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected <deterministic-replay-comparison-v1 ...>"))?;
    require_schema_value(&fields[0], MULTITURN_REPLAY_COMPARISON_SCHEMA, "multiturn replay comparison")?;
    let decision = record_string_value(&fields[MULTITURN_COMPARISON_DECISION_INDEX], "decision")?;
    validate_replay_decision(&decision)?;
    let first_divergence_ref = record_string_value(&fields[MULTITURN_COMPARISON_DIVERGENCE_INDEX], "first-divergence-ref")?;
    validate_divergence_ref(&first_divergence_ref)?;
    let redaction_status = record_string_value(&fields[MULTITURN_COMPARISON_REDACTION_INDEX], "redaction-status")?;
    let first_divergence = (first_divergence_ref != "none").then_some(first_divergence_ref);
    let value = record("deterministic-replay-explain-v1", vec![
        string(MULTITURN_EXPLAIN_SCHEMA),
        record("decision", vec![string(&decision)]),
        record("comparison-ref", vec![string(&comparison_ref)]),
        record("first-divergence-ref", vec![string(first_divergence.as_deref().unwrap_or("none"))]),
        record("redaction-status", vec![string(redaction_status)]),
        sequence(vec![
            string("canonical-receipt-before-render"),
            string("refs-only-no-payloads"),
            string("evidence-only-no-authority"),
        ]),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(ReplayExplainReceipt {
        value,
        receipt_ref,
        decision,
        first_divergence_ref: first_divergence,
    })
}

pub fn compare_replay_prefix_manifests(
    expected: ReplayPrefixManifest,
    actual: ReplayPrefixManifest,
) -> Result<ReplayPrefixReceipt> {
    validate_prefix_manifest(&expected)?;
    validate_prefix_manifest(&actual)?;
    let first_mismatch = first_prefix_mismatch(&expected, &actual);
    let first_mismatch_ref = first_mismatch.as_ref().map(canonical_hash).transpose()?;
    let decision = if first_mismatch_ref.is_some() { "deny" } else { "pass" };
    let value = record("deterministic-replay-prefix-comparison-v1", vec![
        string(MULTITURN_PREFIX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("expected-manifest-ref", vec![string(expected.manifest_ref)]),
        record("actual-manifest-ref", vec![string(actual.manifest_ref)]),
        record("first-mismatch-ref", vec![string(first_mismatch_ref.as_deref().unwrap_or("none"))]),
        record("partial-fetch", vec![string("range-receipt-required")]),
        sequence(vec![string("manifest-backed-prefix"), string("evidence-only-no-authority")]),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(ReplayPrefixReceipt {
        value,
        receipt_ref,
        decision: decision.to_string(),
        first_mismatch_ref,
    })
}

#[derive(Clone, Copy)]
enum ReplayRefVectorKind {
    Turn,
    EffectLog,
    Output,
}

struct ReplayRefPair<'a> {
    expected_ref: &'a str,
    actual_ref: &'a str,
}

fn first_multiturn_divergence(expected: ReplayTraceSummary, actual: ReplayTraceSummary) -> Option<ReplayDivergencePath> {
    if expected.run_identity_ref != actual.run_identity_ref {
        return Some(summary_divergence(
            "run-identity",
            "identity.ref",
            &expected,
            &actual,
            ReplayRefPair {
                expected_ref: &expected.run_identity_ref,
                actual_ref: &actual.run_identity_ref,
            },
        ));
    }
    if let Some(divergence) = first_boundary_divergence(&expected, &actual) {
        return Some(divergence);
    }
    if let Some(divergence) = first_ref_vector_divergence(ReplayRefVectorKind::EffectLog, &expected, &actual) {
        return Some(divergence);
    }
    if let Some(divergence) = first_ref_vector_divergence(ReplayRefVectorKind::Output, &expected, &actual) {
        return Some(divergence);
    }
    if expected.final_state_ref != actual.final_state_ref {
        return Some(summary_divergence(
            "final-state",
            "final-state-ref",
            &expected,
            &actual,
            ReplayRefPair {
                expected_ref: &expected.final_state_ref,
                actual_ref: &actual.final_state_ref,
            },
        ));
    }
    first_ref_vector_divergence(ReplayRefVectorKind::Turn, &expected, &actual)
}

fn first_boundary_divergence(expected: &ReplayTraceSummary, actual: &ReplayTraceSummary) -> Option<ReplayDivergencePath> {
    let limit = expected.boundary_refs.len().max(actual.boundary_refs.len());
    for index in 0..limit {
        let expected_boundary = expected.boundary_refs.get(index);
        let actual_boundary = actual.boundary_refs.get(index);
        match (expected_boundary, actual_boundary) {
            (Some(expected_boundary), Some(actual_boundary)) if expected_boundary == actual_boundary => {}
            (Some(expected_boundary), Some(actual_boundary)) => {
                return Some(boundary_divergence(expected_boundary, actual_boundary, expected, actual));
            }
            (Some(expected_boundary), None) => {
                return Some(missing_boundary_divergence(expected_boundary, expected, actual, "missing-actual"));
            }
            (None, Some(actual_boundary)) => {
                return Some(missing_boundary_divergence(actual_boundary, expected, actual, "missing-expected"));
            }
            (None, None) => {}
        }
    }
    None
}

fn first_ref_vector_divergence(
    kind: ReplayRefVectorKind,
    expected: &ReplayTraceSummary,
    actual: &ReplayTraceSummary,
) -> Option<ReplayDivergencePath> {
    let (kind_label, path_prefix, expected_refs, actual_refs) = replay_ref_vector(kind, expected, actual);
    let limit = expected_refs.len().max(actual_refs.len());
    for index in 0..limit {
        let expected_ref = expected_refs.get(index).map(String::as_str).unwrap_or("none");
        let actual_ref = actual_refs.get(index).map(String::as_str).unwrap_or("none");
        if expected_ref != actual_ref {
            return Some(summary_divergence(
                kind_label,
                &format!("{path_prefix}[{index}]"),
                expected,
                actual,
                ReplayRefPair {
                    expected_ref,
                    actual_ref,
                },
            ));
        }
    }
    None
}

fn replay_ref_vector<'a>(
    kind: ReplayRefVectorKind,
    expected: &'a ReplayTraceSummary,
    actual: &'a ReplayTraceSummary,
) -> (&'static str, &'static str, &'a [String], &'a [String]) {
    match kind {
        ReplayRefVectorKind::Turn => ("turn", "turn-refs", &expected.turn_refs, &actual.turn_refs),
        ReplayRefVectorKind::EffectLog => (
            "effect-log",
            "effect-log-refs",
            &expected.effect_log_refs,
            &actual.effect_log_refs,
        ),
        ReplayRefVectorKind::Output => ("output", "output-refs", &expected.output_refs, &actual.output_refs),
    }
}

fn boundary_divergence(
    expected_boundary: &ReplayBoundaryRef,
    actual_boundary: &ReplayBoundaryRef,
    expected: &ReplayTraceSummary,
    actual: &ReplayTraceSummary,
) -> ReplayDivergencePath {
    ReplayDivergencePath {
        turn_index: expected_boundary.turn_index,
        event_index: expected_boundary.event_index,
        boundary_kind: expected_boundary.boundary_kind.clone(),
        actor_id: expected_boundary.actor_id.clone(),
        session_id: expected_boundary.session_id.clone(),
        vat_id: expected_boundary.vat_id.clone(),
        field_path: expected_boundary.field_path.clone(),
        expected_ref: expected_boundary.boundary_ref.clone(),
        actual_ref: actual_boundary.boundary_ref.clone(),
        handler_profile_ref: expected.handler_profile_ref.clone().min(actual.handler_profile_ref.clone()),
        redaction_status: "refs-only".to_string(),
    }
}

fn missing_boundary_divergence(
    boundary: &ReplayBoundaryRef,
    expected: &ReplayTraceSummary,
    actual: &ReplayTraceSummary,
    missing_side: &str,
) -> ReplayDivergencePath {
    let expected_ref = if missing_side == "missing-expected" { "none" } else { boundary.boundary_ref.as_str() };
    let actual_ref = if missing_side == "missing-actual" { "none" } else { boundary.boundary_ref.as_str() };
    summary_divergence(
        &boundary.boundary_kind,
        &boundary.field_path,
        expected,
        actual,
        ReplayRefPair {
            expected_ref,
            actual_ref,
        },
    )
}

fn summary_divergence(
    kind: &str,
    field_path: &str,
    expected: &ReplayTraceSummary,
    actual: &ReplayTraceSummary,
    refs: ReplayRefPair<'_>,
) -> ReplayDivergencePath {
    ReplayDivergencePath {
        turn_index: FIRST_REPLAY_TURN_INDEX,
        event_index: FIRST_REPLAY_TURN_INDEX,
        boundary_kind: kind.to_string(),
        actor_id: None,
        session_id: None,
        vat_id: None,
        field_path: field_path.to_string(),
        expected_ref: refs.expected_ref.to_string(),
        actual_ref: refs.actual_ref.to_string(),
        handler_profile_ref: expected.handler_profile_ref.clone().min(actual.handler_profile_ref.clone()),
        redaction_status: "refs-only".to_string(),
    }
}
