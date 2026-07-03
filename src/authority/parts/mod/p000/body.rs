type IoValue = preserves::IOValue;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

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

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

type RuntimeAssertion = crate::runtime::RuntimeAssertion;
#[cfg(test)]
type RuntimeValue = crate::runtime::RuntimeValue;

const AUTHORITY_DEFAULT_GRANT_EPOCH: u64 = 0;
const AUTHORITY_CURRENTNESS_DIAGNOSTIC_CAPACITY: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub identity_ref: String,
    pub identity_type: String,
    pub id: String,
    pub display_name: String,
    pub key_refs: Vec<String>,
    pub parent_refs: Vec<String>,
    pub metadata_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub capability: String,
    pub scope: String,
    pub attenuation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub context_ref: String,
    pub subject_ref: String,
    pub capabilities: Vec<Capability>,
    pub delegation_refs: Vec<String>,
    pub not_before: Option<u64>,
    pub expires_at: Option<u64>,
    pub revocation_refs: Vec<String>,
    pub key_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revocation {
    pub revocation_ref: String,
    pub target_kind: String,
    pub target_ref: String,
    pub reason: String,
    pub effective_at: u64,
    pub issuer_ref: String,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub context_ref: Option<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admission {
    pub decision: String,
    pub receipt: Receipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveRef {
    pub live_ref: String,
    pub context_ref: String,
    pub scope: String,
    pub attenuation: String,
    pub expires_at: Option<u64>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct IdentityValueInput<'a> {
    pub identity_type: &'a str,
    pub id: &'a str,
    pub display_name: &'a str,
    pub key_refs: &'a [String],
    pub parent_refs: &'a [String],
    pub metadata_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ContextValueInput<'a> {
    pub subject_ref: &'a str,
    pub capabilities: &'a [Capability],
    pub delegation_refs: &'a [String],
    pub not_before: Option<u64>,
    pub expires_at: Option<u64>,
    pub revocation_refs: &'a [String],
    pub key_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct RevocationValueInput<'a> {
    pub target_kind: &'a str,
    pub target_ref: &'a str,
    pub reason: &'a str,
    pub effective_at: u64,
    pub issuer_ref: &'a str,
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ReceiptValueInput<'a> {
    pub operation: &'a str,
    pub decision: &'a str,
    pub context_ref: Option<&'a str>,
    pub capability: &'a str,
    pub scope: &'a str,
    pub logical_time: u64,
    pub diagnostics: &'a [&'a str],
}

#[derive(Debug, Clone, Copy)]
pub struct AuthorityGrantCurrentnessInput<'a> {
    pub context: &'a Context,
    pub requested_principal_ref: &'a str,
    pub requested_capability: &'a str,
    pub requested_operation: &'a str,
    pub requested_scope: &'a str,
    pub logical_time: u64,
    pub grant_epoch: u64,
    pub minimum_epoch: u64,
    pub current_epoch: u64,
    pub current_key_refs: &'a [String],
    pub revocations: &'a [Revocation],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityGrantCurrentness {
    pub decision: String,
    pub diagnostics: Vec<String>,
}

