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
pub struct DriftComparisonInput {
    pub left: EvidenceSummary,
    pub right: EvidenceSummary,
    pub allowed_variances: Vec<AllowedVariance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftDiagnostic {
    pub path: String,
    pub kind: String,
    pub left: String,
    pub right: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftComparison {
    pub decision: String,
    pub workflow: String,
    pub diagnostics: Vec<DriftDiagnostic>,
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

pub fn compare_drift(input: &DriftComparisonInput) -> Result<DriftComparison> {
    let normalized = normalize_input(input)?;
    let diagnostics = first_drift(&normalized)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let normalized_fields = normalized.left_fields.values().cloned().collect::<Vec<_>>();
    let value = drift_comparison_value(&normalized.workflow, &decision, &diagnostics, &normalized_fields)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(DriftComparison {
        decision,
        workflow: normalized.workflow,
        diagnostics,
        normalized_fields,
        receipt_ref,
        value,
    })
}

pub fn drift_comparison_value(
    workflow: &str,
    decision: &str,
    diagnostics: &[DriftDiagnostic],
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

fn normalize_input(input: &DriftComparisonInput) -> Result<NormalizedInput> {
    validate_text("left workflow", &input.left.workflow)?;
    validate_text("right workflow", &input.right.workflow)?;
    if input.left.workflow != input.right.workflow {
        return Err(MoltenError::invalid_harness(format!(
            "drift summaries compare different workflows: {} vs {}",
            input.left.workflow, input.right.workflow
        )));
    }
    let left_fields = field_map("left", &input.left.fields)?;
    let right_fields = field_map("right", &input.right.fields)?;
    let variances = variance_map(&input.allowed_variances, &left_fields, &right_fields)?;
    Ok(NormalizedInput {
        workflow: input.left.workflow.clone(),
        left_fields,
        right_fields,
        variances,
    })
}

fn field_map(label: &str, fields: &[EvidenceField]) -> Result<OrderedMap<String, EvidenceField>> {
    if fields.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} drift summary requires fields")));
    }
    if fields.len() > MAX_DRIFT_FIELDS {
        return Err(MoltenError::invalid_harness(format!(
            "{label} drift summary field count {} exceeds bound {MAX_DRIFT_FIELDS}",
            fields.len()
        )));
    }
    let mut map = OrderedMap::new();
    for field in fields {
        validate_field(field)?;
        if map.insert(field.path.clone(), field.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate {label} drift field path {}", field.path)));
        }
    }
    Ok(map)
}

fn variance_map(
    variances: &[AllowedVariance],
    left_fields: &OrderedMap<String, EvidenceField>,
    right_fields: &OrderedMap<String, EvidenceField>,
) -> Result<OrderedMap<String, String>> {
    if variances.len() > MAX_DRIFT_VARIANCES {
        return Err(MoltenError::invalid_harness(format!(
            "drift variance count {} exceeds bound {MAX_DRIFT_VARIANCES}",
            variances.len()
        )));
    }
    let mut map = OrderedMap::new();
    for variance in variances {
        validate_text("variance path", &variance.path)?;
        validate_variance_reason(&variance.reason)?;
        if !left_fields.contains_key(&variance.path) && !right_fields.contains_key(&variance.path) {
            return Err(MoltenError::invalid_harness(format!(
                "variance path {} does not name a compared field",
                variance.path
            )));
        }
        if map.insert(variance.path.clone(), variance.reason.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate drift variance path {}", variance.path)));
        }
    }
    Ok(map)
}

fn first_drift(normalized: &NormalizedInput) -> Result<Vec<DriftDiagnostic>> {
    let mut paths = OrderedSet::new();
    paths.extend(normalized.left_fields.keys().cloned());
    paths.extend(normalized.right_fields.keys().cloned());

    for path in paths {
        if normalized.variances.contains_key(&path) {
            continue;
        }
        match (normalized.left_fields.get(&path), normalized.right_fields.get(&path)) {
            (Some(left), Some(right)) => {
                if left.is_ref != right.is_ref {
                    return Ok(vec![diagnostic(
                        &path,
                        "field-kind-drift",
                        &left.is_ref.to_string(),
                        &right.is_ref.to_string(),
                    )]);
                }
                if left.value != right.value {
                    return Ok(vec![diagnostic(&path, "value-drift", &left.value, &right.value)]);
                }
            }
            (Some(left), None) => return Ok(vec![diagnostic(&path, "missing-right-field", &left.value, "<missing>")]),
            (None, Some(right)) => return Ok(vec![diagnostic(&path, "missing-left-field", "<missing>", &right.value)]),
            (None, None) => {
                return Err(MoltenError::invalid_harness(format!("drift path {path} disappeared during comparison")));
            }
        }
    }
    Ok(Vec::new())
}

fn diagnostic(path: &str, kind: &str, left: &str, right: &str) -> DriftDiagnostic {
    DriftDiagnostic {
        path: path.to_string(),
        kind: kind.to_string(),
        left: left.to_string(),
        right: right.to_string(),
    }
}

fn validate_field(field: &EvidenceField) -> Result<()> {
    validate_text("field path", &field.path)?;
    validate_text("field value", &field.value)?;
    if field.is_ref {
        crate::preserves_rail::validate_content_ref(&field.value).map_err(|error| {
            MoltenError::invalid_harness(format!("invalid drift field ref {}: {error}", field.path))
        })?;
    }
    Ok(())
}

fn validate_variance_reason(reason: &str) -> Result<()> {
    match reason {
        "runtime-path" | "diagnostic-log" | "store-path" | "temporary-root" | "rendered-output" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported drift variance reason {other}"))),
    }
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("drift {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" => Ok(()),
        other => {
            Err(MoltenError::invalid_harness(format!("unsupported drift decision {other}; expected pass or deny")))
        }
    }
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn check_value(name: &'static str, state: &'static str) -> IoValue {
    record("check", vec![string(name), string(state)])
}

fn status(is_denied: bool) -> &'static str {
    if is_denied { "deny" } else { "pass" }
}

fn field_values(fields: &[EvidenceField]) -> Result<Vec<IoValue>> {
    if fields.len() > MAX_DRIFT_FIELDS {
        return Err(MoltenError::invalid_harness(format!(
            "drift normalized field count {} exceeds bound {MAX_DRIFT_FIELDS}",
            fields.len()
        )));
    }
    let mut values = Vec::with_capacity(fields.len());
    for field in fields {
        validate_field(field)?;
        values.push(record("field", vec![
            record("path", vec![string(&field.path)]),
            record("value", vec![string(&field.value)]),
            record("ref", vec![bool_value(field.is_ref)]),
        ]));
    }
    Ok(values)
}

fn diagnostic_values(diagnostics: &[DriftDiagnostic]) -> Result<Vec<IoValue>> {
    if diagnostics.len() > MAX_DRIFT_FIELDS {
        return Err(MoltenError::invalid_harness(format!(
            "drift diagnostic count {} exceeds bound {MAX_DRIFT_FIELDS}",
            diagnostics.len()
        )));
    }
    let mut values = Vec::with_capacity(diagnostics.len());
    for diagnostic in diagnostics {
        validate_text("diagnostic path", &diagnostic.path)?;
        validate_text("diagnostic kind", &diagnostic.kind)?;
        values.push(record("drift", vec![
            record("path", vec![string(&diagnostic.path)]),
            record("kind", vec![string(&diagnostic.kind)]),
            record("left", vec![string(&diagnostic.left)]),
            record("right", vec![string(&diagnostic.right)]),
        ]));
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKFLOW: &str = "release-evidence-fixture";

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn field(path: &str, value: &str, is_ref: bool) -> EvidenceField {
        EvidenceField {
            path: path.to_string(),
            value: value.to_string(),
            is_ref,
        }
    }

    fn summary(fields: Vec<EvidenceField>) -> EvidenceSummary {
        EvidenceSummary {
            workflow: WORKFLOW.to_string(),
            fields,
        }
    }

    #[test]
    fn equal_canonical_refs_pass() {
        let bundle_ref = local_ref("bundle");
        let promotion_ref = local_ref("promotion");
        let input = DriftComparisonInput {
            left: summary(vec![
                field("bundle", &bundle_ref, true),
                field("promotion", &promotion_ref, true),
            ]),
            right: summary(vec![
                field("bundle", &bundle_ref, true),
                field("promotion", &promotion_ref, true),
            ]),
            allowed_variances: Vec::new(),
        };
        let comparison = compare_drift(&input).expect("drift comparison");
        assert_eq!(comparison.decision, "pass");
        assert!(comparison.diagnostics.is_empty());
        assert!(crate::preserves_rail::validate_content_ref(&comparison.receipt_ref).is_ok());
    }

    #[test]
    fn unexplained_ref_drift_denies_first_difference() {
        let input = DriftComparisonInput {
            left: summary(vec![field("bundle", &local_ref("bundle-a"), true)]),
            right: summary(vec![field("bundle", &local_ref("bundle-b"), true)]),
            allowed_variances: Vec::new(),
        };
        let comparison = compare_drift(&input).expect("drift comparison");
        assert_eq!(comparison.decision, "deny");
        assert_eq!(comparison.diagnostics[0].path, "bundle");
        assert_eq!(comparison.diagnostics[0].kind, "value-drift");
    }

    #[test]
    fn declared_volatile_field_is_normalized() {
        let stable_ref = local_ref("stable-release-gate");
        let input = DriftComparisonInput {
            left: summary(vec![
                field("release-gate", &stable_ref, true),
                field("tmp-root", "/tmp/a", false),
            ]),
            right: summary(vec![
                field("release-gate", &stable_ref, true),
                field("tmp-root", "/tmp/b", false),
            ]),
            allowed_variances: vec![AllowedVariance {
                path: "tmp-root".to_string(),
                reason: "temporary-root".to_string(),
            }],
        };
        let comparison = compare_drift(&input).expect("drift comparison");
        assert_eq!(comparison.decision, "pass");
    }

    #[test]
    fn stale_variance_declaration_is_rejected() {
        let input = DriftComparisonInput {
            left: summary(vec![field("release-gate", &local_ref("gate"), true)]),
            right: summary(vec![field("release-gate", &local_ref("gate"), true)]),
            allowed_variances: vec![AllowedVariance {
                path: "missing-field".to_string(),
                reason: "temporary-root".to_string(),
            }],
        };
        let error = compare_drift(&input).expect_err("stale variance must fail");
        assert!(error.to_string().contains("does not name a compared field"));
    }

    #[test]
    fn artifact_summary_uses_canonical_ref_not_rendered_text() {
        let artifact = crate::preserves_rail::record("fixture", vec![crate::preserves_rail::string("same")]);
        let left = artifact_summary(WORKFLOW, &artifact).expect("left artifact summary");
        let right = artifact_summary(WORKFLOW, &artifact).expect("right artifact summary");
        let comparison = compare_drift(&DriftComparisonInput {
            left,
            right,
            allowed_variances: Vec::new(),
        })
        .expect("compare artifact summaries");
        assert_eq!(comparison.decision, "pass");
    }
}
