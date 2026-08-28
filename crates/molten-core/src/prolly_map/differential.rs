use super::*;
use crate::world_benchmark::*;
use crate::world_state_oracle::*;

const REQUIRED_CONSUMER_COUNT: u32 = 2;
const REQUIRED_RECEIPTS_PER_CONSUMER: u32 = 1;

// r[impl molten.prolly_map.differential]
pub fn compare_with_doltlite(
    profile: &ProllyProfile,
    snapshot: &MapSnapshot,
    oracle: &OracleObservation,
) -> Result<ProllyDifferentialEvidence, Vec<ProllyIssue>> {
    let oracle_issues = validate_observation(oracle, OracleBounds::standard(), true);
    if !oracle_issues.is_empty() {
        return Err(vec![ProllyIssue::IdentityFailure(format!(
            "oracle observation denied: {oracle_issues:?}"
        ))]);
    }
    let read = validate_snapshot(profile, snapshot)?;
    let rows = read.entries.iter().map(entry_as_row).collect::<Result<Vec<_>, _>>()?;
    let is_unsupported = oracle.outcome == OracleOutcome::Unsupported;
    let first_divergence = if rows != oracle.rows {
        first_row_divergence(&rows, &oracle.rows)
    } else if !matches!(oracle.outcome, OracleOutcome::Applied | OracleOutcome::EqualState) {
        Some("outcome".to_string())
    } else {
        None
    };
    let decision = if is_unsupported {
        DifferentialDecision::Unsupported
    } else if first_divergence.is_some() {
        DifferentialDecision::Divergence
    } else {
        DifferentialDecision::Agreement
    };
    Ok(ProllyDifferentialEvidence {
        map_root_ref: snapshot.root.root_ref.clone(),
        oracle_observation_ref: oracle.observation_ref.clone(),
        decision,
        first_divergence,
        cross_format_root_equality_required: false,
        proves_correctness: false,
    })
}

// r[impl molten.prolly_map.extraction]
pub fn classify_local_extraction(
    receipt: WorldBenchmarkReceipt,
) -> Result<WorldBenchmarkExtractionDecision, Vec<WorldBenchmarkIssue>> {
    classify_world_benchmark_extraction(
        &[WorldBenchmarkExtractionEvidence {
            receipt,
            owned_adapter: true,
            product_neutral_limit_failed: false,
        }],
        &WorldBenchmarkExtractionPolicy {
            minimum_accepted_receipts_per_consumer: REQUIRED_RECEIPTS_PER_CONSUMER,
            minimum_credible_consumers: REQUIRED_CONSUMER_COUNT,
            require_product_neutral_limit: true,
        },
    )
}

fn entry_as_row(entry: &SemanticEntry) -> Result<SemanticStateRow, Vec<ProllyIssue>> {
    let key = String::from_utf8(entry.key.clone()).map_err(|_| vec![ProllyIssue::DifferentialInputNotText])?;
    let value = String::from_utf8(entry.value.clone()).map_err(|_| vec![ProllyIssue::DifferentialInputNotText])?;
    Ok(SemanticStateRow { key, value })
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
