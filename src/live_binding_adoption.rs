use preserves::IOValue;
use preserves::Value;

use crate::error::MoltenError;
use crate::error::Result;

pub const BINDING_RECORD_SCHEMA: &str = "molten.binding.record.v1";
pub const BINDING_SNAPSHOT_SCHEMA: &str = "molten.binding.snapshot.v1";
pub const RESOLUTION_RECEIPT_SCHEMA: &str = "molten.binding.resolution-receipt.v1";
pub const TRANSITION_RECEIPT_SCHEMA: &str = "molten.binding.transition-receipt.v1";
pub const ROOT_INVENTORY_SCHEMA: &str = "molten.retirement.root-inventory.v1";
pub const GENERATION_ATTRIBUTION_SCHEMA: &str = "molten.retirement.generation-attribution.v1";
pub const RETIREMENT_REPORT_SCHEMA: &str = "molten.retirement.report.v1";
pub const DEPLOY_DIAGNOSTIC_SCHEMA: &str = "molten.retirement.deploy-diagnostic.v1";
pub const SEMANTIC_OPERATION_BINDING_SCHEMA: &str = "molten.effects.semantic-operation-binding.v1";

const ADOPTION_ARTIFACT_ARITY: usize = 3;
const FIELD_ARITY: usize = 2;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AdoptionArtifactKind {
    BindingRecord,
    BindingSnapshot,
    ResolutionReceipt,
    TransitionReceipt,
    RootInventory,
    GenerationAttribution,
    RetirementReport,
    DeployDiagnostic,
    SemanticOperationBinding,
}

impl AdoptionArtifactKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::BindingRecord => "binding-record-v1",
            Self::BindingSnapshot => "binding-snapshot-v1",
            Self::ResolutionReceipt => "binding-resolution-receipt-v1",
            Self::TransitionReceipt => "binding-transition-receipt-v1",
            Self::RootInventory => "retirement-root-inventory-v1",
            Self::GenerationAttribution => "generation-attribution-v1",
            Self::RetirementReport => "retirement-report-v1",
            Self::DeployDiagnostic => "deploy-diagnostic-v1",
            Self::SemanticOperationBinding => "semantic-operation-binding-v1",
        }
    }

    pub const fn schema(self) -> &'static str {
        match self {
            Self::BindingRecord => BINDING_RECORD_SCHEMA,
            Self::BindingSnapshot => BINDING_SNAPSHOT_SCHEMA,
            Self::ResolutionReceipt => RESOLUTION_RECEIPT_SCHEMA,
            Self::TransitionReceipt => TRANSITION_RECEIPT_SCHEMA,
            Self::RootInventory => ROOT_INVENTORY_SCHEMA,
            Self::GenerationAttribution => GENERATION_ATTRIBUTION_SCHEMA,
            Self::RetirementReport => RETIREMENT_REPORT_SCHEMA,
            Self::DeployDiagnostic => DEPLOY_DIAGNOSTIC_SCHEMA,
            Self::SemanticOperationBinding => SEMANTIC_OPERATION_BINDING_SCHEMA,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CanonicalField {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdoptionArtifact {
    pub kind: AdoptionArtifactKind,
    pub artifact_ref: String,
    pub fields: Vec<CanonicalField>,
    pub non_claims: Vec<String>,
    pub value: IOValue,
}

fn validate_field(field: &CanonicalField) -> Result<()> {
    if field.name.is_empty() || field.value.is_empty() {
        return Err(MoltenError::invalid_harness("adoption artifact field name and value must not be empty"));
    }
    if !field.name.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-') {
        return Err(MoltenError::invalid_harness("adoption artifact field name must be a lowercase token"));
    }
    Ok(())
}

fn normalize_fields(fields: &[CanonicalField]) -> Result<Vec<CanonicalField>> {
    let mut normalized = fields.to_vec();
    for field in &normalized {
        validate_field(field)?;
    }
    normalized.sort();
    if normalized.windows(2).any(|pair| pair[0].name == pair[1].name) {
        return Err(MoltenError::invalid_harness("adoption artifact contains a duplicate field"));
    }
    Ok(normalized)
}

fn normalize_non_claims(non_claims: &[String]) -> Result<Vec<String>> {
    if non_claims.is_empty() || non_claims.iter().any(String::is_empty) {
        return Err(MoltenError::invalid_harness("adoption artifact requires non-empty non-claims"));
    }
    let mut normalized = non_claims.to_vec();
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn artifact_value(kind: AdoptionArtifactKind, fields: &[CanonicalField], non_claims: &[String]) -> IOValue {
    crate::preserves_rail::record(kind.label(), vec![
        crate::preserves_rail::string(kind.schema()),
        crate::preserves_rail::record("fields", vec![crate::preserves_rail::sequence(
            fields
                .iter()
                .map(|field| {
                    crate::preserves_rail::record("field", vec![
                        crate::preserves_rail::string(&field.name),
                        crate::preserves_rail::string(&field.value),
                    ])
                })
                .collect(),
        )]),
        crate::preserves_rail::record("non-claims", vec![crate::preserves_rail::sequence(
            non_claims.iter().map(crate::preserves_rail::string).collect(),
        )]),
    ])
}

// r[impl molten.artifacts.live_binding.cutover]
// r[impl molten.retirement.trace_report]
// r[impl molten.retirement.deploy_diagnostics]
pub fn build_adoption_artifact(
    kind: AdoptionArtifactKind,
    fields: &[CanonicalField],
    non_claims: &[String],
) -> Result<AdoptionArtifact> {
    let fields = normalize_fields(fields)?;
    let non_claims = normalize_non_claims(non_claims)?;
    let value = artifact_value(kind, &fields, &non_claims);
    Ok(AdoptionArtifact {
        kind,
        artifact_ref: crate::preserves_rail::canonical_hash(&value)?,
        fields,
        non_claims,
        value,
    })
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn parse_fields(value: &Value<IOValue>) -> Result<Vec<CanonicalField>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = value
        .collect_simple_record("fields", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected adoption fields record"))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected adoption field sequence"))?;
    let mut fields = Vec::with_capacity(values.len());
    for value in values.iter() {
        let value = crate::preserves_rail::value_to_iovalue(value);
        let field = value
            .collect_simple_record("field", Some(FIELD_ARITY))
            .ok_or_else(|| MoltenError::invalid_harness("expected adoption field record"))?;
        fields.push(CanonicalField {
            name: required_string(&field[0], "adoption field name")?,
            value: required_string(&field[1], "adoption field value")?,
        });
    }
    normalize_fields(&fields)
}

fn parse_non_claims(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = value
        .collect_simple_record("non-claims", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected non-claims record"))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected non-claims sequence"))?;
    let mut non_claims = Vec::with_capacity(values.len());
    for value in values.iter() {
        non_claims.push(required_string(value, "adoption non-claim")?);
    }
    normalize_non_claims(&non_claims)
}

