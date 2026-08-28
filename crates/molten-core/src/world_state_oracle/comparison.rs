use super::*;

// r[impl molten.world_state_oracle.observations]
pub fn compare_oracle_observations(
    expected: &OracleObservation,
    observed: &OracleObservation,
) -> Result<OracleComparison, Vec<OracleIssue>> {
    let mut issues = validate_observation(expected, OracleBounds::standard(), true);
    issues.extend(validate_observation(observed, OracleBounds::standard(), true));
    if expected.case != observed.case {
        issues.push(OracleIssue::ComparisonCaseMismatch);
    }
    issues.sort();
    issues.dedup();
    if !issues.is_empty() {
        return Err(issues);
    }

    let is_unsupported = matches!(expected.outcome, OracleOutcome::Unsupported)
        || matches!(observed.outcome, OracleOutcome::Unsupported);
    let first_divergence = if expected.branch != observed.branch {
        Some("branch".to_string())
    } else if expected.rows != observed.rows {
        first_row_divergence(&expected.rows, &observed.rows)
    } else if expected.outcome != observed.outcome {
        Some("outcome".to_string())
    } else {
        None
    };
    let decision = if is_unsupported {
        ComparisonDecision::Unsupported
    } else if first_divergence.is_some() {
        ComparisonDecision::Divergence
    } else {
        ComparisonDecision::Agreement
    };
    let mut comparison = OracleComparison {
        schema: ORACLE_COMPARISON_SCHEMA.to_string(),
        comparison_ref: String::new(),
        expected_ref: expected.observation_ref.clone(),
        observed_ref: observed.observation_ref.clone(),
        decision,
        first_divergence,
        backend_roots_compared_as_global: false,
        non_claims: REQUIRED_ORACLE_NON_CLAIMS.to_vec(),
    };
    comparison.comparison_ref = identify_comparison(&comparison);
    Ok(comparison)
}

pub fn validate_oracle_comparison(comparison: &OracleComparison) -> Vec<OracleIssue> {
    let mut issues = Vec::new();
    if comparison.schema != ORACLE_COMPARISON_SCHEMA {
        issues.push(OracleIssue::SchemaMismatch);
    }
    for reference in [&comparison.expected_ref, &comparison.observed_ref] {
        if !is_blake3_ref(reference) {
            issues.push(OracleIssue::MalformedReference(reference.clone()));
        }
    }
    if comparison.backend_roots_compared_as_global || comparison.non_claims != REQUIRED_ORACLE_NON_CLAIMS {
        issues.push(OracleIssue::ComparisonOverclaim);
    }
    if comparison.comparison_ref != identify_comparison(comparison) {
        issues.push(OracleIssue::ComparisonIdentityMismatch);
    }
    issues.sort();
    issues.dedup();
    issues
}

fn first_row_divergence(expected: &[SemanticStateRow], observed: &[SemanticStateRow]) -> Option<String> {
    let common = expected.len().min(observed.len());
    for index in 0..common {
        if expected[index] != observed[index] {
            return Some(format!("rows[{index}]"));
        }
    }
    Some("row-count".to_string())
}
