use molten_core::world_state_oracle::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

pub const ORACLE_SOURCE_RECORD: &str = "molten-semantic-state-oracle-source-v1";
pub const ORACLE_OBSERVATION_RECORD: &str = "molten-semantic-state-oracle-observation-v1";
pub const ORACLE_COMPARISON_RECORD: &str = "molten-semantic-state-oracle-comparison-v1";
pub const ORACLE_PROJECTION_RECORD: &str = "molten-semantic-state-oracle-projection-v1";

const ORACLE_RECORD_CONTEXT: &str = "onixresearch.molten.semantic-state-oracle.record.v1";
const MAX_ORACLE_RECORD_BYTES: usize = 1_048_576;

#[derive(Debug, Clone)]
pub struct CanonicalOracleRecord {
    pub record_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

pub fn canonical_oracle_source(source: &OracleSourceDescriptor) -> Result<CanonicalOracleRecord> {
    let issues = validate_source_descriptor(source);
    if !issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("oracle source denied: {issues:?}")));
    }
    canonical(
        "source",
        record(ORACLE_SOURCE_RECORD, vec![
            field("schema", string(&source.schema)),
            field("repository", string(&source.repository)),
            field("source-revision", string(&source.revision)),
            field("adapter-version", string(&source.adapter_version)),
            field("backend-format", string(&source.backend_format)),
            field("imported-scope", sequence(source.imported_scope.iter().map(string).collect())),
            field("build-inputs", sequence(source.build_inputs.iter().map(string).collect())),
            field("notice-refs", sequence(source.notice_refs.iter().map(string).collect())),
            field("contract-refs", sequence(source.contract_refs.iter().map(string).collect())),
            field("remotes-enabled", boolean(source.remotes_enabled)),
            field("vec1-enabled", boolean(source.vec1_enabled)),
            field("build-ref", string(&source.build_ref)),
            field("bounds", bounds_value(source.bounds)?),
        ]),
    )
}

pub fn canonical_oracle_observation(observation: &OracleObservation) -> Result<CanonicalOracleRecord> {
    let issues = validate_observation(observation, OracleBounds::standard(), true);
    if !issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("oracle observation denied: {issues:?}")));
    }
    canonical(
        "observation",
        record(ORACLE_OBSERVATION_RECORD, vec![
            field("schema", string(&observation.schema)),
            field("observation-ref", string(&observation.observation_ref)),
            field("source-revision", string(&observation.source_revision)),
            field("build-ref", string(&observation.build_ref)),
            field("adapter-ref", string(&observation.adapter_ref)),
            field("case", string(observation.case.as_str())),
            field("branch", optional_string(observation.branch.as_deref())),
            field("rows", sequence(observation.rows.iter().map(row_value).collect())),
            field("outcome", string(observation.outcome.as_str())),
            field("backend-root", optional_string(observation.backend_root.as_deref())),
            field("backend-root-is-global-identity", boolean(observation.backend_root_is_global_identity)),
            field("diagnostics", sequence(observation.diagnostics.iter().map(string).collect())),
            non_claims(&observation.non_claims),
        ]),
    )
}

pub fn canonical_oracle_comparison(comparison: &OracleComparison) -> Result<CanonicalOracleRecord> {
    let issues = validate_oracle_comparison(comparison);
    if !issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("oracle comparison denied: {issues:?}")));
    }
    canonical(
        "comparison",
        record(ORACLE_COMPARISON_RECORD, vec![
            field("schema", string(&comparison.schema)),
            field("comparison-ref", string(&comparison.comparison_ref)),
            field("expected-ref", string(&comparison.expected_ref)),
            field("observed-ref", string(&comparison.observed_ref)),
            field("decision", string(decision_name(comparison.decision))),
            field("first-divergence", optional_string(comparison.first_divergence.as_deref())),
            field("backend-roots-compared-as-global", boolean(comparison.backend_roots_compared_as_global)),
            non_claims(&comparison.non_claims),
        ]),
    )
}

pub fn canonical_oracle_projection(projection: &OracleEvidenceProjection) -> Result<CanonicalOracleRecord> {
    let issues = validate_oracle_projection(projection);
    if !issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("oracle projection denied: {issues:?}")));
    }
    canonical(
        "projection",
        record(ORACLE_PROJECTION_RECORD, vec![
            field("schema", string(&projection.schema)),
            field("projection-ref", string(&projection.projection_ref)),
            field("consumer", string(projection.consumer.as_str())),
            field("source-revision", string(&projection.source_revision)),
            field("build-ref", string(&projection.build_ref)),
            field("adapter-ref", string(&projection.adapter_ref)),
            field("case", string(projection.case.as_str())),
            field("observation-ref", string(&projection.observation_ref)),
            field("comparison-ref", string(&projection.comparison_ref)),
            field("decision", string(decision_name(projection.decision))),
            field("branch", optional_string(projection.branch.as_deref())),
            field("rows", sequence(projection.rows.iter().map(row_value).collect())),
            field("outcome", string(projection.outcome.as_str())),
            field("backend-root-included", boolean(projection.backend_root_included)),
            field("authority-granted", boolean(projection.authority_granted)),
            field("correctness-proven", boolean(projection.correctness_proven)),
            non_claims(&projection.non_claims),
        ]),
    )
}

fn canonical(kind: &str, value: IOValue) -> Result<CanonicalOracleRecord> {
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_ORACLE_RECORD_BYTES {
        return Err(MoltenError::invalid_harness("oracle record exceeds its canonical byte bound"));
    }
    let mut hasher = blake3::Hasher::new_derive_key(ORACLE_RECORD_CONTEXT);
    update(&mut hasher, kind)?;
    let byte_length =
        u64::try_from(bytes.len()).map_err(|_| MoltenError::invalid_harness("oracle record length exceeds u64"))?;
    hasher.update(&byte_length.to_be_bytes());
    hasher.update(&bytes);
    Ok(CanonicalOracleRecord {
        record_ref: format!("blake3:{}", hasher.finalize().to_hex()),
        value,
        bytes,
    })
}

fn bounds_value(bounds: OracleBounds) -> Result<IOValue> {
    Ok(record("bounds", vec![
        field("max-rows", usize_value(bounds.max_rows)?),
        field("max-key-bytes", usize_value(bounds.max_key_bytes)?),
        field("max-value-bytes", usize_value(bounds.max_value_bytes)?),
        field("max-diagnostics", usize_value(bounds.max_diagnostics)?),
    ]))
}

fn row_value(row: &SemanticStateRow) -> IOValue {
    record("row", vec![field("key", string(&row.key)), field("value", string(&row.value))])
}

fn non_claims(values: &[OracleNonClaim]) -> IOValue {
    field("non-claims", sequence(values.iter().map(|value| string(value.as_str())).collect()))
}

fn decision_name(decision: ComparisonDecision) -> &'static str {
    match decision {
        ComparisonDecision::Agreement => "agreement",
        ComparisonDecision::Divergence => "divergence",
        ComparisonDecision::Unsupported => "unsupported",
    }
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<()> {
    let length =
        u64::try_from(value.len()).map_err(|_| MoltenError::invalid_harness("oracle identity field exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

fn optional_string(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn usize_value(value: usize) -> Result<IOValue> {
    u64::try_from(value)
        .map(number)
        .map_err(|_| MoltenError::invalid_harness("oracle bound exceeds u64"))
}

fn boolean(value: bool) -> IOValue {
    record(if value { "true" } else { "false" }, Vec::new())
}

fn number(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}
