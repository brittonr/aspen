type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Record<T> = preserves::Record<T>;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

use preserves::ValueImpl;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
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

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const MODE_STRUCTURAL: &str = "structural";
pub const MODE_UNIQUE: &str = "unique";
pub const MODE_BRANDED_STRUCTURAL: &str = "branded-structural";

pub const DECISION_EXACT_ARTIFACT_MATCH: &str = "exact-artifact-match";
pub const DECISION_STRUCTURAL_MATCH: &str = "structural-match";
pub const DECISION_BRAND_MATCH: &str = "brand-match";
pub const DECISION_ADMITTED_ALIAS: &str = "admitted-alias";
pub const DECISION_MIGRATION_AVAILABLE: &str = "migration-available";
pub const DECISION_MISMATCH_REQUIRES_MIGRATION: &str = "mismatch-requires-migration";
pub const DECISION_DENIED_BY_POLICY: &str = "denied-by-policy";

const MAX_SEARCH_MATCHES: usize = 4_096;
const _: () = assert!(MAX_SEARCH_MATCHES > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityInput {
    pub mode: String,
    pub schema_ref: String,
    pub shape: IoValue,
    pub brand_ref: Option<String>,
    pub metadata_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub identity_ref: String,
    pub mode: String,
    pub schema_ref: String,
    pub normalized_shape_ref: String,
    pub structural_fingerprint: String,
    pub brand_ref: Option<String>,
    pub metadata_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasInput {
    pub from_schema_ref: String,
    pub to_schema_ref: String,
    pub scope: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias {
    pub alias_ref: String,
    pub from_schema_ref: String,
    pub to_schema_ref: String,
    pub scope: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityInput {
    pub expected: Identity,
    pub actual: Identity,
    pub alias: Option<Alias>,
    pub migration_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub deny_by_policy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compatibility {
    pub compatibility_ref: String,
    pub decision: String,
    pub expected_identity_ref: String,
    pub expected_schema_ref: String,
    pub actual_identity_ref: String,
    pub actual_schema_ref: String,
    pub alias_ref: Option<String>,
    pub migration_ref: Option<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub compatibility_ref: String,
    pub value: IoValue,
}

pub fn normalize_shape(shape: &IoValue) -> Result<IoValue> {
    if let Some(fields) = shape.collect_simple_record("shape", None) {
        if fields.len() == 0 {
            return Err(MoltenError::invalid_harness("schema shape record requires kind"));
        }
        let kind = required_string(&fields[0], "shape kind")?;
        match kind.as_str() {
            "record" => normalize_record_shape(&fields),
            "field" => normalize_field_shape(&fields),
            "sequence" => normalize_unary_shape("sequence", &fields),
            "option" => normalize_unary_shape("option", &fields),
            "map" => normalize_binary_shape("map", &fields),
            "string" | "bytes" | "u64" | "i64" | "bool" | "symbol" | "any-preserves" => {
                if fields.len() != 1 {
                    return Err(MoltenError::invalid_harness(format!("shape {kind} expects no additional fields")));
                }
                Ok(record("shape", vec![string(kind)]))
            }
            other => Err(MoltenError::invalid_harness(format!("unsupported normalized schema shape kind {other}"))),
        }
    } else {
        Err(MoltenError::invalid_harness("expected <shape ...> normalized schema shape"))
    }
}

pub fn structural_fingerprint(shape: &IoValue) -> Result<(IoValue, String, String)> {
    let normalized = normalize_shape(shape)?;
    let normalized_shape_ref = canonical_hash(&normalized)?;
    let fingerprint = canonical_hash(&record("schema-structural-fingerprint-v1", vec![
        string(crate::preserves_rail::SCHEMA_STRUCTURAL_FINGERPRINT_SCHEMA),
        normalized.clone(),
    ]))?;
    Ok((normalized, normalized_shape_ref, fingerprint))
}

pub fn identity_value(input: &IdentityInput) -> Result<IoValue> {
    validate_mode(&input.mode)?;
    validate_ref(&input.schema_ref, "schema identity schema ref")?;
    validate_refs(&input.metadata_refs, "schema identity metadata ref")?;
    validate_refs(&input.policy_refs, "schema identity policy ref")?;
    validate_refs(&input.evidence_refs, "schema identity evidence ref")?;
    if let Some(brand_ref) = input.brand_ref.as_ref() {
        validate_ref(brand_ref, "schema identity brand ref")?;
    }
    if input.mode == MODE_BRANDED_STRUCTURAL && input.brand_ref.is_none() {
        return Err(MoltenError::invalid_harness("branded-structural schema identity requires brand ref"));
    }
    if input.mode != MODE_BRANDED_STRUCTURAL && input.brand_ref.is_some() {
        return Err(MoltenError::invalid_harness("only branded-structural schema identity may include brand ref"));
    }
    let (normalized_shape, normalized_shape_ref, fingerprint) = structural_fingerprint(&input.shape)?;
    Ok(record("schema-identity-v1", vec![
        string(crate::preserves_rail::SCHEMA_IDENTITY_SCHEMA),
        record("mode", vec![string(&input.mode)]),
        record("schema", vec![string(&input.schema_ref)]),
        record("shape", vec![string(&normalized_shape_ref), string(&fingerprint), normalized_shape]),
        record("brand", vec![optional_ref_value(input.brand_ref.as_deref())]),
        record("metadata", vec![refs_sequence(&input.metadata_refs)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("evidence", vec![refs_sequence(&input.evidence_refs)]),
        checks_value(&[
            "names-not-identity",
            "domain-separated-fingerprint",
            "unique-not-structural-by-default",
            "content-addressing-is-not-trust",
        ]),
    ]))
}

pub fn parse_identity(value: &IoValue) -> Result<Identity> {
    let fields = value
        .collect_simple_record("schema-identity-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <schema-identity-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::SCHEMA_IDENTITY_SCHEMA, "schema identity")?;
    let mode = record_string(&fields[1], "mode")?;
    validate_mode(&mode)?;
    let shape_record = value_to_iovalue(&fields[3]);
    let shape_fields = simple_record(&shape_record, "shape", 3)?;
    let normalized_shape_ref = required_ref(&shape_fields[0], "normalized shape ref")?;
    let fingerprint = required_ref(&shape_fields[1], "structural fingerprint")?;
    let normalized = value_to_iovalue(&shape_fields[2]);
    let (_normalized, actual_shape_ref, actual_fingerprint) = structural_fingerprint(&normalized)?;
    if actual_shape_ref != normalized_shape_ref || actual_fingerprint != fingerprint {
        return Err(MoltenError::invalid_harness("schema identity shape fingerprint mismatch"));
    }
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "names-not-identity", "schema identity")?;
    let brand_ref = record_optional_ref(&fields[4], "brand")?;
    if mode == MODE_BRANDED_STRUCTURAL && brand_ref.is_none() {
        return Err(MoltenError::invalid_harness("branded-structural schema identity missing brand ref"));
    }
    if mode != MODE_BRANDED_STRUCTURAL && brand_ref.is_some() {
        return Err(MoltenError::invalid_harness("non-branded schema identity includes brand ref"));
    }
    Ok(Identity {
        identity_ref: canonical_hash(value)?,
        mode,
        schema_ref: record_ref(&fields[2], "schema")?,
        normalized_shape_ref,
        structural_fingerprint: fingerprint,
        brand_ref,
        metadata_refs: record_ref_sequence(&fields[5], "metadata")?,
        policy_refs: record_ref_sequence(&fields[6], "policy")?,
        evidence_refs: record_ref_sequence(&fields[7], "evidence")?,
        value: value.clone(),
    })
}

pub fn alias_value(input: &AliasInput) -> Result<IoValue> {
    validate_ref(&input.from_schema_ref, "schema alias from ref")?;
    validate_ref(&input.to_schema_ref, "schema alias to ref")?;
    validate_alias_scope(&input.scope)?;
    validate_refs(&input.policy_refs, "schema alias policy ref")?;
    validate_refs(&input.evidence_refs, "schema alias evidence ref")?;
    Ok(record("schema-alias-v1", vec![
        string(crate::preserves_rail::SCHEMA_ALIAS_SCHEMA),
        record("from", vec![string(&input.from_schema_ref)]),
        record("to", vec![string(&input.to_schema_ref)]),
        record("scope", vec![string(&input.scope)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("evidence", vec![refs_sequence(&input.evidence_refs)]),
        checks_value(&["alias-is-not-name", "directional-alias", "policy-admission-required"]),
    ]))
}

pub fn parse_alias(value: &IoValue) -> Result<Alias> {
    let fields = value
        .collect_simple_record("schema-alias-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <schema-alias-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::SCHEMA_ALIAS_SCHEMA, "schema alias")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "alias-is-not-name", "schema alias")?;
    let scope = record_string(&fields[3], "scope")?;
    validate_alias_scope(&scope)?;
    Ok(Alias {
        alias_ref: canonical_hash(value)?,
        from_schema_ref: record_ref(&fields[1], "from")?,
        to_schema_ref: record_ref(&fields[2], "to")?,
        scope,
        policy_refs: record_ref_sequence(&fields[4], "policy")?,
        evidence_refs: record_ref_sequence(&fields[5], "evidence")?,
        value: value.clone(),
    })
}

pub fn compatibility_decision_value(input: &CompatibilityInput) -> Result<IoValue> {
    validate_refs(&input.policy_refs, "schema compatibility policy ref")?;
    validate_refs(&input.evidence_refs, "schema compatibility evidence ref")?;
    if let Some(migration_ref) = input.migration_ref.as_ref() {
        validate_ref(migration_ref, "schema compatibility migration ref")?;
    }
    let decision = compatibility_decision(input)?;
    Ok(record("schema-compatibility-v1", vec![
        string(crate::preserves_rail::SCHEMA_COMPATIBILITY_SCHEMA),
        record("decision", vec![string(&decision)]),
        compatibility_identity_record("expected", &input.expected),
        compatibility_identity_record("actual", &input.actual),
        record("alias", vec![optional_ref_value(
            input.alias.as_ref().map(|alias| alias.alias_ref.as_str()),
        )]),
        record("migration", vec![optional_ref_value(input.migration_ref.as_deref())]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("evidence", vec![refs_sequence(&input.evidence_refs)]),
        checks_value(&[
            "unique-not-structural-by-default",
            "alias-is-explicit",
            "migration-is-explicit",
            "policy-denial-wins",
        ]),
    ]))
}
