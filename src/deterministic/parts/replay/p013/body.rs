const MULTITURN_REPLAY_SUMMARY_SCHEMA: &str = "molten.determinism.multiturn-replay.summary.v1";
const MULTITURN_REPLAY_COMPARISON_SCHEMA: &str = "molten.determinism.multiturn-replay.comparison.v1";
const MULTITURN_REPLAY_DIVERGENCE_SCHEMA: &str = "molten.determinism.multiturn-replay.first-divergence.v1";
const MAX_MULTITURN_REPLAY_ITEMS: usize = 1024;
const FIRST_REPLAY_TURN_INDEX: u64 = 0;
const SCHEDULER_EVENT_INDEX: u64 = 0;
const INPUT_EVENT_INDEX: u64 = 1;
const EFFECT_REQUEST_EVENT_INDEX: u64 = 2;
const EFFECT_RESPONSE_EVENT_INDEX: u64 = 3;
const POLICY_EVENT_INDEX: u64 = 4;
const ACTION_EVENT_INDEX: u64 = 5;
const RECEIPT_EVENT_INDEX: u64 = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayBoundaryRef {
    pub turn_index: u64,
    pub event_index: u64,
    pub boundary_kind: String,
    pub actor_id: Option<String>,
    pub session_id: Option<String>,
    pub vat_id: Option<String>,
    pub field_path: String,
    pub boundary_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayTraceSummary {
    pub run_identity_ref: String,
    pub handler_profile_ref: String,
    pub turn_refs: Vec<String>,
    pub boundary_refs: Vec<ReplayBoundaryRef>,
    pub effect_log_refs: Vec<String>,
    pub output_refs: Vec<String>,
    pub final_state_ref: String,
}

#[derive(Clone, Debug)]
pub struct ReplayComparisonReceipt {
    pub value: IoValue,
    pub receipt_ref: String,
    pub decision: String,
    pub first_divergence_ref: Option<String>,
    pub first_divergence: Option<IoValue>,
}

#[derive(Clone, Debug)]
struct ReplayDivergencePath {
    turn_index: u64,
    event_index: u64,
    boundary_kind: String,
    actor_id: Option<String>,
    session_id: Option<String>,
    vat_id: Option<String>,
    field_path: String,
    expected_ref: String,
    actual_ref: String,
    handler_profile_ref: String,
    redaction_status: String,
}

pub fn replay_summary_from_fixture_record_value(value: IoValue) -> Result<ReplayTraceSummary> {
    let parts = parse_fixture_record_run_parts(&value)?;
    Ok(replay_summary_from_parts(parts))
}

pub fn compare_replay_fixture_values(expected: IoValue, actual: IoValue) -> Result<ReplayComparisonReceipt> {
    let expected = replay_summary_from_fixture_record_value(expected)?;
    let actual = replay_summary_from_fixture_record_value(actual)?;
    compare_replay_summaries(expected, actual)
}

pub fn compare_replay_summaries(
    expected: ReplayTraceSummary,
    actual: ReplayTraceSummary,
) -> Result<ReplayComparisonReceipt> {
    validate_replay_summary(&expected)?;
    validate_replay_summary(&actual)?;
    let expected_value = replay_summary_value(expected.clone());
    let actual_value = replay_summary_value(actual.clone());
    let expected_summary_ref = canonical_hash(&expected_value)?;
    let actual_summary_ref = canonical_hash(&actual_value)?;
    let first_divergence = first_multiturn_divergence(expected, actual);
    let first_divergence_value = first_divergence.map(first_divergence_path_value).transpose()?;
    let first_divergence_ref = first_divergence_value.as_ref().map(canonical_hash).transpose()?;
    let decision = if first_divergence_ref.is_some() { "deny" } else { "pass" };
    let value = record("deterministic-replay-comparison-v1", vec![
        string(MULTITURN_REPLAY_COMPARISON_SCHEMA),
        record("decision", vec![string(decision)]),
        record("expected-summary-ref", vec![string(&expected_summary_ref)]),
        record("actual-summary-ref", vec![string(&actual_summary_ref)]),
        record("first-divergence-ref", vec![string(first_divergence_ref.as_deref().unwrap_or("none"))]),
        record("redaction-status", vec![string("refs-only")]),
        sequence(compare_checks(decision)),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(ReplayComparisonReceipt {
        value,
        receipt_ref,
        decision: decision.to_string(),
        first_divergence_ref,
        first_divergence: first_divergence_value,
    })
}

fn replay_summary_from_parts(parts: ReplayRunParts) -> ReplayTraceSummary {
    let boundary_refs = fixture_boundaries(&parts);
    ReplayTraceSummary {
        run_identity_ref: parts.identity_ref,
        handler_profile_ref: DEFAULT_HANDLER_PROFILE_REF.to_string(),
        turn_refs: vec![parts.turn_journal_ref],
        boundary_refs,
        effect_log_refs: vec![parts.effect_log_ref],
        output_refs: vec![parts.output_ref],
        final_state_ref: parts.after_state_ref,
    }
}

fn fixture_boundaries(parts: &ReplayRunParts) -> Vec<ReplayBoundaryRef> {
    vec![
        fixture_boundary("scheduler", SCHEDULER_EVENT_INDEX, "turns[0].scheduler-key-ref", &parts.scheduler_ref),
        fixture_boundary("input", INPUT_EVENT_INDEX, "turns[0].input-ref", &parts.input_ref),
        fixture_boundary(
            "effect-request",
            EFFECT_REQUEST_EVENT_INDEX,
            "turns[0].effect-request-ref",
            &parts.effect_request_ref,
        ),
        fixture_boundary(
            "effect-response",
            EFFECT_RESPONSE_EVENT_INDEX,
            "turns[0].effect-response-ref",
            &parts.effect_response_ref,
        ),
        fixture_boundary("policy-decision", POLICY_EVENT_INDEX, "turns[0].policy-decision-ref", &parts.policy_decision_ref),
        fixture_boundary("action", ACTION_EVENT_INDEX, "turns[0].action-ref", &parts.action_ref),
        fixture_boundary("receipt", RECEIPT_EVENT_INDEX, "turns[0].receipt-ref", &parts.receipt_ref),
    ]
}

fn fixture_boundary(kind: &str, event_index: u64, field_path: &str, reference: &str) -> ReplayBoundaryRef {
    ReplayBoundaryRef {
        turn_index: FIRST_REPLAY_TURN_INDEX,
        event_index,
        boundary_kind: kind.to_string(),
        actor_id: Some("actor:helper".to_string()),
        session_id: None,
        vat_id: None,
        field_path: field_path.to_string(),
        boundary_ref: reference.to_string(),
    }
}

fn validate_replay_summary(summary: &ReplayTraceSummary) -> Result<()> {
    validate_content_ref(&summary.run_identity_ref)?;
    validate_content_ref(&summary.handler_profile_ref)?;
    validate_ref_list_bound(&summary.turn_refs, "turn refs")?;
    validate_boundary_refs(&summary.boundary_refs)?;
    validate_ref_list_bound(&summary.effect_log_refs, "effect log refs")?;
    validate_ref_list_bound(&summary.output_refs, "output refs")?;
    validate_content_ref(&summary.final_state_ref)
}

fn validate_ref_list_bound(refs: &[String], label: &'static str) -> Result<()> {
    if refs.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(format!("{label} must not be empty")));
    }
    if refs.len() > MAX_MULTITURN_REPLAY_ITEMS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "{label} count exceeds {MAX_MULTITURN_REPLAY_ITEMS}"
        )));
    }
    for reference in refs {
        validate_content_ref(reference)?;
    }
    Ok(())
}

