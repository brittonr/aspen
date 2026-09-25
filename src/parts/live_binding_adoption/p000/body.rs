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
const SEMANTIC_SURFACE_NAMES: [&str; molten_core::live_binding::SEMANTIC_SURFACE_COUNT] = [
    "manifest",
    "handler-binding",
    "handle",
    "request",
    "response",
    "effect-log",
    "adapter-import",
    "remote-execution",
    "runtime-receipt",
    "replay-identity",
    "evaluation-cache-key",
    "job",
    "upgrade-check",
];

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
    pub value: preserves::IOValue,
}

fn validate_field(field: &CanonicalField) -> crate::error::Result<()> {
    if field.name.is_empty() || field.value.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(
            "adoption artifact field name and value must not be empty",
        ));
    }
    if !field.name.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-') {
        return Err(crate::error::MoltenError::invalid_harness(
            "adoption artifact field name must be a lowercase token",
        ));
    }
    Ok(())
}

fn normalize_fields(fields: &[CanonicalField]) -> crate::error::Result<Vec<CanonicalField>> {
    let mut normalized = fields.to_vec();
    for field in &normalized {
        validate_field(field)?;
    }
    normalized.sort();
    if normalized.windows(2).any(|pair| pair[0].name == pair[1].name) {
        return Err(crate::error::MoltenError::invalid_harness("adoption artifact contains a duplicate field"));
    }
    Ok(normalized)
}

fn normalize_non_claims(non_claims: &[String]) -> crate::error::Result<Vec<String>> {
    if non_claims.is_empty() || non_claims.iter().any(String::is_empty) {
        return Err(crate::error::MoltenError::invalid_harness("adoption artifact requires non-empty non-claims"));
    }
    let mut normalized = non_claims.to_vec();
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn artifact_value(kind: AdoptionArtifactKind, fields: &[CanonicalField], non_claims: &[String]) -> preserves::IOValue {
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
) -> crate::error::Result<AdoptionArtifact> {
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

fn semantic_identity_text(identity: &kamacite_core::Identity) -> String {
    format!("{}:{}:{}", identity.domain, identity.algorithm, identity.hex.as_str())
}

// r[impl molten.effects.semantic_operation_identity]
// r[impl molten.effects.semantic_handler_matching]
pub fn build_strict_semantic_operation_binding(
    declared_operation: &kamacite_core::Identity,
    surfaces: &molten_core::live_binding::SemanticSurfaceBindings,
) -> crate::error::Result<AdoptionArtifact> {
    molten_core::live_binding::validate_semantic_surfaces(declared_operation, surfaces).map_err(|error| {
        crate::error::MoltenError::invalid_harness(format!("strict semantic operation binding denied: {error:?}"))
    })?;
    let mut fields = Vec::with_capacity(molten_core::live_binding::SEMANTIC_SURFACE_COUNT + 1);
    fields.push(CanonicalField {
        name: "declared-operation".to_string(),
        value: semantic_identity_text(declared_operation),
    });
    for (name, identity) in SEMANTIC_SURFACE_NAMES.iter().zip(surfaces.identities()) {
        fields.push(CanonicalField {
            name: (*name).to_string(),
            value: semantic_identity_text(identity),
        });
    }
    build_adoption_artifact(
        AdoptionArtifactKind::SemanticOperationBinding,
        &fields,
        &molten_core::live_binding::SEMANTIC_NON_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect::<Vec<_>>(),
    )
}

fn required_string(value: &preserves::Value<preserves::IOValue>, field: &str) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn parse_fields(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<CanonicalField>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = value
        .collect_simple_record("fields", Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected adoption fields record"))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected adoption field sequence"))?;
    let mut fields = Vec::with_capacity(values.len());
    for value in values.iter() {
        let value = crate::preserves_rail::value_to_iovalue(value);
        let field = value
            .collect_simple_record("field", Some(FIELD_ARITY))
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected adoption field record"))?;
        fields.push(CanonicalField {
            name: required_string(&field[0], "adoption field name")?,
            value: required_string(&field[1], "adoption field value")?,
        });
    }
    normalize_fields(&fields)
}

fn parse_non_claims(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = value
        .collect_simple_record("non-claims", Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected non-claims record"))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected non-claims sequence"))?;
    let mut non_claims = Vec::with_capacity(values.len());
    for value in values.iter() {
        non_claims.push(required_string(value, "adoption non-claim")?);
    }
    normalize_non_claims(&non_claims)
}

pub fn parse_adoption_artifact(
    kind: AdoptionArtifactKind,
    value: &preserves::IOValue,
) -> crate::error::Result<AdoptionArtifact> {
    let record = value
        .collect_simple_record(kind.label(), Some(ADOPTION_ARTIFACT_ARITY))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("unexpected adoption artifact label or arity"))?;
    let schema = required_string(&record[0], "adoption artifact schema")?;
    if schema != kind.schema() {
        return Err(crate::error::MoltenError::invalid_harness(format!(
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
