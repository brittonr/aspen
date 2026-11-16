use std::collections::BTreeSet;

use super::*;

const OBSERVATION_IDENTITY_DOMAIN: &str = "onixresearch.molten.semantic-state-oracle-observation.v1";
const COMPARISON_IDENTITY_DOMAIN: &str = "onixresearch.molten.semantic-state-oracle-comparison.v1";
const CANONICAL_LENGTH_BYTES: usize = core::mem::size_of::<u128>();
const _: () = assert!(core::mem::size_of::<usize>() <= CANONICAL_LENGTH_BYTES);

// r[impl molten.world_state_oracle.observations]
pub fn build_oracle_observation(
    source: &OracleSourceDescriptor,
    mut input: OracleObservationInput,
) -> Result<OracleObservation, Vec<OracleIssue>> {
    let source_issues = validate_source_descriptor(source);
    if !source_issues.is_empty() {
        return Err(source_issues);
    }
    input.rows.sort();
    let mut observation = OracleObservation {
        schema: ORACLE_OBSERVATION_SCHEMA.to_string(),
        observation_ref: String::new(),
        source_revision: source.revision.clone(),
        build_ref: source.build_ref.clone(),
        adapter_ref: input.adapter_ref,
        case: input.case,
        branch: input.branch,
        rows: input.rows,
        outcome: input.outcome,
        backend_root: input.backend_root,
        backend_root_is_global_identity: false,
        diagnostics: input.diagnostics,
        non_claims: REQUIRED_ORACLE_NON_CLAIMS.to_vec(),
    };
    let issues = validate_observation(&observation, source.bounds, false);
    if !issues.is_empty() {
        return Err(issues);
    }
    observation.observation_ref = identify_observation(&observation);
    Ok(observation)
}

pub fn validate_observation(
    observation: &OracleObservation,
    bounds: OracleBounds,
    require_identity: bool,
) -> Vec<OracleIssue> {
    let mut issues = Vec::with_capacity(bounds.max_diagnostics);
    if observation.schema != ORACLE_OBSERVATION_SCHEMA {
        issues.push(OracleIssue::SchemaMismatch);
    }
    for reference in [&observation.build_ref, &observation.adapter_ref] {
        if !is_blake3_ref(reference) {
            issues.push(OracleIssue::MalformedReference(reference.clone()));
        }
    }
    if observation.rows.len() > bounds.max_rows {
        issues.push(OracleIssue::RowLimitExceeded);
    }
    let mut keys = BTreeSet::new();
    let mut prior = None;
    for row in &observation.rows {
        if !keys.insert(row.key.as_str()) {
            issues.push(OracleIssue::DuplicateSemanticKey(row.key.clone()));
        }
        if prior.is_some_and(|prior_key: &str| prior_key >= row.key.as_str()) {
            issues.push(OracleIssue::NonCanonicalRowOrder);
        }
        prior = Some(row.key.as_str());
        if row.key.len() > bounds.max_key_bytes {
            issues.push(OracleIssue::KeyLimitExceeded(row.key.clone()));
        }
        if row.value.len() > bounds.max_value_bytes {
            issues.push(OracleIssue::ValueLimitExceeded(row.key.clone()));
        }
    }
    if observation.diagnostics.len() > bounds.max_diagnostics {
        issues.push(OracleIssue::DiagnosticLimitExceeded);
    }
    if observation.backend_root_is_global_identity {
        issues.push(OracleIssue::BackendIdentityOverclaim);
    }
    for non_claim in REQUIRED_ORACLE_NON_CLAIMS {
        if !observation.non_claims.contains(&non_claim) {
            issues.push(OracleIssue::MissingNonClaim(non_claim));
        }
    }
    if require_identity && observation.observation_ref != identify_observation(observation) {
        issues.push(OracleIssue::ObservationIdentityMismatch);
    }
    issues.sort();
    issues.dedup();
    issues
}

pub fn identify_observation(observation: &OracleObservation) -> String {
    let mut bytes = Vec::new();
    push(&mut bytes, OBSERVATION_IDENTITY_DOMAIN);
    push(&mut bytes, &observation.schema);
    push(&mut bytes, &observation.source_revision);
    push(&mut bytes, &observation.build_ref);
    push(&mut bytes, &observation.adapter_ref);
    push(&mut bytes, observation.case.as_str());
    push_optional(&mut bytes, observation.branch.as_deref());
    push_length(&mut bytes, observation.rows.len());
    for row in &observation.rows {
        push(&mut bytes, &row.key);
        push(&mut bytes, &row.value);
    }
    push(&mut bytes, observation.outcome.as_str());
    push_optional(&mut bytes, observation.backend_root.as_deref());
    push_length(&mut bytes, observation.diagnostics.len());
    for diagnostic in &observation.diagnostics {
        push(&mut bytes, diagnostic);
    }
    for non_claim in &observation.non_claims {
        push(&mut bytes, non_claim.as_str());
    }
    content_ref(&bytes)
}

pub(crate) fn identify_comparison(comparison: &OracleComparison) -> String {
    let mut bytes = Vec::new();
    push(&mut bytes, COMPARISON_IDENTITY_DOMAIN);
    push(&mut bytes, &comparison.schema);
    push(&mut bytes, &comparison.expected_ref);
    push(&mut bytes, &comparison.observed_ref);
    push(&mut bytes, match comparison.decision {
        ComparisonDecision::Agreement => "agreement",
        ComparisonDecision::Divergence => "divergence",
        ComparisonDecision::Unsupported => "unsupported",
    });
    push_optional(&mut bytes, comparison.first_divergence.as_deref());
    for non_claim in &comparison.non_claims {
        push(&mut bytes, non_claim.as_str());
    }
    content_ref(&bytes)
}

pub(crate) fn content_ref(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn push(bytes: &mut Vec<u8>, value: &str) {
    push_length(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_optional(bytes: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            bytes.push(1);
            push(bytes, value);
        }
        None => bytes.push(0),
    }
}

pub(super) fn push_length(bytes: &mut Vec<u8>, value: usize) {
    let native = value.to_be_bytes();
    let padding_length = CANONICAL_LENGTH_BYTES - native.len();
    let padding = [0_u8; CANONICAL_LENGTH_BYTES];
    bytes.extend_from_slice(&padding[..padding_length]);
    bytes.extend_from_slice(&native);
}