pub fn parse_adoption_artifact(kind: AdoptionArtifactKind, value: &IOValue) -> Result<AdoptionArtifact> {
    let record = value
        .collect_simple_record(kind.label(), Some(ADOPTION_ARTIFACT_ARITY))
        .ok_or_else(|| MoltenError::invalid_harness("unexpected adoption artifact label or arity"))?;
    let schema = required_string(&record[0], "adoption artifact schema")?;
    if schema != kind.schema() {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported adoption schema {schema}; expected {}",
            kind.schema()
        )));
    }
    Ok(AdoptionArtifact {
        kind,
        artifact_ref: crate::preserves_rail::canonical_hash(value)?,
        fields: parse_fields(&record[1])?,
        non_claims: parse_non_claims(&record[2])?,
        value: value.clone(),
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const SEMANTIC_MAPPING_SCHEMA: &str = "molten.semantic-operation-mapping.v1";
    const ADOPTION_KIND_COUNT: usize = 9;

    #[derive(Deserialize)]
    struct SemanticMappingFixture {
        schema: String,
        kamacite_revision: String,
        operation_hex: String,
        surface_hexes: Vec<String>,
        expected: String,
    }

    const KINDS: [AdoptionArtifactKind; ADOPTION_KIND_COUNT] = [
        AdoptionArtifactKind::BindingRecord,
        AdoptionArtifactKind::BindingSnapshot,
        AdoptionArtifactKind::ResolutionReceipt,
        AdoptionArtifactKind::TransitionReceipt,
        AdoptionArtifactKind::RootInventory,
        AdoptionArtifactKind::GenerationAttribution,
        AdoptionArtifactKind::RetirementReport,
        AdoptionArtifactKind::DeployDiagnostic,
        AdoptionArtifactKind::SemanticOperationBinding,
    ];

    #[test]
    fn every_adoption_artifact_has_deterministic_preserves_roundtrip() {
        for kind in KINDS {
            let built = build_adoption_artifact(
                kind,
                &[
                    CanonicalField {
                        name: "snapshot".to_string(),
                        value: "blake3:snapshot".to_string(),
                    },
                    CanonicalField {
                        name: "subject".to_string(),
                        value: "blake3:subject".to_string(),
                    },
                ],
                &["artifact is evidence, not authority".to_string()],
            )
            .expect("build adoption artifact");
            let parsed = parse_adoption_artifact(kind, &built.value).expect("parse adoption artifact");
            assert_eq!(parsed.artifact_ref, built.artifact_ref);
            assert_eq!(parsed.fields, built.fields);
        }
    }

    #[test]
    fn governed_semantic_mapping_fixtures_cover_exact_and_drift_cases() {
        let positive: SemanticMappingFixture =
            serde_json::from_str(include_str!("../fixtures/semantic-operation/mapping-default.json"))
                .expect("parse positive semantic mapping fixture");
        assert_eq!(positive.schema, SEMANTIC_MAPPING_SCHEMA);
        assert_eq!(positive.kamacite_revision, molten_core::live_binding::KAMACITE_SEMANTIC_REVISION);
        assert_eq!(positive.surface_hexes.len(), molten_core::live_binding::SEMANTIC_SURFACE_COUNT);
        assert!(positive.surface_hexes.iter().all(|identity| identity == &positive.operation_hex));
        assert_eq!(positive.expected, "pass");

        let negative: SemanticMappingFixture =
            serde_json::from_str(include_str!("../fixtures/semantic-operation/mapping-drift.json"))
                .expect("parse negative semantic mapping fixture");
        assert!(negative.surface_hexes.iter().any(|identity| identity != &negative.operation_hex));
        assert_eq!(negative.expected, "deny");
    }

    #[test]
    fn duplicate_fields_and_wrong_artifact_kind_fail_closed() {
        let duplicate = CanonicalField {
            name: "snapshot".to_string(),
            value: "blake3:snapshot".to_string(),
        };
        assert!(
            build_adoption_artifact(AdoptionArtifactKind::BindingRecord, &[duplicate.clone(), duplicate], &[
                "not authority".to_string()
            ],)
            .is_err()
        );
        let built = build_adoption_artifact(
            AdoptionArtifactKind::BindingRecord,
            &[CanonicalField {
                name: "snapshot".to_string(),
                value: "blake3:snapshot".to_string(),
            }],
            &["not authority".to_string()],
        )
        .expect("build binding artifact");
        assert!(parse_adoption_artifact(AdoptionArtifactKind::RetirementReport, &built.value,).is_err());
    }
}
