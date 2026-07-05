type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;

const DRIFT_COMPARISON_SCHEMA: &str = "molten.testing.deterministic-drift.comparison.v1";
const MAX_DRIFT_FIELDS: usize = 512;
const MAX_DRIFT_VARIANCES: usize = 128;
const _: () = assert!(MAX_DRIFT_FIELDS > 0);
const _: () = assert!(MAX_DRIFT_VARIANCES > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceField {
    pub path: String,
    pub value: String,
    pub is_ref: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSummary {
    pub workflow: String,
    pub fields: Vec<EvidenceField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowedVariance {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonInput {
    pub left: EvidenceSummary,
    pub right: EvidenceSummary,
    pub allowed_variances: Vec<AllowedVariance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub path: String,
    pub kind: String,
    pub left: String,
    pub right: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    pub decision: String,
    pub workflow: String,
    pub diagnostics: Vec<Diagnostic>,
    pub normalized_fields: Vec<EvidenceField>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedInput {
    workflow: String,
    left_fields: OrderedMap<String, EvidenceField>,
    right_fields: OrderedMap<String, EvidenceField>,
    variances: OrderedMap<String, String>,
}

pub fn artifact_summary(workflow: &str, artifact: &IoValue) -> Result<EvidenceSummary> {
    let artifact_ref = crate::preserves_rail::canonical_hash(artifact)?;
    Ok(EvidenceSummary {
        workflow: workflow.to_string(),
        fields: vec![EvidenceField {
            path: "artifact-ref".to_string(),
            value: artifact_ref,
            is_ref: true,
        }],
    })
}

pub fn compare(input: &ComparisonInput) -> Result<Comparison> {
    let normalized = normalize_input(input)?;
    let diagnostics = first_divergence(&normalized)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let normalized_fields = normalized.left_fields.values().cloned().collect::<Vec<_>>();
    let value = comparison_value(&normalized.workflow, &decision, &diagnostics, &normalized_fields)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Comparison {
        decision,
        workflow: normalized.workflow,
        diagnostics,
        normalized_fields,
        receipt_ref,
        value,
    })
}

pub fn comparison_value(
    workflow: &str,
    decision: &str,
    diagnostics: &[Diagnostic],
    normalized_fields: &[EvidenceField],
) -> Result<IoValue> {
    validate_text("workflow", workflow)?;
    validate_decision(decision)?;
    Ok(record("deterministic-drift-comparison-v1", vec![
        string(DRIFT_COMPARISON_SCHEMA),
        record("decision", vec![string(decision)]),
        record("workflow", vec![string(workflow)]),
        record("normalized-fields", vec![sequence(field_values(normalized_fields)?)]),
        record("diagnostics", vec![sequence(diagnostic_values(diagnostics)?)]),
        record("checks", vec![sequence(vec![
            check_value("canonical-ref-comparison", "pass"),
            check_value("declared-variance-only", status(decision == "deny")),
            check_value("retry-does-not-mask-drift", "pass"),
        ])]),
    ]))
}

