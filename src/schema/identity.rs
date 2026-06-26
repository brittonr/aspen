use std::path::Path;

use preserves::IOValue;
use preserves::Record;
use preserves::Value;
use preserves::ValueImpl;

use crate::artifacts;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;

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

const MAX_SCHEMA_SEARCH_MATCHES: usize = 4_096;
const _: () = assert!(MAX_SCHEMA_SEARCH_MATCHES > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaIdentityInput {
    pub mode: String,
    pub schema_ref: String,
    pub shape: IOValue,
    pub brand_ref: Option<String>,
    pub metadata_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaIdentity {
    pub identity_ref: String,
    pub mode: String,
    pub schema_ref: String,
    pub normalized_shape_ref: String,
    pub structural_fingerprint: String,
    pub brand_ref: Option<String>,
    pub metadata_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaAliasInput {
    pub from_schema_ref: String,
    pub to_schema_ref: String,
    pub scope: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaAlias {
    pub alias_ref: String,
    pub from_schema_ref: String,
    pub to_schema_ref: String,
    pub scope: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaCompatibilityInput {
    pub expected: SchemaIdentity,
    pub actual: SchemaIdentity,
    pub alias: Option<SchemaAlias>,
    pub migration_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub deny_by_policy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaCompatibility {
    pub compatibility_ref: String,
    pub decision: String,
    pub expected_identity_ref: String,
    pub expected_schema_ref: String,
    pub actual_identity_ref: String,
    pub actual_schema_ref: String,
    pub alias_ref: Option<String>,
    pub migration_ref: Option<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaCompatibilityReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub compatibility_ref: String,
    pub value: IOValue,
}

pub fn normalize_shape(shape: &IOValue) -> Result<IOValue> {
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

pub fn structural_fingerprint(shape: &IOValue) -> Result<(IOValue, String, String)> {
    let normalized = normalize_shape(shape)?;
    let normalized_shape_ref = canonical_hash(&normalized)?;
    let fingerprint = canonical_hash(&record("schema-structural-fingerprint-v1", vec![
        string(crate::preserves_rail::SCHEMA_STRUCTURAL_FINGERPRINT_SCHEMA),
        normalized.clone(),
    ]))?;
    Ok((normalized, normalized_shape_ref, fingerprint))
}

pub fn schema_identity_value(input: &SchemaIdentityInput) -> Result<IOValue> {
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

pub fn parse_schema_identity(value: &IOValue) -> Result<SchemaIdentity> {
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
    Ok(SchemaIdentity {
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

pub fn schema_alias_value(input: &SchemaAliasInput) -> Result<IOValue> {
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

pub fn parse_schema_alias(value: &IOValue) -> Result<SchemaAlias> {
    let fields = value
        .collect_simple_record("schema-alias-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <schema-alias-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::SCHEMA_ALIAS_SCHEMA, "schema alias")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "alias-is-not-name", "schema alias")?;
    let scope = record_string(&fields[3], "scope")?;
    validate_alias_scope(&scope)?;
    Ok(SchemaAlias {
        alias_ref: canonical_hash(value)?,
        from_schema_ref: record_ref(&fields[1], "from")?,
        to_schema_ref: record_ref(&fields[2], "to")?,
        scope,
        policy_refs: record_ref_sequence(&fields[4], "policy")?,
        evidence_refs: record_ref_sequence(&fields[5], "evidence")?,
        value: value.clone(),
    })
}

pub fn compatibility_decision_value(input: &SchemaCompatibilityInput) -> Result<IOValue> {
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

pub fn parse_schema_compatibility(value: &IOValue) -> Result<SchemaCompatibility> {
    let fields = value
        .collect_simple_record("schema-compatibility-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <schema-compatibility-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::SCHEMA_COMPATIBILITY_SCHEMA, "schema compatibility")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "unique-not-structural-by-default", "schema compatibility")?;
    let expected = parse_compatibility_identity(&fields[2], "expected")?;
    let actual = parse_compatibility_identity(&fields[3], "actual")?;
    Ok(SchemaCompatibility {
        compatibility_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        expected_identity_ref: expected.0,
        expected_schema_ref: expected.1,
        actual_identity_ref: actual.0,
        actual_schema_ref: actual.1,
        alias_ref: record_optional_ref(&fields[4], "alias")?,
        migration_ref: record_optional_ref(&fields[5], "migration")?,
        value: value.clone(),
    })
}

pub fn compatibility_admits_storage(
    value: &IOValue,
    expected_schema_ref: &str,
    actual_schema_ref: &str,
) -> Result<bool> {
    compatibility_admits_scope(value, expected_schema_ref, actual_schema_ref, "typed storage request")
}

pub fn compatibility_admits_protocol_payload(
    value: &IOValue,
    expected_schema_ref: &str,
    actual_schema_ref: &str,
) -> Result<bool> {
    compatibility_admits_scope(value, expected_schema_ref, actual_schema_ref, "protocol payload request")
}

pub fn compatibility_admits_effect_schema(
    value: &IOValue,
    expected_schema_ref: &str,
    actual_schema_ref: &str,
) -> Result<bool> {
    compatibility_admits_scope(value, expected_schema_ref, actual_schema_ref, "effect schema request")
}

pub fn compatibility_admits_policy_contract_schema(
    value: &IOValue,
    expected_schema_ref: &str,
    actual_schema_ref: &str,
) -> Result<bool> {
    compatibility_admits_scope(value, expected_schema_ref, actual_schema_ref, "policy contract schema request")
}

fn compatibility_admits_scope(
    value: &IOValue,
    expected_schema_ref: &str,
    actual_schema_ref: &str,
    context: &str,
) -> Result<bool> {
    let parsed = parse_schema_compatibility(value)?;
    if parsed.expected_schema_ref != expected_schema_ref || parsed.actual_schema_ref != actual_schema_ref {
        return Err(MoltenError::invalid_harness(format!("schema compatibility refs do not match {context}")));
    }
    Ok(matches!(
        parsed.decision.as_str(),
        DECISION_EXACT_ARTIFACT_MATCH
            | DECISION_STRUCTURAL_MATCH
            | DECISION_BRAND_MATCH
            | DECISION_ADMITTED_ALIAS
            | DECISION_MIGRATION_AVAILABLE
    ))
}

pub fn compatibility_receipt_value(operation: &str, compatibility_value: &IOValue) -> Result<IOValue> {
    validate_non_empty(operation, "schema compatibility receipt operation")?;
    let compatibility = parse_schema_compatibility(compatibility_value)?;
    let decision = if matches!(
        compatibility.decision.as_str(),
        DECISION_EXACT_ARTIFACT_MATCH
            | DECISION_STRUCTURAL_MATCH
            | DECISION_BRAND_MATCH
            | DECISION_ADMITTED_ALIAS
            | DECISION_MIGRATION_AVAILABLE
    ) {
        "pass"
    } else {
        "deny"
    };
    Ok(record("schema-compatibility-receipt-v1", vec![
        string(crate::preserves_rail::SCHEMA_COMPATIBILITY_RECEIPT_SCHEMA),
        record("operation", vec![string(operation)]),
        record("decision", vec![string(decision)]),
        record("compatibility", vec![string(&compatibility.compatibility_ref)]),
        record("expected-schema", vec![string(&compatibility.expected_schema_ref)]),
        record("actual-schema", vec![string(&compatibility.actual_schema_ref)]),
        checks_value(&["schema-compatibility-recorded", "policy-denial-wins"]),
    ]))
}

pub fn parse_compatibility_receipt(value: &IOValue) -> Result<SchemaCompatibilityReceipt> {
    let fields = value
        .collect_simple_record("schema-compatibility-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <schema-compatibility-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::SCHEMA_COMPATIBILITY_RECEIPT_SCHEMA,
        "schema compatibility receipt",
    )?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "schema-compatibility-recorded", "schema compatibility receipt")?;
    Ok(SchemaCompatibilityReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        compatibility_ref: record_ref(&fields[3], "compatibility")?,
        value: value.clone(),
    })
}

pub fn search_registry_by_fingerprint(registry_root: &Path, fingerprint: &str) -> Result<Vec<SchemaIdentity>> {
    validate_ref(fingerprint, "schema structural fingerprint")?;
    let mut matches = Vec::new();
    for artifact in artifacts::list_artifacts(registry_root, Some("schema-identity"))? {
        let payload = artifacts::read_payload(registry_root, &artifact.artifact_ref)?;
        if let Ok(identity) = parse_schema_identity(&payload)
            && identity.structural_fingerprint == fingerprint
        {
            push_bounded(&mut matches, identity, MAX_SCHEMA_SEARCH_MATCHES, "schema search matches")?;
        }
    }
    matches.sort_by(|left, right| left.identity_ref.cmp(&right.identity_ref));
    Ok(matches)
}

fn compatibility_decision(input: &SchemaCompatibilityInput) -> Result<String> {
    if input.deny_by_policy {
        return Ok(DECISION_DENIED_BY_POLICY.to_string());
    }
    if input.expected.schema_ref == input.actual.schema_ref {
        return Ok(DECISION_EXACT_ARTIFACT_MATCH.to_string());
    }
    if let Some(alias) = input.alias.as_ref()
        && alias.from_schema_ref == input.actual.schema_ref
        && alias.to_schema_ref == input.expected.schema_ref
        && matches!(alias.scope.as_str(), "storage" | "effect" | "protocol" | "policy" | "global-local-fixture")
    {
        return Ok(DECISION_ADMITTED_ALIAS.to_string());
    }
    if input.expected.mode == MODE_STRUCTURAL
        && input.actual.mode == MODE_STRUCTURAL
        && input.expected.structural_fingerprint == input.actual.structural_fingerprint
    {
        return Ok(DECISION_STRUCTURAL_MATCH.to_string());
    }
    if input.expected.mode == MODE_BRANDED_STRUCTURAL
        && input.actual.mode == MODE_BRANDED_STRUCTURAL
        && input.expected.brand_ref == input.actual.brand_ref
        && input.expected.structural_fingerprint == input.actual.structural_fingerprint
    {
        return Ok(DECISION_BRAND_MATCH.to_string());
    }
    if input.migration_ref.is_some() {
        return Ok(DECISION_MIGRATION_AVAILABLE.to_string());
    }
    Ok(DECISION_MISMATCH_REQUIRES_MIGRATION.to_string())
}

fn normalize_record_shape(fields: &Record<Value<IOValue>>) -> Result<IOValue> {
    if fields.len() != 3 {
        return Err(MoltenError::invalid_harness("record shape expects label and field sequence"));
    }
    let label = required_string(&fields[1], "record shape label")?;
    let field_items = required_sequence(&fields[2], "record shape fields")?;
    let mut normalized_fields = Vec::with_capacity(field_items.len());
    for field in field_items.iter() {
        normalized_fields.push(normalize_shape(&value_to_iovalue(&field))?);
    }
    normalized_fields.sort_by_key(|field| canonical_hash(field).unwrap_or_else(|_| String::new()));
    Ok(record("shape", vec![string("record"), string(label), sequence(normalized_fields)]))
}

fn normalize_field_shape(fields: &Record<Value<IOValue>>) -> Result<IOValue> {
    if fields.len() != 3 {
        return Err(MoltenError::invalid_harness("field shape expects name and nested shape"));
    }
    Ok(record("shape", vec![
        string("field"),
        required_string(&fields[1], "field shape name").map(string)?,
        normalize_shape(&value_to_iovalue(&fields[2]))?,
    ]))
}

fn normalize_unary_shape(kind: &'static str, fields: &Record<Value<IOValue>>) -> Result<IOValue> {
    if fields.len() != 2 {
        return Err(MoltenError::invalid_harness(format!("{kind} shape expects one nested shape")));
    }
    Ok(record("shape", vec![string(kind), normalize_shape(&value_to_iovalue(&fields[1]))?]))
}

fn normalize_binary_shape(kind: &'static str, fields: &Record<Value<IOValue>>) -> Result<IOValue> {
    if fields.len() != 3 {
        return Err(MoltenError::invalid_harness(format!("{kind} shape expects two nested shapes")));
    }
    Ok(record("shape", vec![
        string(kind),
        normalize_shape(&value_to_iovalue(&fields[1]))?,
        normalize_shape(&value_to_iovalue(&fields[2]))?,
    ]))
}

fn compatibility_identity_record(label: &'static str, identity: &SchemaIdentity) -> IOValue {
    record(label, vec![
        string(&identity.identity_ref),
        string(&identity.schema_ref),
        string(&identity.mode),
        string(&identity.structural_fingerprint),
        optional_ref_value(identity.brand_ref.as_deref()),
    ])
}

fn parse_compatibility_identity(value: &Value<IOValue>, label: &str) -> Result<(String, String)> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 5)?;
    Ok((
        required_ref(&fields[0], "compatibility identity ref")?,
        required_ref(&fields[1], "compatibility schema ref")?,
    ))
}

fn validate_mode(mode: &str) -> Result<()> {
    if matches!(mode, MODE_STRUCTURAL | MODE_UNIQUE | MODE_BRANDED_STRUCTURAL) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported schema identity mode {mode}; expected structural, unique, or branded-structural"
        )))
    }
}

fn validate_alias_scope(scope: &str) -> Result<()> {
    if matches!(scope, "storage" | "effect" | "protocol" | "policy" | "global-local-fixture") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported schema alias scope {scope}; expected storage, effect, protocol, policy, or global-local-fixture"
        )))
    }
}

fn refs_sequence(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}

fn parse_ref_sequence_value(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        refs.push(required_ref(&item, label)?);
    }
    Ok(refs)
}

fn checks_value(names: &[&str]) -> IOValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "checks")?;
    let mut parsed = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(&item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("schema identity check {name} has status {status}")));
        }
        parsed.push(name);
    }
    Ok(parsed)
}

