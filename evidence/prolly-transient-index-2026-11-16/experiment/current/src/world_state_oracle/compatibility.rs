use std::collections::BTreeSet;

use super::*;

mod rows;

pub use rows::standard_compatibility_rows;

pub const STANDARD_COMPATIBILITY_LIMITS: CompatibilityLimits = CompatibilityLimits {
    max_adapted: 2,
    max_intentional: 7,
    max_unsupported: 1,
    max_engine_gap: 0,
};

// r[impl molten.world_state_oracle.compatibility]
pub fn validate_compatibility_rows(
    rows: &[CompatibilityRow],
    limits: CompatibilityLimits,
) -> Result<CompatibilitySummary, Vec<OracleIssue>> {
    let mut issues = Vec::with_capacity(MAX_ORACLE_DIAGNOSTICS);
    let mut ids = BTreeSet::new();
    let mut summary = CompatibilitySummary {
        compatible: 0,
        adapted: 0,
        intentional: 0,
        unsupported: 0,
        engine_gap: 0,
    };
    let mut prior = None;
    for row in rows {
        validate_row(row, prior, &mut ids, &mut issues);
        prior = Some(row.id.as_str());
        increment_status(&mut summary, row.status);
    }
    validate_limits(&summary, limits, &mut issues);
    issues.sort();
    issues.dedup();
    if issues.is_empty() { Ok(summary) } else { Err(issues) }
}

fn validate_row<'a>(
    row: &'a CompatibilityRow,
    prior: Option<&str>,
    ids: &mut BTreeSet<&'a str>,
    issues: &mut Vec<OracleIssue>,
) {
    if !ids.insert(row.id.as_str()) {
        issues.push(OracleIssue::DuplicateCompatibilityRow(row.id.clone()));
    }
    if prior.is_some_and(|prior_id| prior_id >= row.id.as_str()) {
        issues.push(OracleIssue::NonCanonicalRowOrder);
    }
    if !is_blake3_ref(&row.evidence_ref) {
        issues.push(OracleIssue::CompatibilityEvidenceMissing(row.id.clone()));
    }
    if row.fixture.is_empty() {
        issues.push(OracleIssue::CompatibilityFixtureMissing(row.id.clone()));
    }
    if row.explanation.is_empty() {
        issues.push(OracleIssue::CompatibilityExplanationMissing(row.id.clone()));
    }
    if row.status.requires_issue() && row.issue.as_deref().is_none_or(str::is_empty) {
        issues.push(OracleIssue::CompatibilityIssueMissing(row.id.clone()));
    }
}

fn validate_limits(summary: &CompatibilitySummary, limits: CompatibilityLimits, issues: &mut Vec<OracleIssue>) {
    for (status, actual, maximum) in [
        (CompatibilityStatus::Adapted, summary.adapted, limits.max_adapted),
        (CompatibilityStatus::Intentional, summary.intentional, limits.max_intentional),
        (CompatibilityStatus::Unsupported, summary.unsupported, limits.max_unsupported),
        (CompatibilityStatus::EngineGap, summary.engine_gap, limits.max_engine_gap),
    ] {
        if actual > maximum {
            issues.push(OracleIssue::CompatibilityLimitExceeded(status));
        }
    }
}

fn increment_status(summary: &mut CompatibilitySummary, status: CompatibilityStatus) {
    match status {
        CompatibilityStatus::Compatible => summary.compatible = summary.compatible.saturating_add(1),
        CompatibilityStatus::Adapted => summary.adapted = summary.adapted.saturating_add(1),
        CompatibilityStatus::Intentional => summary.intentional = summary.intentional.saturating_add(1),
        CompatibilityStatus::Unsupported => summary.unsupported = summary.unsupported.saturating_add(1),
        CompatibilityStatus::EngineGap => summary.engine_gap = summary.engine_gap.saturating_add(1),
    }
}
