use redb::ReadableDatabase;
use redb::ReadableTable;

type Database = redb::Database;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

const ADAPTER_KIND_STORAGE: &str = crate::effects::ADAPTER_KIND_STORAGE;
type EffectScope = crate::effects::EffectScope;

fn effect_handle_value(input: &crate::effects::EffectHandleInput) -> Result<IoValue> {
    crate::effects::effect_handle_value(input)
}

fn handler_binding_value(input: &crate::effects::HandlerBindingInput) -> Result<IoValue> {
    crate::effects::handler_binding_value(input)
}

fn validate_handle_for_request(
    handler_value: &IoValue,
    handle_value: &IoValue,
    request: &crate::effects::EffectHandleRequest<'_>,
) -> Result<crate::effects::EffectHandleValidation> {
    crate::effects::validate_handle_for_request(handler_value, handle_value, request)
}

fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn parse_canonical_bytes(bytes: &[u8]) -> Result<IoValue> {
    crate::preserves_rail::parse_canonical_bytes(bytes)
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

pub const INLINE_VALUE_LIMIT: usize = 4096;

const MAX_TYPED_STORAGE_RECEIPTS: usize = 100_000;
const _: () = assert!(MAX_TYPED_STORAGE_RECEIPTS > 0);

const INDEX_FILE: &str = "typed-storage.redb";
const INDEX_RECORDS: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("typed_storage_records_v1");
const INDEX_REFS: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("typed_storage_refs_v1");
const INDEX_INLINE_VALUES: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("typed_storage_inline_values_v1");
const INDEX_RECEIPTS: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("typed_storage_receipts_v1");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admission {
    pub actor_ref: String,
    pub capability_ref: String,
    pub policy_ref: String,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

impl Admission {
    pub fn local_fixture(label: &str) -> Self {
        Self {
            actor_ref: local_ref("storage-actor", label),
            capability_ref: local_ref("storage-capability", label),
            policy_ref: local_ref("storage-policy", label),
            resource_refs: vec![local_ref("storage-resource", label)],
            evidence_refs: vec![local_ref("storage-evidence", label)],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutInput {
    pub namespace: String,
    pub key: String,
    pub schema_ref: Option<String>,
    pub value: IoValue,
    pub producer_ref: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub admission: Admission,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Put {
    pub storage_ref: String,
    pub typed_ref_value: IoValue,
    pub schema_ref: String,
    pub value_ref: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Get {
    pub storage_ref: String,
    pub typed_ref: EntryRef,
    pub value: IoValue,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verify {
    pub storage_ref: String,
    pub typed_ref: EntryRef,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migrate {
    pub old_storage_ref: String,
    pub new_storage_ref: String,
    pub old_value_ref: String,
    pub new_value_ref: String,
    pub recipe_ref: String,
    pub typed_ref_value: IoValue,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRecipeInput {
    pub source_schema_ref: String,
    pub target_schema_ref: String,
    pub transformer_ref: String,
    pub transformer_kind: String,
    pub mode: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRecipe {
    pub recipe_ref: String,
    pub source_schema_ref: String,
    pub target_schema_ref: String,
    pub transformer_ref: String,
    pub transformer_kind: String,
    pub mode: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub storage_ref: Option<String>,
    pub namespace: Option<String>,
    pub key: Option<String>,
    pub schema_ref: Option<String>,
    pub value_ref: Option<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    Inline { length: u64 },
    ContentRef { manifest_ref: String, length: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryRef {
    pub storage_ref: String,
    pub namespace: String,
    pub key: String,
    pub schema_ref: String,
    pub value_ref: String,
    pub payload: Payload,
    pub producer_ref: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub revision: u64,
    pub actor_ref: String,
    pub capability_ref: String,
    pub effect_handle_ref: String,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EffectEvidence {
    manifest_ref: String,
    handler_binding_ref: String,
    handle_ref: String,
}

pub struct SchemaCompatibilityGetInput<'a> {
    pub root: &'a Path,
    pub namespace: &'a str,
    pub key: &'a str,
    pub expected_schema_ref: &'a str,
    pub schema_compatibility_value: &'a IoValue,
    pub admission: &'a Admission,
}

pub struct MigrationGetInput<'a> {
    pub root: &'a Path,
    pub namespace: &'a str,
    pub key: &'a str,
    pub expected_schema_ref: &'a str,
    pub migration_recipe_value: &'a IoValue,
    pub admission: &'a Admission,
}

struct GetValueInnerInput<'a> {
    root: &'a Path,
    namespace: &'a str,
    key: &'a str,
    expected_schema_ref: Option<&'a str>,
    admission: &'a Admission,
    migration_receipt_value: Option<&'a IoValue>,
    schema_compatibility_value: Option<&'a IoValue>,
}

struct EffectEvidenceInput<'a> {
    operation: &'a str,
    namespace: &'a str,
    key: &'a str,
    schema_ref: &'a str,
    producer_ref: &'a str,
    admission: &'a Admission,
    remote_use: bool,
}

struct ReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    storage_ref: Option<&'a str>,
    namespace: Option<&'a str>,
    key: Option<&'a str>,
    schema_ref: Option<&'a str>,
    value_ref: Option<&'a str>,
    effect: &'a EffectEvidence,
    checks: Vec<(&'a str, &'a str)>,
    details: Vec<IoValue>,
}

struct DenialReceiptValueInput<'a> {
    operation: &'a str,
    storage_ref: Option<&'a str>,
    namespace: Option<&'a str>,
    key: Option<&'a str>,
    schema_ref: Option<&'a str>,
    value_ref: Option<&'a str>,
    reason: String,
    checks: Vec<(&'a str, &'a str)>,
    details: Vec<IoValue>,
}

struct PayloadParts {
    value: IoValue,
    details: Vec<IoValue>,
}

struct PersistInput<'a> {
    root: &'a Path,
    storage_key: &'a str,
    storage_ref: &'a str,
    typed_ref_value: &'a IoValue,
    value_ref: &'a str,
    value_bytes: &'a [u8],
    receipt_value: &'a IoValue,
}

struct EntryInput<'a> {
    input: &'a PutInput,
    schema_ref: &'a str,
    value_ref: &'a str,
    payload: &'a IoValue,
    revision: u64,
    effect_handle_ref: &'a str,
}