fn require_check(checks: &[String], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &Value<IOValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IOValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IOValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IOValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IOValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &Value<IOValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    if total > maximum {
        return Err(MoltenError::invalid_harness(format!("{label} count {total} exceeds bound {maximum}")));
    }
    values.push_item(value);
    Ok(())
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref, got {value_ref}: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::preserves_rail::parse_text;

    #[test]
    fn structural_fingerprint_ignores_record_field_order() {
        let first = parse_text(
            r#"<shape "record" "profile" [<shape "field" "name" <shape "string">> <shape "field" "age" <shape "u64">>]>"#,
        )
        .expect("first shape");
        let second = parse_text(
            r#"<shape "record" "profile" [<shape "field" "age" <shape "u64">> <shape "field" "name" <shape "string">>]>"#,
        )
        .expect("second shape");
        let (_, _, first_fp) = structural_fingerprint(&first).expect("first fp");
        let (_, _, second_fp) = structural_fingerprint(&second).expect("second fp");
        assert_eq!(first_fp, second_fp);
    }

    #[test]
    fn unique_schemas_need_exact_ref_alias_or_migration_even_with_same_shape() {
        let shape = parse_text(r#"<shape "record" "id" [<shape "field" "value" <shape "string">>]>"#).expect("shape");
        let expected = identity(MODE_UNIQUE, "user-id", &shape, None);
        let actual = identity(MODE_UNIQUE, "order-id", &shape, None);
        let mismatch = compatibility_decision_value(&SchemaCompatibilityInput {
            expected: expected.clone(),
            actual: actual.clone(),
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("mismatch");
        assert_eq!(
            parse_schema_compatibility(&mismatch).expect("parse mismatch").decision,
            DECISION_MISMATCH_REQUIRES_MIGRATION
        );
        let alias = parse_schema_alias(
            &schema_alias_value(&SchemaAliasInput {
                from_schema_ref: actual.schema_ref.clone(),
                to_schema_ref: expected.schema_ref.clone(),
                scope: "storage".to_string(),
                policy_refs: vec![test_ref("alias-policy")],
                evidence_refs: vec![test_ref("alias-evidence")],
            })
            .expect("alias value"),
        )
        .expect("alias");
        let admitted = compatibility_decision_value(&SchemaCompatibilityInput {
            expected: expected.clone(),
            actual: actual.clone(),
            alias: Some(alias.clone()),
            migration_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("alias admitted");
        assert_eq!(parse_schema_compatibility(&admitted).expect("parse admitted").decision, DECISION_ADMITTED_ALIAS);
        let migration = compatibility_decision_value(&SchemaCompatibilityInput {
            expected: expected.clone(),
            actual: actual.clone(),
            alias: None,
            migration_ref: Some(test_ref("migration-recipe")),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("migration available");
        assert_eq!(
            parse_schema_compatibility(&migration).expect("parse migration").decision,
            DECISION_MIGRATION_AVAILABLE
        );
        let reverse = compatibility_decision_value(&SchemaCompatibilityInput {
            expected: actual,
            actual: expected,
            alias: Some(alias),
            migration_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("alias reverse");
        assert_eq!(
            parse_schema_compatibility(&reverse).expect("parse reverse").decision,
            DECISION_MISMATCH_REQUIRES_MIGRATION
        );
    }

    #[test]
    fn structural_and_branded_compatibility_are_explicit() {
        let shape = parse_text(r#"<shape "sequence" <shape "string">>"#).expect("shape");
        let left = identity(MODE_STRUCTURAL, "left", &shape, None);
        let right = identity(MODE_STRUCTURAL, "right", &shape, None);
        let structural = compatibility_decision_value(&SchemaCompatibilityInput {
            expected: left,
            actual: right,
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("structural compat");
        assert_eq!(
            parse_schema_compatibility(&structural).expect("parse structural").decision,
            DECISION_STRUCTURAL_MATCH
        );

        let brand = test_ref("brand");
        let branded_left = identity(MODE_BRANDED_STRUCTURAL, "brand-left", &shape, Some(brand.clone()));
        let branded_right = identity(MODE_BRANDED_STRUCTURAL, "brand-right", &shape, Some(brand));
        let branded = compatibility_decision_value(&SchemaCompatibilityInput {
            expected: branded_left,
            actual: branded_right,
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("brand compat");
        assert_eq!(parse_schema_compatibility(&branded).expect("parse brand").decision, DECISION_BRAND_MATCH);
    }

    #[test]
    fn compatibility_helpers_cover_storage_protocol_effect_and_policy_contracts() {
        let shape =
            parse_text(r#"<shape "record" "event" [<shape "field" "payload" <shape "bytes">>]>"#).expect("shape");
        let expected = identity(MODE_UNIQUE, "expected-event", &shape, None);
        let actual = identity(MODE_UNIQUE, "actual-event", &shape, None);
        for (scope, admits) in [
            ("storage", compatibility_admits_storage as fn(&IOValue, &str, &str) -> Result<bool>),
            ("protocol", compatibility_admits_protocol_payload),
            ("effect", compatibility_admits_effect_schema),
            ("policy", compatibility_admits_policy_contract_schema),
        ] {
            let alias = parse_schema_alias(
                &schema_alias_value(&SchemaAliasInput {
                    from_schema_ref: actual.schema_ref.clone(),
                    to_schema_ref: expected.schema_ref.clone(),
                    scope: scope.to_string(),
                    policy_refs: vec![test_ref(&format!("{scope}-policy"))],
                    evidence_refs: vec![test_ref(&format!("{scope}-evidence"))],
                })
                .expect("alias value"),
            )
            .expect("alias");
            let compatibility = compatibility_decision_value(&SchemaCompatibilityInput {
                expected: expected.clone(),
                actual: actual.clone(),
                alias: Some(alias),
                migration_ref: None,
                policy_refs: vec![test_ref("policy")],
                evidence_refs: vec![test_ref("evidence")],
                deny_by_policy: false,
            })
            .expect("compatibility");
            assert_eq!(
                parse_schema_compatibility(&compatibility).expect("parse compatibility").decision,
                DECISION_ADMITTED_ALIAS
            );
            assert!(admits(&compatibility, &expected.schema_ref, &actual.schema_ref).expect("admitted"));
            let receipt = compatibility_receipt_value(scope, &compatibility).expect("receipt");
            assert_eq!(parse_compatibility_receipt(&receipt).expect("parse receipt").decision, "pass");
        }
    }

    #[test]
    fn registry_search_finds_matching_fingerprints() {
        let root = temp_dir("schema-registry");
        let shape = parse_text(r#"<shape "string">"#).expect("shape");
        let identity = identity(MODE_STRUCTURAL, "search", &shape, None);
        let installed = artifacts::install_artifact(&root, &artifacts::ArtifactInstallInput {
            kind: "schema-identity".to_string(),
            payload: identity.value.clone(),
            schema_refs: vec![identity.schema_ref.clone()],
            dependency_refs: vec![identity.schema_ref.clone()],
            effect_manifest_ref: None,
            policy_refs: identity.policy_refs.clone(),
            evidence_refs: identity.evidence_refs.clone(),
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install identity");
        assert_eq!(installed.decision, "deny", "identity depends on schema ref not installed yet");
        let schema_artifact = artifacts::install_artifact(&root, &artifacts::ArtifactInstallInput {
            kind: "schema".to_string(),
            payload: record("schema-source", vec![string("search")]),
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install schema");
        let identity = identity_with_schema(MODE_STRUCTURAL, &schema_artifact.artifact_ref, &shape, None);
        let installed = artifacts::install_artifact(&root, &artifacts::ArtifactInstallInput {
            kind: "schema-identity".to_string(),
            payload: identity.value.clone(),
            schema_refs: vec![identity.schema_ref.clone()],
            dependency_refs: vec![identity.schema_ref.clone()],
            effect_manifest_ref: None,
            policy_refs: identity.policy_refs.clone(),
            evidence_refs: identity.evidence_refs.clone(),
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install identity");
        assert_eq!(installed.decision, "pass");
        let matches = search_registry_by_fingerprint(&root, &identity.structural_fingerprint).expect("search");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].identity_ref, identity.identity_ref);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_fingerprint_and_compatibility_invariants(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let shape = record("shape", vec![
            string("record"),
            string(format!("profile-{salt}")),
            sequence(vec![
                record("shape", vec![string("field"), string("name"), record("shape", vec![string("string")])]),
                record("shape", vec![string("field"), string("age"), record("shape", vec![string("u64")])]),
            ]),
        ]);
        let (_, _, first_fp) = structural_fingerprint(&shape).expect("first fp");
        let (_, _, second_fp) = structural_fingerprint(&shape).expect("second fp");
        assert_eq!(first_fp, second_fp);
        let expected = identity(MODE_STRUCTURAL, &format!("expected-{salt}"), &shape, None);
        let actual = identity(MODE_STRUCTURAL, &format!("actual-{salt}"), &shape, None);
        let compatibility = compatibility_decision_value(&SchemaCompatibilityInput {
            expected: expected.clone(),
            actual: actual.clone(),
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref(&format!("policy-{salt}"))],
            evidence_refs: vec![test_ref(&format!("evidence-{salt}"))],
            deny_by_policy: false,
        })
        .expect("compatibility");
        let parsed = parse_schema_compatibility(&compatibility).expect("parse compatibility");
        assert_eq!(parsed.decision, DECISION_STRUCTURAL_MATCH);
        assert!(
            compatibility_admits_storage(&compatibility, &expected.schema_ref, &actual.schema_ref).expect("admits")
        );
        let denied = compatibility_decision_value(&SchemaCompatibilityInput {
            expected,
            actual,
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref(&format!("policy-{salt}"))],
            evidence_refs: vec![test_ref(&format!("evidence-{salt}"))],
            deny_by_policy: true,
        })
        .expect("denied compatibility");
        assert_eq!(parse_schema_compatibility(&denied).expect("parse denied").decision, DECISION_DENIED_BY_POLICY);
    }

    fn identity(mode: &str, label: &str, shape: &IOValue, brand_ref: Option<String>) -> SchemaIdentity {
        identity_with_schema(mode, &test_ref(&format!("schema-{label}")), shape, brand_ref)
    }

    fn identity_with_schema(
        mode: &str,
        schema_ref: &str,
        shape: &IOValue,
        brand_ref: Option<String>,
    ) -> SchemaIdentity {
        let value = schema_identity_value(&SchemaIdentityInput {
            mode: mode.to_string(),
            schema_ref: schema_ref.to_string(),
            shape: shape.clone(),
            brand_ref,
            metadata_refs: vec![test_ref("metadata")],
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("identity value");
        parse_schema_identity(&value).expect("identity")
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("schema-identity-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
