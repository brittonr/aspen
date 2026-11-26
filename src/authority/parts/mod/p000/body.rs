type IoValue = preserves::IOValue;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::Failure;
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
/// Receipt diagnostic stating that capability-scope admission makes no principal or key claim.
pub const CAPABILITY_SCOPE_NON_CLAIM: &str = "principal-and-key-currentness-not-claimed";

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

/// Caller-independent facts for [`admit_authority`] and [`gatekeeper_resolve_live_ref`]. The expected
/// principal, minimum epoch and current key set must come from the caller's own authenticated view,
/// never from the context being admitted.
#[derive(Debug, Clone, Copy)]
pub struct AuthorityAdmissionInput<'a> {
    pub context_value: &'a IoValue,
    pub expected_principal_ref: &'a str,
    pub requested_capability: &'a str,
    pub requested_scope: &'a str,
    pub logical_time: u64,
    pub minimum_epoch: u64,
    pub current_key_refs: &'a [String],
    pub revocation_values: &'a [IoValue],
}

#[derive(Clone, Copy)]
struct ScopeCheck<'a> {
    context: &'a Context,
    requested_capability: &'a str,
    requested_operation: &'a str,
    requested_scope: &'a str,
    logical_time: u64,
    revocations: &'a [Revocation],
}

struct AdmissionReceiptInput<'a> {
    operation: &'a str,
    decision: String,
    context: &'a Context,
    capability: &'a str,
    scope: &'a str,
    logical_time: u64,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityGrantCurrentness {
    pub decision: String,
    pub diagnostics: Vec<String>,
}
