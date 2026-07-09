pub const COVERAGE_MATRIX_SCHEMA: &str = "molten.determinism.replay-coverage-matrix.v1";

const COVERAGE_ROW_LIMIT: usize = 64;
const COVERAGE_DIAGNOSTIC_LIMIT: usize = 64;
const _: () = assert!(COVERAGE_ROW_LIMIT > 0);
const _: () = assert!(COVERAGE_DIAGNOSTIC_LIMIT > 0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageEligibility {
    Deterministic,
    Recorded,
    DiagnosticOnly,
    NonReplayable,
}

impl CoverageEligibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic",
            Self::Recorded => "recorded",
            Self::DiagnosticOnly => "diagnostic-only",
            Self::NonReplayable => "non-replayable",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageRow {
    pub subsystem: String,
    pub workflow: String,
    pub eligibility: CoverageEligibility,
    pub fresh_run_ref: Option<String>,
    pub verify_ref: Option<String>,
    pub second_fresh_run_ref: Option<String>,
    pub negative_evidence_ref: Option<String>,
    pub index_ref: Option<String>,
    pub caveat_refs: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageMatrix {
    pub decision: String,
    pub matrix_ref: String,
    pub diagnostics: Vec<String>,
    pub rows: Vec<CoverageRow>,
    pub value: IoValue,
}

pub fn validate_coverage_matrix(rows: &[CoverageRow]) -> Result<CoverageMatrix> {
    validate_coverage_row_count(rows)?;
    let mut diagnostics = Vec::new();
    let mut seen = OrderedSet::new();
    for row in rows {
        validate_coverage_row(row, &mut diagnostics)?;
        let key = coverage_row_key(row);
        if !seen.insert(key.clone()) {
            push_coverage_diagnostic(&mut diagnostics, format!("duplicate replay coverage row {key}"))?;
        }
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let value = coverage_matrix_value(&decision, rows, &diagnostics)?;
    let matrix_ref = canonical_hash(&value)?;
    Ok(CoverageMatrix {
        decision,
        matrix_ref,
        diagnostics,
        rows: rows.to_vec(),
        value,
    })
}

fn validate_coverage_row(row: &CoverageRow, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
    validate_token(&row.subsystem, "coverage subsystem")?;
    validate_token(&row.workflow, "coverage workflow")?;
    validate_optional_ref(row.fresh_run_ref.as_deref(), "coverage fresh run ref")?;
    validate_optional_ref(row.verify_ref.as_deref(), "coverage verify ref")?;
    validate_optional_ref(row.second_fresh_run_ref.as_deref(), "coverage second fresh run ref")?;
    validate_optional_ref(row.negative_evidence_ref.as_deref(), "coverage negative evidence ref")?;
    validate_optional_ref(row.index_ref.as_deref(), "coverage replay index ref")?;
    validate_ref_slice(&row.caveat_refs, "coverage caveat ref")?;
    match row.eligibility {
        CoverageEligibility::Deterministic | CoverageEligibility::Recorded => validate_required_coverage(row, diagnostics)?,
        CoverageEligibility::DiagnosticOnly => validate_diagnostic_only_coverage(row, diagnostics)?,
        CoverageEligibility::NonReplayable => validate_non_replayable_coverage(row, diagnostics)?,
    }
    Ok(())
}

fn validate_required_coverage(
    row: &CoverageRow,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if row.fresh_run_ref.is_none() {
        push_coverage_diagnostic(diagnostics, format!("{} missing fresh run evidence", coverage_row_key(row)))?;
    }
    if row.verify_ref.is_none() {
        push_coverage_diagnostic(diagnostics, format!("{} missing replay verify evidence", coverage_row_key(row)))?;
    }
    if row.second_fresh_run_ref.is_none() {
        push_coverage_diagnostic(diagnostics, format!("{} missing second fresh run evidence", coverage_row_key(row)))?;
    }
    if row.negative_evidence_ref.is_none() {
        push_coverage_diagnostic(diagnostics, format!("{} missing negative tamper evidence", coverage_row_key(row)))?;
    }
    Ok(())
}

fn validate_diagnostic_only_coverage(
    row: &CoverageRow,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if row.verify_ref.is_some() {
        push_coverage_diagnostic(
            diagnostics,
            format!("{} is diagnostic-only and cannot satisfy deterministic replay evidence", coverage_row_key(row)),
        )?;
    }
    if row.caveat_refs.is_empty() {
        push_coverage_diagnostic(diagnostics, format!("{} missing diagnostic-only caveat", coverage_row_key(row)))?;
    }
    Ok(())
}

fn validate_non_replayable_coverage(
    row: &CoverageRow,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if row.verify_ref.is_some() || row.fresh_run_ref.is_some() {
        push_coverage_diagnostic(
            diagnostics,
            format!("{} is non-replayable and cannot report replay pass evidence", coverage_row_key(row)),
        )?;
    }
    if row.caveat_refs.is_empty() {
        push_coverage_diagnostic(diagnostics, format!("{} missing non-replayable caveat", coverage_row_key(row)))?;
    }
    Ok(())
}

fn coverage_matrix_value(decision: &str, rows: &[CoverageRow], diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("replay-coverage-matrix-v1", vec![
        string(COVERAGE_MATRIX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("rows", vec![sequence(rows.iter().map(coverage_row_value).collect())]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("evidence-only"), string("pass")]),
            record("check", vec![string("positive-and-negative-evidence"), string(decision)]),
            record("check", vec![string("diagnostic-only-excluded"), string(decision)]),
        ])]),
    ]))
}