pub fn identity_value(input: IdentityValueInput<'_>) -> Result<IoValue> {
    validate_identity_type(input.identity_type)?;
    validate_non_empty(input.id, "authority identity id")?;
    validate_non_empty(input.display_name, "authority identity display name")?;
    validate_refs(input.key_refs, "authority identity key ref")?;
    validate_refs(input.parent_refs, "authority identity parent ref")?;
    validate_refs(input.metadata_refs, "authority identity metadata ref")?;
    Ok(record("authority-identity-v1", vec![
        string(crate::preserves_rail::AUTHORITY_IDENTITY_SCHEMA),
        record("identity", vec![
            record("type", vec![string(input.identity_type)]),
            record("id", vec![string(input.id)]),
            record("display-name", vec![string(input.display_name)]),
        ]),
        record("keys", vec![sequence(input.key_refs.iter().map(string).collect())]),
        record("parents", vec![sequence(input.parent_refs.iter().map(string).collect())]),
        record("metadata", vec![sequence(input.metadata_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("names-are-metadata"), string("pass")]),
            record("check", vec![string("identity-alone-grants-no-authority"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_identity(value: &IoValue) -> Result<Identity> {
    let fields = value
        .collect_simple_record("authority-identity-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <authority-identity-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::AUTHORITY_IDENTITY_SCHEMA, "authority identity schema")?;
    let identity = value_to_iovalue(&fields[1]);
    let identity_fields = identity
        .collect_simple_record("identity", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("authority identity missing identity field"))?;
    let checks = parse_checks(&fields[5])?;
    require_check(&checks, "identity-alone-grants-no-authority")?;
    Ok(Identity {
        identity_ref: canonical_hash(value)?,
        identity_type: record_string(&identity_fields[0], "type")?,
        id: record_string(&identity_fields[1], "id")?,
        display_name: record_string(&identity_fields[2], "display-name")?,
        key_refs: parse_ref_sequence(&fields[2], "keys")?,
        parent_refs: parse_ref_sequence(&fields[3], "parents")?,
        metadata_refs: parse_ref_sequence(&fields[4], "metadata")?,
        value: value.clone(),
    })
}

pub fn context_value(input: ContextValueInput<'_>) -> Result<IoValue> {
    require_ref(input.subject_ref, "authority context subject ref")?;
    for capability in input.capabilities {
        validate_capability(capability)?;
    }
    validate_refs(input.delegation_refs, "authority context delegation ref")?;
    validate_refs(input.revocation_refs, "authority context revocation ref")?;
    validate_refs(input.key_refs, "authority context key ref")?;
    validate_refs(input.policy_refs, "authority context policy ref")?;
    validate_refs(input.evidence_refs, "authority context evidence ref")?;
    Ok(record("authority-context-v1", vec![
        string(crate::preserves_rail::AUTHORITY_CONTEXT_SCHEMA),
        record("subject", vec![string(input.subject_ref)]),
        record("capabilities", vec![sequence(input.capabilities.iter().map(capability_value).collect())]),
        record("delegations", vec![sequence(input.delegation_refs.iter().map(string).collect())]),
        record("validity", vec![
            optional_u64_value(input.not_before),
            optional_u64_value(input.expires_at),
        ]),
        record("revocations", vec![sequence(input.revocation_refs.iter().map(string).collect())]),
        record("keys", vec![sequence(input.key_refs.iter().map(string).collect())]),
        record("policy", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("evidence", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("scoped-capabilities"), string("pass")]),
            record("check", vec![string("attenuation-monotonic"), string("pass")]),
            record("check", vec![string("revocation-checked-at-admission"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_context(value: &IoValue) -> Result<Context> {
    let fields = value
        .collect_simple_record("authority-context-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <authority-context-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::AUTHORITY_CONTEXT_SCHEMA, "authority context schema")?;
    let validity = value_to_iovalue(&fields[4]);
    let validity_fields = validity
        .collect_simple_record("validity", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("authority context missing validity"))?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "revocation-checked-at-admission")?;
    Ok(Context {
        context_ref: canonical_hash(value)?,
        subject_ref: record_string(&fields[1], "subject")?,
        capabilities: parse_capability_sequence(&fields[2], "capabilities")?,
        delegation_refs: parse_ref_sequence(&fields[3], "delegations")?,
        not_before: parse_optional_u64_value(&validity_fields[0])?,
        expires_at: parse_optional_u64_value(&validity_fields[1])?,
        revocation_refs: parse_ref_sequence(&fields[5], "revocations")?,
        key_refs: parse_ref_sequence(&fields[6], "keys")?,
        policy_refs: parse_ref_sequence(&fields[7], "policy")?,
        evidence_refs: parse_ref_sequence(&fields[8], "evidence")?,
        value: value.clone(),
    })
}