fn validate_boundary_refs(boundaries: &[ReplayBoundaryRef]) -> Result<()> {
    if boundaries.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("boundary refs must not be empty"));
    }
    if boundaries.len() > MAX_MULTITURN_REPLAY_ITEMS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "boundary refs count exceeds {MAX_MULTITURN_REPLAY_ITEMS}"
        )));
    }
    for boundary in boundaries {
        validate_content_ref(&boundary.boundary_ref)?;
        validate_replay_profile(&boundary.boundary_kind)?;
        if boundary.field_path.is_empty() {
            return Err(crate::error::MoltenError::invalid_harness("boundary field path cannot be empty"));
        }
    }
    Ok(())
}

fn replay_summary_value(summary: ReplayTraceSummary) -> IoValue {
    record("multiturn-replay-summary-v1", vec![
        string(MULTITURN_REPLAY_SUMMARY_SCHEMA),
        record("run-identity-ref", vec![string(summary.run_identity_ref)]),
        record("handler-profile-ref", vec![string(summary.handler_profile_ref)]),
        record("turn-refs", vec![refs_sequence(summary.turn_refs)]),
        record("boundary-refs", vec![sequence(summary.boundary_refs.into_iter().map(boundary_value).collect())]),
        record("effect-log-refs", vec![refs_sequence(summary.effect_log_refs)]),
        record("output-refs", vec![refs_sequence(summary.output_refs)]),
        record("final-state-ref", vec![string(summary.final_state_ref)]),
    ])
}

fn boundary_value(boundary: ReplayBoundaryRef) -> IoValue {
    record("replay-boundary-ref", vec![
        record("turn-index", vec![u64_value(boundary.turn_index)]),
        record("event-index", vec![u64_value(boundary.event_index)]),
        record("boundary-kind", vec![string(boundary.boundary_kind)]),
        record("actor-id", vec![optional_text(boundary.actor_id)]),
        record("session-id", vec![optional_text(boundary.session_id)]),
        record("vat-id", vec![optional_text(boundary.vat_id)]),
        record("field-path", vec![string(boundary.field_path)]),
        record("boundary-ref", vec![string(boundary.boundary_ref)]),
    ])
}

fn refs_sequence(refs: Vec<String>) -> IoValue {
    sequence(refs.into_iter().map(string).collect())
}