fn coverage_row_value(row: &CoverageRow) -> IoValue {
    record("coverage-row", vec![
        record("subsystem", vec![string(&row.subsystem)]),
        record("workflow", vec![string(&row.workflow)]),
        record("eligibility", vec![string(row.eligibility.as_str())]),
        record("fresh-run", vec![optional_ref_value(row.fresh_run_ref.as_deref())]),
        record("replay-verify", vec![optional_ref_value(row.verify_ref.as_deref())]),
        record("second-fresh-run", vec![optional_ref_value(row.second_fresh_run_ref.as_deref())]),
        record("negative-evidence", vec![optional_ref_value(row.negative_evidence_ref.as_deref())]),
        record("replay-index", vec![optional_ref_value(row.index_ref.as_deref())]),
        record("caveats", vec![sequence(row.caveat_refs.iter().map(string).collect())]),
    ])
}

fn validate_coverage_row_count(rows: &[CoverageRow]) -> Result<()> {
    if rows.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("replay coverage matrix requires rows"));
    }
    if rows.len() > COVERAGE_ROW_LIMIT {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "replay coverage rows {} exceed bound {COVERAGE_ROW_LIMIT}",
            rows.len()
        )));
    }
    Ok(())
}

fn validate_ref_slice(refs: &[String], field: &str) -> Result<()> {
    if refs.len() > COVERAGE_ROW_LIMIT {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "{field} count {} exceeds bound {COVERAGE_ROW_LIMIT}",
            refs.len()
        )));
    }
    for reference in refs {
        validate_content_ref(reference)?;
    }
    Ok(())
}

fn validate_optional_ref(reference: Option<&str>, field: &str) -> Result<()> {
    if let Some(reference) = reference {
        validate_content_ref(reference).map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("invalid {field} {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_token(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(format!("{field} cannot be empty")));
    }
    if value.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!("{field} must be lowercase ascii token")))
    }
}

fn coverage_row_key(row: &CoverageRow) -> String {
    format!("{}/{}", row.subsystem, row.workflow)
}

fn push_coverage_diagnostic(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    diagnostic: impl Into<String>,
) -> Result<()> {
    crate::bounded::push_bounded(
        diagnostics,
        diagnostic.into(),
        COVERAGE_DIAGNOSTIC_LIMIT,
        "replay coverage diagnostics",
    )
}
