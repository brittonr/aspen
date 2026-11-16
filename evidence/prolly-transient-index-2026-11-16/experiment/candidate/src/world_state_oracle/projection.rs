use super::*;

const PROJECTION_IDENTITY_DOMAIN: &str = "onixresearch.molten.semantic-state-oracle-projection.v1";

// r[impl molten.world_state_oracle.observations]
pub fn project_oracle_evidence(
    consumer: OracleConsumer,
    observation: &OracleObservation,
    comparison: &OracleComparison,
) -> Result<OracleEvidenceProjection, Vec<OracleIssue>> {
    let mut issues = validate_observation(observation, OracleBounds::standard(), true);
    if comparison.schema != ORACLE_COMPARISON_SCHEMA
        || comparison.comparison_ref != identify_comparison(comparison)
        || comparison.observed_ref != observation.observation_ref
    {
        issues.push(OracleIssue::ProjectionComparisonMismatch);
    }
    if comparison.backend_roots_compared_as_global || comparison.non_claims != REQUIRED_ORACLE_NON_CLAIMS {
        issues.push(OracleIssue::ProjectionOverclaim);
    }
    issues.sort();
    issues.dedup();
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut projection = OracleEvidenceProjection {
        schema: ORACLE_PROJECTION_SCHEMA.to_string(),
        projection_ref: String::new(),
        consumer,
        source_revision: observation.source_revision.clone(),
        build_ref: observation.build_ref.clone(),
        adapter_ref: observation.adapter_ref.clone(),
        case: observation.case,
        observation_ref: observation.observation_ref.clone(),
        comparison_ref: comparison.comparison_ref.clone(),
        decision: comparison.decision,
        branch: observation.branch.clone(),
        rows: observation.rows.clone(),
        outcome: observation.outcome,
        backend_root_included: false,
        authority_granted: false,
        correctness_proven: false,
        non_claims: REQUIRED_ORACLE_NON_CLAIMS.to_vec(),
    };
    projection.projection_ref = identify_projection(&projection);
    Ok(projection)
}

pub fn validate_oracle_projection(projection: &OracleEvidenceProjection) -> Vec<OracleIssue> {
    let mut issues = Vec::new();
    if projection.schema != ORACLE_PROJECTION_SCHEMA {
        issues.push(OracleIssue::SchemaMismatch);
    }
    for reference in [
        &projection.build_ref,
        &projection.adapter_ref,
        &projection.observation_ref,
        &projection.comparison_ref,
    ] {
        if !is_blake3_ref(reference) {
            issues.push(OracleIssue::MalformedReference(reference.clone()));
        }
    }
    if projection.backend_root_included
        || projection.authority_granted
        || projection.correctness_proven
        || projection.non_claims != REQUIRED_ORACLE_NON_CLAIMS
    {
        issues.push(OracleIssue::ProjectionOverclaim);
    }
    if projection.projection_ref != identify_projection(projection) {
        issues.push(OracleIssue::ProjectionIdentityMismatch);
    }
    issues.sort();
    issues.dedup();
    issues
}

pub fn identify_projection(projection: &OracleEvidenceProjection) -> String {
    let mut bytes = Vec::new();
    push(&mut bytes, PROJECTION_IDENTITY_DOMAIN);
    push(&mut bytes, &projection.schema);
    push(&mut bytes, projection.consumer.as_str());
    push(&mut bytes, &projection.source_revision);
    push(&mut bytes, &projection.build_ref);
    push(&mut bytes, &projection.adapter_ref);
    push(&mut bytes, projection.case.as_str());
    push(&mut bytes, &projection.observation_ref);
    push(&mut bytes, &projection.comparison_ref);
    push(&mut bytes, decision_name(projection.decision));
    push_optional(&mut bytes, projection.branch.as_deref());
    super::identity::push_length(&mut bytes, projection.rows.len());
    for row in &projection.rows {
        push(&mut bytes, &row.key);
        push(&mut bytes, &row.value);
    }
    push(&mut bytes, projection.outcome.as_str());
    for non_claim in &projection.non_claims {
        push(&mut bytes, non_claim.as_str());
    }
    content_ref(&bytes)
}

fn decision_name(decision: ComparisonDecision) -> &'static str {
    match decision {
        ComparisonDecision::Agreement => "agreement",
        ComparisonDecision::Divergence => "divergence",
        ComparisonDecision::Unsupported => "unsupported",
    }
}

fn push(bytes: &mut Vec<u8>, value: &str) {
    super::identity::push_length(bytes, value.len());
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
