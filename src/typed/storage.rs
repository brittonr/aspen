use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use preserves::CompoundClass;
use preserves::IOValue;
use preserves::Record;
use preserves::Value;
use preserves::ValueClass;
use redb::Database;
use redb::ReadableDatabase;
use redb::ReadableTable;
use redb::TableDefinition;

use crate::chunk_store;
use crate::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE;
use crate::effects::ADAPTER_KIND_STORAGE;
use crate::effects::EffectHandleInput;
use crate::effects::EffectHandleRequest;
use crate::effects::EffectScope;
use crate::effects::HandlerBindingInput;
use crate::effects::TRANSFER_LOCAL_ONLY;
use crate::effects::effect_handle_value;
use crate::effects::handler_binding_value;
use crate::effects::validate_handle_for_request;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::TYPED_STORAGE_EFFECT_MANIFEST_SCHEMA;
use crate::preserves_rail::TYPED_STORAGE_MIGRATION_RECIPE_SCHEMA;
use crate::preserves_rail::TYPED_STORAGE_RECEIPT_SCHEMA;
use crate::preserves_rail::TYPED_STORAGE_REF_SCHEMA;
use crate::preserves_rail::TYPED_STORAGE_SCHEMA_ARTIFACT_SCHEMA;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::content_ref_from_bytes;
use crate::preserves_rail::parse_canonical_bytes;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;
use crate::schema_identity;

pub const INLINE_VALUE_LIMIT: usize = 4096;

const MAX_TYPED_STORAGE_RECEIPTS: usize = 100_000;
const _: () = assert!(MAX_TYPED_STORAGE_RECEIPTS > 0);

const INDEX_FILE: &str = "typed-storage.redb";
const INDEX_RECORDS: TableDefinition<&str, &[u8]> = TableDefinition::new("typed_storage_records_v1");
const INDEX_REFS: TableDefinition<&str, &[u8]> = TableDefinition::new("typed_storage_refs_v1");
const INDEX_INLINE_VALUES: TableDefinition<&str, &[u8]> = TableDefinition::new("typed_storage_inline_values_v1");
const INDEX_RECEIPTS: TableDefinition<&str, &[u8]> = TableDefinition::new("typed_storage_receipts_v1");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedStorageAdmission {
    pub actor_ref: String,
    pub capability_ref: String,
    pub policy_ref: String,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

impl TypedStorageAdmission {
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
pub struct TypedStoragePutInput {
    pub namespace: String,
    pub key: String,
    pub schema_ref: Option<String>,
    pub value: IOValue,
    pub producer_ref: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub admission: TypedStorageAdmission,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedStoragePut {
    pub storage_ref: String,
    pub typed_ref_value: IOValue,
    pub schema_ref: String,
    pub value_ref: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedStorageGet {
    pub storage_ref: String,
    pub typed_ref: TypedStorageRef,
    pub value: IOValue,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedStorageVerify {
    pub storage_ref: String,
    pub typed_ref: TypedStorageRef,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedStorageMigrate {
    pub old_storage_ref: String,
    pub new_storage_ref: String,
    pub old_value_ref: String,
    pub new_value_ref: String,
    pub recipe_ref: String,
    pub typed_ref_value: IOValue,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageMigrationRecipeInput {
    pub source_schema_ref: String,
    pub target_schema_ref: String,
    pub transformer_ref: String,
    pub transformer_kind: String,
    pub mode: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageMigrationRecipe {
    pub recipe_ref: String,
    pub source_schema_ref: String,
    pub target_schema_ref: String,
    pub transformer_ref: String,
    pub transformer_kind: String,
    pub mode: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedStorageReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub storage_ref: Option<String>,
    pub namespace: Option<String>,
    pub key: Option<String>,
    pub schema_ref: Option<String>,
    pub value_ref: Option<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedStoragePayload {
    Inline { length: u64 },
    ContentRef { manifest_ref: String, length: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedStorageRef {
    pub storage_ref: String,
    pub namespace: String,
    pub key: String,
    pub schema_ref: String,
    pub value_ref: String,
    pub payload: TypedStoragePayload,
    pub producer_ref: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub revision: u64,
    pub actor_ref: String,
    pub capability_ref: String,
    pub effect_handle_ref: String,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StorageEffectEvidence {
    manifest_ref: String,
    handler_binding_ref: String,
    handle_ref: String,
}

pub struct SchemaCompatibilityGetInput<'a> {
    pub root: &'a Path,
    pub namespace: &'a str,
    pub key: &'a str,
    pub expected_schema_ref: &'a str,
    pub schema_compatibility_value: &'a IOValue,
    pub admission: &'a TypedStorageAdmission,
}

pub struct MigrationGetInput<'a> {
    pub root: &'a Path,
    pub namespace: &'a str,
    pub key: &'a str,
    pub expected_schema_ref: &'a str,
    pub migration_recipe_value: &'a IOValue,
    pub admission: &'a TypedStorageAdmission,
}

struct GetValueInnerInput<'a> {
    root: &'a Path,
    namespace: &'a str,
    key: &'a str,
    expected_schema_ref: Option<&'a str>,
    admission: &'a TypedStorageAdmission,
    migration_receipt_value: Option<&'a IOValue>,
    schema_compatibility_value: Option<&'a IOValue>,
}

struct StorageEffectEvidenceInput<'a> {
    operation: &'a str,
    namespace: &'a str,
    key: &'a str,
    schema_ref: &'a str,
    producer_ref: &'a str,
    admission: &'a TypedStorageAdmission,
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
    effect: &'a StorageEffectEvidence,
    checks: Vec<(&'a str, &'a str)>,
    details: Vec<IOValue>,
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
    details: Vec<IOValue>,
}

pub fn put_value(root: &Path, input: &TypedStoragePutInput) -> Result<TypedStoragePut> {
    ensure_dirs(root)?;
    validate_namespace_key(&input.namespace, &input.key)?;
    require_ref(&input.producer_ref, "typed storage producer ref")?;
    validate_refs(&input.policy_refs, "typed storage policy ref")?;
    validate_refs(&input.evidence_refs, "typed storage evidence ref")?;
    validate_admission(&input.admission)?;

    let inferred_schema_ref = inferred_schema_ref(&input.value)?;
    let schema_ref = input.schema_ref.clone().unwrap_or_else(|| inferred_schema_ref.clone());
    if schema_ref != inferred_schema_ref {
        let receipt_value = denial_receipt_value(DenialReceiptValueInput {
            operation: "put",
            storage_ref: None,
            namespace: Some(&input.namespace),
            key: Some(&input.key),
            schema_ref: Some(&schema_ref),
            value_ref: None,
            reason: "declared schema ref does not match inferred Preserves value schema".to_string(),
            checks: vec![
                ("schema-binding", "fail"),
                ("no-raw-memory", "pass"),
                ("denial-receipt", "pass"),
            ],
            details: Vec::new(),
        });
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(
            "typed storage write rejected: declared schema ref does not match inferred value schema",
        ));
    }
    let effect = storage_effect_evidence(StorageEffectEvidenceInput {
        operation: "put",
        namespace: &input.namespace,
        key: &input.key,
        schema_ref: &schema_ref,
        producer_ref: &input.producer_ref,
        admission: &input.admission,
        remote_use: false,
    })?;
    let value_bytes = canonical_bytes(&input.value)?;
    let value_ref = canonical_hash(&input.value)?;
    let (payload, payload_details) = if value_bytes.len() <= INLINE_VALUE_LIMIT {
        (record("inline", vec![u64_value(value_bytes.len() as u64)]), vec![record("payload", vec![
            string("inline"),
            u64_value(value_bytes.len() as u64),
        ])])
    } else {
        let put = chunk_store::put_bytes(
            &chunk_root(root),
            "typed-storage-value",
            &value_bytes,
            DEFAULT_FIXED_V1_CHUNK_SIZE,
        )?;
        (record("content-ref", vec![string(&put.manifest_ref), u64_value(value_bytes.len() as u64)]), vec![
            record("payload", vec![string("content-ref"), string(&put.manifest_ref)]),
            record("chunk-store-receipt", vec![string(canonical_hash(&put.receipt_value)?)]),
        ])
    };
    let storage_key = storage_key_ref(&input.namespace, &input.key)?;
    let revision = next_revision(root, &storage_key)?;
    let typed_ref_value = typed_ref_value(TypedRefValueInput {
        namespace: &input.namespace,
        key: &input.key,
        schema_ref: &schema_ref,
        value_ref: &value_ref,
        payload: &payload,
        producer_ref: &input.producer_ref,
        policy_refs: &input.policy_refs,
        evidence_refs: &input.evidence_refs,
        revision,
        actor_ref: &input.admission.actor_ref,
        capability_ref: &input.admission.capability_ref,
        effect_handle_ref: &effect.handle_ref,
    });
    let storage_ref = canonical_hash(&typed_ref_value)?;
    let receipt_value = receipt_value(ReceiptValueInput {
        operation: "put",
        decision: "pass",
        storage_ref: Some(&storage_ref),
        namespace: Some(&input.namespace),
        key: Some(&input.key),
        schema_ref: Some(&schema_ref),
        value_ref: Some(&value_ref),
        effect: &effect,
        checks: vec![
            ("effect-manifest", "pass"),
            ("storage-effect-handle", "pass"),
            ("write-admission", "pass"),
            ("schema-binding", "pass"),
            ("canonical-value", "pass"),
            ("redb-adapter", "pass"),
            ("cairn-receipt", "pass"),
            ("no-raw-memory", "pass"),
        ],
        details: payload_details,
    });
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        let typed_ref_bytes = canonical_bytes(&typed_ref_value)?;
        let mut records = write_txn.open_table(INDEX_RECORDS).map_err(index_error)?;
        records.insert(storage_key.as_str(), typed_ref_bytes.as_slice()).map_err(index_error)?;
        let mut refs = write_txn.open_table(INDEX_REFS).map_err(index_error)?;
        refs.insert(storage_ref.as_str(), typed_ref_bytes.as_slice()).map_err(index_error)?;
        if value_bytes.len() <= INLINE_VALUE_LIMIT {
            let mut inline_values = write_txn.open_table(INDEX_INLINE_VALUES).map_err(index_error)?;
            inline_values.insert(value_ref.as_str(), value_bytes.as_slice()).map_err(index_error)?;
        }
        store_receipt_in_tx(&write_txn, &receipt_value)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(TypedStoragePut {
        storage_ref,
        typed_ref_value,
        schema_ref,
        value_ref,
        receipt_value,
    })
}

pub fn get_value(
    root: &Path,
    namespace: &str,
    key: &str,
    expected_schema_ref: Option<&str>,
    admission: &TypedStorageAdmission,
) -> Result<TypedStorageGet> {
    get_value_inner(GetValueInnerInput {
        root,
        namespace,
        key,
        expected_schema_ref,
        admission,
        migration_receipt_value: None,
        schema_compatibility_value: None,
    })
}

pub fn get_value_with_schema_compatibility(input: SchemaCompatibilityGetInput<'_>) -> Result<TypedStorageGet> {
    get_value_inner(GetValueInnerInput {
        root: input.root,
        namespace: input.namespace,
        key: input.key,
        expected_schema_ref: Some(input.expected_schema_ref),
        admission: input.admission,
        migration_receipt_value: None,
        schema_compatibility_value: Some(input.schema_compatibility_value),
    })
}

pub fn get_value_with_migration(input: MigrationGetInput<'_>) -> Result<TypedStorageGet> {
    match get_value_inner(GetValueInnerInput {
        root: input.root,
        namespace: input.namespace,
        key: input.key,
        expected_schema_ref: Some(input.expected_schema_ref),
        admission: input.admission,
        migration_receipt_value: None,
        schema_compatibility_value: None,
    }) {
        Ok(value) => Ok(value),
        Err(first_error) => {
            let recipe = parse_migration_recipe_value(input.migration_recipe_value)?;
            if recipe.target_schema_ref != input.expected_schema_ref {
                return Err(MoltenError::invalid_harness(
                    "typed storage lazy migration rejected: recipe target schema does not match expected schema ref",
                ));
            }
            if !matches!(recipe.mode.as_str(), "lazy-on-read" | "explicit") {
                return Err(MoltenError::invalid_harness(format!(
                    "typed storage lazy migration rejected: recipe mode {} cannot run on read",
                    recipe.mode
                )));
            }
            let migrated =
                migrate_value(input.root, input.namespace, input.key, input.migration_recipe_value, input.admission)
                    .map_err(|migration_error| {
                        MoltenError::invalid_harness(format!(
                            "typed storage lazy migration failed after load miss {first_error}: {migration_error}"
                        ))
                    })?;
            get_value_inner(GetValueInnerInput {
                root: input.root,
                namespace: input.namespace,
                key: input.key,
                expected_schema_ref: Some(input.expected_schema_ref),
                admission: input.admission,
                migration_receipt_value: Some(&migrated.receipt_value),
                schema_compatibility_value: None,
            })
        }
    }
}

fn get_value_inner(input: GetValueInnerInput<'_>) -> Result<TypedStorageGet> {
    ensure_dirs(input.root)?;
    validate_namespace_key(input.namespace, input.key)?;
    validate_admission(input.admission)?;
    let storage_key = storage_key_ref(input.namespace, input.key)?;
    let typed_ref_value = {
        let db = ensure_index_tables(input.root)?;
        let read_txn = db.begin_read().map_err(index_error)?;
        let records = read_txn.open_table(INDEX_RECORDS).map_err(index_error)?;
        let Some(bytes) = records.get(storage_key.as_str()).map_err(index_error)? else {
            drop(records);
            drop(read_txn);
            drop(db);
            let receipt_value = denial_receipt_value(DenialReceiptValueInput {
                operation: "get",
                storage_ref: None,
                namespace: Some(input.namespace),
                key: Some(input.key),
                schema_ref: input.expected_schema_ref,
                value_ref: None,
                reason: "typed storage record not found".to_string(),
                checks: vec![("record-found", "fail"), ("denial-receipt", "pass")],
                details: Vec::new(),
            });
            store_receipt(input.root, &receipt_value)?;
            return Err(MoltenError::invalid_harness("typed storage get rejected: record not found"));
        };
        parse_canonical_bytes(bytes.value())?
    };
    let typed_ref = parse_typed_ref_value(&typed_ref_value)?;
    let has_schema_compatibility_admission = if let Some(expected_schema_ref) = input.expected_schema_ref {
        if typed_ref.schema_ref == expected_schema_ref {
            true
        } else if let Some(schema_compatibility_value) = input.schema_compatibility_value {
            schema_identity::compatibility_admits_storage(
                schema_compatibility_value,
                expected_schema_ref,
                &typed_ref.schema_ref,
            )?
        } else {
            false
        }
    } else {
        true
    };
    if let Some(expected_schema_ref) = input.expected_schema_ref
        && typed_ref.schema_ref != expected_schema_ref
        && !has_schema_compatibility_admission
    {
        let receipt_value = denial_receipt_value(DenialReceiptValueInput {
            operation: "get",
            storage_ref: Some(&typed_ref.storage_ref),
            namespace: Some(input.namespace),
            key: Some(input.key),
            schema_ref: Some(expected_schema_ref),
            value_ref: Some(&typed_ref.value_ref),
            reason: "expected schema ref does not match stored schema ref".to_string(),
            checks: vec![("schema-compatibility", "fail"), ("denial-receipt", "pass")],
            details: Vec::new(),
        });
        store_receipt(input.root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(
            "typed storage get rejected: expected schema ref does not match stored schema ref",
        ));
    }
    let effect = storage_effect_evidence(StorageEffectEvidenceInput {
        operation: "get",
        namespace: input.namespace,
        key: input.key,
        schema_ref: &typed_ref.schema_ref,
        producer_ref: &typed_ref.producer_ref,
        admission: input.admission,
        remote_use: false,
    })?;
    let value_bytes = read_payload_bytes(input.root, &typed_ref)?;
    let value = parse_canonical_bytes(&value_bytes)?;
    let actual_value_ref = canonical_hash(&value)?;
    if actual_value_ref != typed_ref.value_ref {
        let receipt_value = denial_receipt_value(DenialReceiptValueInput {
            operation: "get",
            storage_ref: Some(&typed_ref.storage_ref),
            namespace: Some(input.namespace),
            key: Some(input.key),
            schema_ref: Some(&typed_ref.schema_ref),
            value_ref: Some(&typed_ref.value_ref),
            reason: "stored value hash does not match typed ref".to_string(),
            checks: vec![("content-integrity", "fail"), ("denial-receipt", "pass")],
            details: Vec::new(),
        });
        store_receipt(input.root, &receipt_value)?;
        return Err(MoltenError::invalid_harness("typed storage content integrity check failed"));
    }
    let receipt_value = receipt_value(ReceiptValueInput {
        operation: "get",
        decision: "pass",
        storage_ref: Some(&typed_ref.storage_ref),
        namespace: Some(input.namespace),
        key: Some(input.key),
        schema_ref: Some(&typed_ref.schema_ref),
        value_ref: Some(&typed_ref.value_ref),
        effect: &effect,
        checks: vec![
            ("effect-manifest", "pass"),
            ("storage-effect-handle", "pass"),
            ("load-admission", "pass"),
            ("schema-compatibility", "pass"),
            ("content-integrity", "pass"),
            ("receipt-validation", "pass"),
        ],
        details: {
            let mut details = vec![record("revision", vec![u64_value(typed_ref.revision)])];
            if let Some(migration_receipt_value) = input.migration_receipt_value {
                details.push(record("migration-receipt", vec![string(canonical_hash(migration_receipt_value)?)]));
                details.push(record("migration-mode", vec![string("lazy-on-read")]));
            }
            if let Some(schema_compatibility_value) = input.schema_compatibility_value {
                details.push(record("schema-compatibility", vec![string(canonical_hash(schema_compatibility_value)?)]));
                details.push(record("schema-compatibility-value", vec![schema_compatibility_value.clone()]));
            }
            details
        },
    });
    store_receipt(input.root, &receipt_value)?;
    Ok(TypedStorageGet {
        storage_ref: typed_ref.storage_ref.clone(),
        typed_ref,
        value,
        receipt_value,
    })
}

pub fn verify_ref(root: &Path, storage_ref: &str, expected_schema_ref: Option<&str>) -> Result<TypedStorageVerify> {
    ensure_dirs(root)?;
    require_ref(storage_ref, "typed storage ref")?;
    let typed_ref_value = read_typed_ref(root, storage_ref)?;
    let typed_ref = parse_typed_ref_value(&typed_ref_value)?;
    if let Some(expected_schema_ref) = expected_schema_ref
        && typed_ref.schema_ref != expected_schema_ref
    {
        let receipt_value = denial_receipt_value(DenialReceiptValueInput {
            operation: "verify",
            storage_ref: Some(storage_ref),
            namespace: Some(&typed_ref.namespace),
            key: Some(&typed_ref.key),
            schema_ref: Some(expected_schema_ref),
            value_ref: Some(&typed_ref.value_ref),
            reason: "verify expected schema ref does not match stored schema ref".to_string(),
            checks: vec![("schema-compatibility", "fail"), ("denial-receipt", "pass")],
            details: Vec::new(),
        });
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(
            "typed storage verify rejected: expected schema ref does not match stored schema ref",
        ));
    }
    let value_bytes = read_payload_bytes(root, &typed_ref)?;
    let value = parse_canonical_bytes(&value_bytes)?;
    let actual_value_ref = canonical_hash(&value)?;
    if actual_value_ref != typed_ref.value_ref {
        let receipt_value = denial_receipt_value(DenialReceiptValueInput {
            operation: "verify",
            storage_ref: Some(storage_ref),
            namespace: Some(&typed_ref.namespace),
            key: Some(&typed_ref.key),
            schema_ref: Some(&typed_ref.schema_ref),
            value_ref: Some(&typed_ref.value_ref),
            reason: "verify content hash mismatch".to_string(),
            checks: vec![("content-integrity", "fail"), ("denial-receipt", "pass")],
            details: Vec::new(),
        });
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness("typed storage verify content integrity check failed"));
    }
    let effect = StorageEffectEvidence {
        manifest_ref: typed_ref.effect_handle_ref.clone(),
        handler_binding_ref: typed_ref.effect_handle_ref.clone(),
        handle_ref: typed_ref.effect_handle_ref.clone(),
    };
    let receipt_value = receipt_value(ReceiptValueInput {
        operation: "verify",
        decision: "pass",
        storage_ref: Some(storage_ref),
        namespace: Some(&typed_ref.namespace),
        key: Some(&typed_ref.key),
        schema_ref: Some(&typed_ref.schema_ref),
        value_ref: Some(&typed_ref.value_ref),
        effect: &effect,
        checks: vec![
            ("typed-ref-found", "pass"),
            ("content-integrity", "pass"),
            ("schema-binding", "pass"),
            ("receipt-validation", "pass"),
        ],
        details: Vec::new(),
    });
    store_receipt(root, &receipt_value)?;
    Ok(TypedStorageVerify {
        storage_ref: typed_ref.storage_ref.clone(),
        typed_ref,
        receipt_value,
    })
}

pub fn migrate_value(
    root: &Path,
    namespace: &str,
    key: &str,
    migration_recipe_value: &IOValue,
    admission: &TypedStorageAdmission,
) -> Result<TypedStorageMigrate> {
    ensure_dirs(root)?;
    validate_namespace_key(namespace, key)?;
    validate_admission(admission)?;
    let recipe = parse_migration_recipe_value(migration_recipe_value)?;
    let recipe_ref = recipe.recipe_ref.clone();
    let storage_key = storage_key_ref(namespace, key)?;
    let old_typed_ref_value = {
        let db = ensure_index_tables(root)?;
        let read_txn = db.begin_read().map_err(index_error)?;
        let records = read_txn.open_table(INDEX_RECORDS).map_err(index_error)?;
        let Some(bytes) = records.get(storage_key.as_str()).map_err(index_error)? else {
            drop(records);
            drop(read_txn);
            drop(db);
            let receipt_value = denial_receipt_value(DenialReceiptValueInput {
                operation: "migrate",
                storage_ref: None,
                namespace: Some(namespace),
                key: Some(key),
                schema_ref: Some(&recipe.target_schema_ref),
                value_ref: None,
                reason: "typed storage migration source record not found".to_string(),
                checks: vec![("record-found", "fail"), ("denial-receipt", "pass")],
                details: vec![record("recipe", vec![string(&recipe_ref)])],
            });
            store_receipt(root, &receipt_value)?;
            return Err(MoltenError::invalid_harness("typed storage migration rejected: source record not found"));
        };
        parse_canonical_bytes(bytes.value())?
    };
    let old_typed_ref = parse_typed_ref_value(&old_typed_ref_value)?;
    if old_typed_ref.schema_ref != recipe.source_schema_ref {
        let receipt_value = denial_receipt_value(DenialReceiptValueInput {
            operation: "migrate",
            storage_ref: Some(&old_typed_ref.storage_ref),
            namespace: Some(namespace),
            key: Some(key),
            schema_ref: Some(&recipe.target_schema_ref),
            value_ref: Some(&old_typed_ref.value_ref),
            reason: "typed storage migration source schema does not match recipe".to_string(),
            checks: vec![("source-schema-binding", "fail"), ("denial-receipt", "pass")],
            details: vec![record("recipe", vec![string(&recipe_ref)])],
        });
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(
            "typed storage migration rejected: source schema does not match recipe",
        ));
    }
    let old_value_bytes = read_payload_bytes(root, &old_typed_ref)?;
    let old_value = parse_canonical_bytes(&old_value_bytes)?;
    if canonical_hash(&old_value)? != old_typed_ref.value_ref {
        let receipt_value = denial_receipt_value(DenialReceiptValueInput {
            operation: "migrate",
            storage_ref: Some(&old_typed_ref.storage_ref),
            namespace: Some(namespace),
            key: Some(key),
            schema_ref: Some(&recipe.target_schema_ref),
            value_ref: Some(&old_typed_ref.value_ref),
            reason: "typed storage migration source value hash mismatch".to_string(),
            checks: vec![("source-content-integrity", "fail"), ("denial-receipt", "pass")],
            details: vec![record("recipe", vec![string(&recipe_ref)])],
        });
        store_receipt(root, &receipt_value)?;
        return Err(MoltenError::invalid_harness("typed storage migration source content integrity failed"));
    }
    let new_value = apply_migration_transform(&recipe, &old_value)?;
    let new_value_bytes = canonical_bytes(&new_value)?;
    let new_value_ref = canonical_hash(&new_value)?;
    let (payload, payload_details) = store_payload(root, &new_value_bytes)?;
    let effect = storage_effect_evidence(StorageEffectEvidenceInput {
        operation: "migrate",
        namespace,
        key,
        schema_ref: &recipe.target_schema_ref,
        producer_ref: &recipe.transformer_ref,
        admission,
        remote_use: false,
    })?;
    let revision = next_revision(root, &storage_key)?;
    let mut policy_refs = old_typed_ref.policy_refs.clone();
    policy_refs.extend(recipe.policy_refs.clone());
    policy_refs.sort();
    policy_refs.dedup();
    let mut evidence_refs = old_typed_ref.evidence_refs.clone();
    evidence_refs.push(recipe_ref.clone());
    evidence_refs.extend(recipe.evidence_refs.clone());
    evidence_refs.sort();
    evidence_refs.dedup();
    let typed_ref_value = typed_ref_value(TypedRefValueInput {
        namespace,
        key,
        schema_ref: &recipe.target_schema_ref,
        value_ref: &new_value_ref,
        payload: &payload,
        producer_ref: &recipe.transformer_ref,
        policy_refs: &policy_refs,
        evidence_refs: &evidence_refs,
        revision,
        actor_ref: &admission.actor_ref,
        capability_ref: &admission.capability_ref,
        effect_handle_ref: &effect.handle_ref,
    });
    let new_storage_ref = canonical_hash(&typed_ref_value)?;
    let mut details = vec![
        record("recipe", vec![string(&recipe_ref)]),
        record("mode", vec![string(&recipe.mode)]),
        record("transformer", vec![string(&recipe.transformer_ref), string(&recipe.transformer_kind)]),
        record("old-storage-ref", vec![string(&old_typed_ref.storage_ref)]),
        record("new-storage-ref", vec![string(&new_storage_ref)]),
        record("old-value-ref", vec![string(&old_typed_ref.value_ref)]),
        record("new-value-ref", vec![string(&new_value_ref)]),
        record("source-schema-ref", vec![string(&recipe.source_schema_ref)]),
        record("target-schema-ref", vec![string(&recipe.target_schema_ref)]),
    ];
    details.extend(payload_details);
    let receipt_value = receipt_value(ReceiptValueInput {
        operation: "migrate",
        decision: "pass",
        storage_ref: Some(&new_storage_ref),
        namespace: Some(namespace),
        key: Some(key),
        schema_ref: Some(&recipe.target_schema_ref),
        value_ref: Some(&new_value_ref),
        effect: &effect,
        checks: vec![
            ("effect-manifest", "pass"),
            ("storage-effect-handle", "pass"),
            ("migration-admission", "pass"),
            ("source-schema-binding", "pass"),
            ("target-schema-binding", "pass"),
            ("transformer-binding", "pass"),
            ("migration-trace", "pass"),
            ("original-value-hash", "pass"),
            ("result-value-hash", "pass"),
            ("redb-adapter", "pass"),
            ("cairn-receipt", "pass"),
        ],
        details,
    });
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        let typed_ref_bytes = canonical_bytes(&typed_ref_value)?;
        let mut records = write_txn.open_table(INDEX_RECORDS).map_err(index_error)?;
        records.insert(storage_key.as_str(), typed_ref_bytes.as_slice()).map_err(index_error)?;
        let mut refs = write_txn.open_table(INDEX_REFS).map_err(index_error)?;
        refs.insert(new_storage_ref.as_str(), typed_ref_bytes.as_slice()).map_err(index_error)?;
        if new_value_bytes.len() <= INLINE_VALUE_LIMIT {
            let mut inline_values = write_txn.open_table(INDEX_INLINE_VALUES).map_err(index_error)?;
            inline_values.insert(new_value_ref.as_str(), new_value_bytes.as_slice()).map_err(index_error)?;
        }
        store_receipt_in_tx(&write_txn, &receipt_value)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(TypedStorageMigrate {
        old_storage_ref: old_typed_ref.storage_ref,
        new_storage_ref,
        old_value_ref: old_typed_ref.value_ref,
        new_value_ref,
        recipe_ref,
        typed_ref_value,
        receipt_value,
    })
}

pub fn list_receipt_refs(root: &Path) -> Result<Vec<String>> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let mut refs = Vec::new();
    for item in table.iter().map_err(index_error)? {
        let (key, _value) = item.map_err(index_error)?;
        push_bounded(&mut refs, key.value().to_string(), MAX_TYPED_STORAGE_RECEIPTS, "typed storage receipt refs")?;
    }
    refs.sort();
    Ok(refs)
}

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<TypedStorageReceipt> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let Some(bytes) = table.get(receipt_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("unknown typed storage receipt {receipt_ref}")));
    };
    let value = parse_canonical_bytes(bytes.value())?;
    parse_receipt_value(&value, Some(receipt_ref))
}

pub fn inferred_schema_ref(value: &IOValue) -> Result<String> {
    canonical_hash(&inferred_schema_value(value))
}

pub fn inferred_schema_value(value: &IOValue) -> IOValue {
    let class = match value.value_class() {
        ValueClass::Atomic(_) => "atomic",
        ValueClass::Embedded => "embedded",
        ValueClass::Compound(CompoundClass::Record) => "record",
        ValueClass::Compound(CompoundClass::Sequence) => "sequence",
        ValueClass::Compound(CompoundClass::Set) => "set",
        ValueClass::Compound(CompoundClass::Dictionary) => "dictionary",
    };
    record("storage-schema-artifact-v1", vec![
        string(TYPED_STORAGE_SCHEMA_ARTIFACT_SCHEMA),
        record("inference", vec![string("preserves-value-class")]),
        record("class", vec![string(class)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("canonical-preserves-class"), string("pass")]),
            record("check", vec![string("no-raw-memory-layout"), string("pass")]),
        ])]),
    ])
}

pub fn effect_manifest_value(
    producer_ref: &str,
    namespace: &str,
    schema_ref: &str,
    operations: &[String],
) -> Result<IOValue> {
    require_ref(producer_ref, "storage effect manifest producer ref")?;
    validate_namespace(namespace)?;
    require_ref(schema_ref, "storage effect manifest schema ref")?;
    validate_operations(operations)?;
    Ok(record("storage-effect-manifest-v1", vec![
        string(TYPED_STORAGE_EFFECT_MANIFEST_SCHEMA),
        record("producer", vec![string(producer_ref)]),
        record("namespace", vec![string(namespace)]),
        record("schema-ref", vec![string(schema_ref)]),
        record("operations", vec![sequence(operations.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("declared-storage-effect"), string("pass")]),
            record("check", vec![string("typed-schema-binding"), string("pass")]),
            record("check", vec![string("handler-profile-required"), string("pass")]),
        ])]),
    ]))
}

pub fn migration_recipe_value(input: &StorageMigrationRecipeInput) -> Result<IOValue> {
    require_ref(&input.source_schema_ref, "migration source schema ref")?;
    require_ref(&input.target_schema_ref, "migration target schema ref")?;
    require_ref(&input.transformer_ref, "migration transformer ref")?;
    validate_transformer_kind(&input.transformer_kind)?;
    validate_migration_mode(&input.mode)?;
    validate_refs(&input.policy_refs, "migration policy ref")?;
    validate_refs(&input.evidence_refs, "migration evidence ref")?;
    Ok(record("storage-migration-recipe-v1", vec![
        string(TYPED_STORAGE_MIGRATION_RECIPE_SCHEMA),
        record("source-schema-ref", vec![string(&input.source_schema_ref)]),
        record("target-schema-ref", vec![string(&input.target_schema_ref)]),
        record("transformer", vec![string(&input.transformer_ref), string(&input.transformer_kind)]),
        record("mode", vec![string(&input.mode)]),
        refs_record("policy", &input.policy_refs),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "migration-recipe-artifact",
            "source-schema-binding",
            "target-schema-binding",
            "transformer-artifact-binding",
            "policy-admission-required",
            "migration-trace-required",
        ]),
    ]))
}

pub fn parse_migration_recipe_value(value: &IOValue) -> Result<StorageMigrationRecipe> {
    let recipe = simple_record(value, "storage-migration-recipe-v1", 8)?;
    require_schema(&recipe[0], TYPED_STORAGE_MIGRATION_RECIPE_SCHEMA, "storage migration recipe")?;
    let transformer = value_to_iovalue(&recipe[3]);
    let transformer = simple_record(&transformer, "transformer", 2)?;
    let transformer_kind = required_string(&transformer[1], "migration transformer kind")?;
    validate_transformer_kind(&transformer_kind)?;
    let mode = record_string(&recipe[4], "mode")?;
    validate_migration_mode(&mode)?;
    let checks = parse_checks(&recipe[7])?;
    require_check(&checks, "migration-recipe-artifact", "storage migration recipe")?;
    require_check(&checks, "migration-trace-required", "storage migration recipe")?;
    Ok(StorageMigrationRecipe {
        recipe_ref: canonical_hash(value)?,
        source_schema_ref: record_ref(&recipe[1], "source-schema-ref")?,
        target_schema_ref: record_ref(&recipe[2], "target-schema-ref")?,
        transformer_ref: required_ref(&transformer[0], "migration transformer ref")?,
        transformer_kind,
        mode,
        policy_refs: record_ref_sequence(&recipe[5], "policy")?,
        evidence_refs: record_ref_sequence(&recipe[6], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn parse_typed_ref_value(value: &IOValue) -> Result<TypedStorageRef> {
    let fields = simple_record(value, "typed-storage-ref-v1", 12)?;
    require_schema(&fields[0], TYPED_STORAGE_REF_SCHEMA, "typed storage ref")?;
    let namespace = record_string(&fields[1], "namespace")?;
    let key = record_string(&fields[2], "key")?;
    let schema_ref = record_ref(&fields[3], "schema-ref")?;
    let value_ref = record_ref(&fields[4], "value-ref")?;
    let payload = parse_payload(&fields[5])?;
    let producer_ref = record_ref(&fields[6], "producer")?;
    let policy_refs = record_ref_sequence(&fields[7], "policy")?;
    let evidence_refs = record_ref_sequence(&fields[8], "evidence")?;
    let revision = record_u64(&fields[9], "revision")?;
    let authority_value = value_to_iovalue(&fields[10]);
    let authority = simple_record(&authority_value, "authority", 3)?;
    let actor_ref = required_ref(&authority[0], "typed storage actor ref")?;
    let capability_ref = required_ref(&authority[1], "typed storage capability ref")?;
    let effect_handle_ref = required_ref(&authority[2], "typed storage effect handle ref")?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "typed-durable-ref", "typed storage ref")?;
    require_check(&checks, "handle-not-authority", "typed storage ref")?;
    let storage_ref = canonical_hash(value)?;
    Ok(TypedStorageRef {
        storage_ref,
        namespace,
        key,
        schema_ref,
        value_ref,
        payload,
        producer_ref,
        policy_refs,
        evidence_refs,
        revision,
        actor_ref,
        capability_ref,
        effect_handle_ref,
        checks,
        value: value.clone(),
    })
}

pub fn parse_receipt_value(value: &IOValue, expected_receipt_ref: Option<&str>) -> Result<TypedStorageReceipt> {
    let fields = simple_record(value, "typed-storage-receipt-v1", 9)?;
    require_schema(&fields[0], TYPED_STORAGE_RECEIPT_SCHEMA, "typed storage receipt")?;
    let operation = record_string(&fields[1], "operation")?;
    let decision = record_string(&fields[2], "decision")?;
    if decision != "pass" && decision != "deny" {
        return Err(MoltenError::invalid_harness(format!(
            "typed storage receipt decision must be pass or deny, got {decision}"
        )));
    }
    let storage_ref = record_optional_ref(&fields[3], "storage-ref")?;
    let binding = parse_binding_record(&fields[4])?;
    let value_ref = binding.value_ref.clone();
    let checks = parse_checks(&fields[6])?;
    let receipt_ref = canonical_hash(value)?;
    if let Some(expected) = expected_receipt_ref
        && receipt_ref != expected
    {
        return Err(MoltenError::invalid_harness(format!(
            "typed storage receipt hash mismatch: got {receipt_ref}, expected {expected}"
        )));
    }
    Ok(TypedStorageReceipt {
        receipt_ref,
        operation,
        decision,
        storage_ref,
        namespace: binding.namespace,
        key: binding.key,
        schema_ref: binding.schema_ref,
        value_ref,
        checks,
        value: value.clone(),
    })
}

struct TypedRefValueInput<'a> {
    namespace: &'a str,
    key: &'a str,
    schema_ref: &'a str,
    value_ref: &'a str,
    payload: &'a IOValue,
    producer_ref: &'a str,
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
    revision: u64,
    actor_ref: &'a str,
    capability_ref: &'a str,
    effect_handle_ref: &'a str,
}

fn typed_ref_value(input: TypedRefValueInput<'_>) -> IOValue {
    record("typed-storage-ref-v1", vec![
        string(TYPED_STORAGE_REF_SCHEMA),
        record("namespace", vec![string(input.namespace)]),
        record("key", vec![string(input.key)]),
        record("schema-ref", vec![string(input.schema_ref)]),
        record("value-ref", vec![string(input.value_ref)]),
        record("payload", vec![input.payload.clone()]),
        record("producer", vec![string(input.producer_ref)]),
        refs_record("policy", input.policy_refs),
        refs_record("evidence", input.evidence_refs),
        record("revision", vec![u64_value(input.revision)]),
        record("authority", vec![
            string(input.actor_ref),
            string(input.capability_ref),
            string(input.effect_handle_ref),
        ]),
        checks_value(&[
            "typed-durable-ref",
            "schema-ref-binding",
            "value-ref-binding",
            "producer-artifact-binding",
            "handle-not-authority",
            "no-raw-memory-layout",
        ]),
    ])
}

fn storage_effect_evidence(input: StorageEffectEvidenceInput<'_>) -> Result<StorageEffectEvidence> {
    validate_operation(input.operation)?;
    let manifest =
        effect_manifest_value(input.producer_ref, input.namespace, input.schema_ref, &[input.operation.to_string()])?;
    let manifest_ref = canonical_hash(&manifest)?;
    let run_ref =
        canonical_hash(&record("typed-storage-run", vec![string(input.namespace), string(input.schema_ref)]))?;
    let session_ref = canonical_hash(&record("typed-storage-session", vec![
        string(input.namespace),
        string(&input.admission.policy_ref),
        string(&input.admission.capability_ref),
    ]))?;
    let turn_ref = canonical_hash(&record("typed-storage-operation", vec![
        string(input.operation),
        string(input.namespace),
        string(input.key),
        string(input.schema_ref),
    ]))?;
    let scope = EffectScope {
        run_ref: run_ref.clone(),
        session_ref: session_ref.clone(),
        actor_ref: Some(input.admission.actor_ref.clone()),
        turn_ref: Some(turn_ref.clone()),
    };
    let adapter_ref =
        canonical_hash(&record("typed-storage-redb-adapter", vec![string(input.namespace), string(input.schema_ref)]))?;
    let mut evidence_refs = vec![manifest_ref.clone()];
    evidence_refs.extend(input.admission.evidence_refs.clone());
    let handler = handler_binding_value(&HandlerBindingInput {
        profile: "typed-storage-redb".to_string(),
        scope: scope.clone(),
        adapter_kind: ADAPTER_KIND_STORAGE.to_string(),
        adapter_ref,
        executor_preflight_ref: None,
        policy_ref: input.admission.policy_ref.clone(),
        capability_context_ref: input.admission.capability_ref.clone(),
        authority_context_ref: None,
        resource_refs: input.admission.resource_refs.clone(),
        operations: vec![input.operation.to_string()],
        evidence_refs: evidence_refs.clone(),
    })?;
    let handler_binding_ref = canonical_hash(&handler)?;
    let handle = effect_handle_value(&EffectHandleInput {
        kind: ADAPTER_KIND_STORAGE.to_string(),
        scope,
        handler_binding_ref: handler_binding_ref.clone(),
        operations: vec![input.operation.to_string()],
        capability_context_ref: input.admission.capability_ref.clone(),
        authority_context_ref: None,
        resource_refs: input.admission.resource_refs.clone(),
        not_before: Some(0),
        expires_at: None,
        revocation_refs: Vec::new(),
        transfer: TRANSFER_LOCAL_ONLY.to_string(),
        parent_handle_ref: None,
        evidence_refs,
    })?;
    let handle_ref = canonical_hash(&handle)?;
    let validation = validate_handle_for_request(&handler, &handle, &EffectHandleRequest {
        kind: ADAPTER_KIND_STORAGE,
        operation: input.operation,
        run_ref: &run_ref,
        session_ref: &session_ref,
        actor_ref: Some(&input.admission.actor_ref),
        turn_ref: Some(&turn_ref),
        policy_ref: &input.admission.policy_ref,
        capability_context_ref: &input.admission.capability_ref,
        authority_context_ref: None,
        resource_refs: &input.admission.resource_refs,
        logical_time: 0,
        remote_use: input.remote_use,
        revoked_refs: &[],
    })?;
    if validation.handler_binding_ref != handler_binding_ref || validation.handle_ref != handle_ref {
        return Err(MoltenError::invalid_harness("typed storage handle validation ref mismatch"));
    }
    Ok(StorageEffectEvidence {
        manifest_ref,
        handler_binding_ref,
        handle_ref,
    })
}

fn store_payload(root: &Path, value_bytes: &[u8]) -> Result<(IOValue, Vec<IOValue>)> {
    if value_bytes.len() <= INLINE_VALUE_LIMIT {
        Ok((record("inline", vec![u64_value(value_bytes.len() as u64)]), vec![record("payload", vec![
            string("inline"),
            u64_value(value_bytes.len() as u64),
        ])]))
    } else {
        let put =
            chunk_store::put_bytes(&chunk_root(root), "typed-storage-value", value_bytes, DEFAULT_FIXED_V1_CHUNK_SIZE)?;
        Ok((record("content-ref", vec![string(&put.manifest_ref), u64_value(value_bytes.len() as u64)]), vec![
            record("payload", vec![string("content-ref"), string(&put.manifest_ref)]),
            record("chunk-store-receipt", vec![string(canonical_hash(&put.receipt_value)?)]),
        ]))
    }
}

fn apply_migration_transform(recipe: &StorageMigrationRecipe, old_value: &IOValue) -> Result<IOValue> {
    match recipe.transformer_kind.as_str() {
        "identity" | "schema-rename" => Ok(old_value.clone()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported typed storage migration transformer kind {other}"
        ))),
    }
}

fn read_payload_bytes(root: &Path, typed_ref: &TypedStorageRef) -> Result<Vec<u8>> {
    match &typed_ref.payload {
        TypedStoragePayload::Inline { length } => {
            let db = ensure_index_tables(root)?;
            let read_txn = db.begin_read().map_err(index_error)?;
            let table = read_txn.open_table(INDEX_INLINE_VALUES).map_err(index_error)?;
            let Some(bytes) = table.get(typed_ref.value_ref.as_str()).map_err(index_error)? else {
                return Err(MoltenError::invalid_harness(format!(
                    "missing inline typed storage value {}",
                    typed_ref.value_ref
                )));
            };
            let bytes = bytes.value().to_vec();
            if bytes.len() as u64 != *length {
                return Err(MoltenError::invalid_harness(format!(
                    "inline typed storage length mismatch: got {}, expected {length}",
                    bytes.len()
                )));
            }
            Ok(bytes)
        }
        TypedStoragePayload::ContentRef { manifest_ref, length } => {
            let read = chunk_store::read_object(&chunk_root(root), manifest_ref)?;
            if read.bytes.len() as u64 != *length {
                return Err(MoltenError::invalid_harness(format!(
                    "chunk-backed typed storage length mismatch: got {}, expected {length}",
                    read.bytes.len()
                )));
            }
            Ok(read.bytes)
        }
    }
}

fn read_typed_ref(root: &Path, storage_ref: &str) -> Result<IOValue> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let refs = read_txn.open_table(INDEX_REFS).map_err(index_error)?;
    let Some(bytes) = refs.get(storage_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("unknown typed storage ref {storage_ref}")));
    };
    parse_canonical_bytes(bytes.value())
}

fn next_revision(root: &Path, storage_key: &str) -> Result<u64> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let records = read_txn.open_table(INDEX_RECORDS).map_err(index_error)?;
    let Some(bytes) = records.get(storage_key).map_err(index_error)? else {
        return Ok(1);
    };
    let value = parse_canonical_bytes(bytes.value())?;
    Ok(parse_typed_ref_value(&value)?.revision.saturating_add(1))
}

fn receipt_value(input: ReceiptValueInput<'_>) -> IOValue {
    record("typed-storage-receipt-v1", vec![
        string(TYPED_STORAGE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("storage-ref", vec![optional_ref_value(input.storage_ref)]),
        record("binding", vec![
            optional_string_value(input.namespace),
            optional_string_value(input.key),
            optional_ref_value(input.schema_ref),
            optional_ref_value(input.value_ref),
        ]),
        record("effect", vec![
            string(&input.effect.manifest_ref),
            string(&input.effect.handler_binding_ref),
            string(&input.effect.handle_ref),
        ]),
        checks_record(input.checks),
        record("details", vec![sequence(input.details)]),
        record("tool", vec![string("molten"), string(env!("CARGO_PKG_VERSION"))]),
    ])
}

fn denial_receipt_value(input: DenialReceiptValueInput<'_>) -> IOValue {
    let fallback_ref = local_ref("typed-storage-denial-effect", input.operation);
    let effect = StorageEffectEvidence {
        manifest_ref: fallback_ref.clone(),
        handler_binding_ref: fallback_ref.clone(),
        handle_ref: fallback_ref,
    };
    let mut details = input.details;
    details.push(record("reason", vec![string(input.reason)]));
    receipt_value(ReceiptValueInput {
        operation: input.operation,
        decision: "deny",
        storage_ref: input.storage_ref,
        namespace: input.namespace,
        key: input.key,
        schema_ref: input.schema_ref,
        value_ref: input.value_ref,
        effect: &effect,
        checks: input.checks,
        details,
    })
}

fn parse_payload(value: &Value<IOValue>) -> Result<TypedStoragePayload> {
    let value = value_to_iovalue(value);
    let payload = simple_record(&value, "payload", 1)?;
    let payload_value = value_to_iovalue(&payload[0]);
    if let Some(inline) = payload_value.collect_simple_record("inline", Some(1)) {
        return Ok(TypedStoragePayload::Inline {
            length: required_u64(&inline[0], "inline payload length")?,
        });
    }
    if let Some(content) = payload_value.collect_simple_record("content-ref", Some(2)) {
        return Ok(TypedStoragePayload::ContentRef {
            manifest_ref: required_ref(&content[0], "payload manifest ref")?,
            length: required_u64(&content[1], "content payload length")?,
        });
    }
    Err(MoltenError::invalid_harness("typed storage payload must be inline or content-ref"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReceiptBinding {
    namespace: Option<String>,
    key: Option<String>,
    schema_ref: Option<String>,
    value_ref: Option<String>,
}

fn parse_binding_record(value: &Value<IOValue>) -> Result<ReceiptBinding> {
    let value = value_to_iovalue(value);
    let binding = simple_record(&value, "binding", 4)?;
    Ok(ReceiptBinding {
        namespace: parse_optional_string_value(&binding[0])?,
        key: parse_optional_string_value(&binding[1])?,
        schema_ref: parse_optional_ref_value(&binding[2])?,
        value_ref: parse_optional_ref_value(&binding[3])?,
    })
}

fn storage_key_ref(namespace: &str, key: &str) -> Result<String> {
    canonical_hash(&record("typed-storage-key-v1", vec![string(namespace), string(key)]))
}

fn chunk_root(root: &Path) -> PathBuf {
    root.join("chunks")
}

fn ensure_dirs(root: &Path) -> Result<()> {
    fs::create_dir_all(root).map_err(MoltenError::from)?;
    fs::create_dir_all(chunk_root(root)).map_err(MoltenError::from)
}

fn ensure_index_tables(root: &Path) -> Result<Database> {
    ensure_dirs(root)?;
    let db = Database::create(index_path(root)).map_err(index_error)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        write_txn.open_table(INDEX_RECORDS).map_err(index_error)?;
        write_txn.open_table(INDEX_REFS).map_err(index_error)?;
        write_txn.open_table(INDEX_INLINE_VALUES).map_err(index_error)?;
        write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(db)
}

fn store_receipt(root: &Path, receipt_value: &IOValue) -> Result<()> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    store_receipt_in_tx(&write_txn, receipt_value)?;
    write_txn.commit().map_err(index_error)
}

fn store_receipt_in_tx(write_txn: &redb::WriteTransaction, receipt_value: &IOValue) -> Result<()> {
    let parsed = parse_receipt_value(receipt_value, None)?;
    let receipt_bytes = canonical_bytes(receipt_value)?;
    let mut receipts = write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    receipts.insert(parsed.receipt_ref.as_str(), receipt_bytes.as_slice()).map_err(index_error)?;
    Ok(())
}

fn index_path(root: &Path) -> PathBuf {
    root.join(INDEX_FILE)
}

fn index_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("typed storage redb index error: {error}"))
}

fn validate_admission(admission: &TypedStorageAdmission) -> Result<()> {
    require_ref(&admission.actor_ref, "typed storage actor ref")?;
    require_ref(&admission.capability_ref, "typed storage capability ref")?;
    require_ref(&admission.policy_ref, "typed storage policy ref")?;
    if admission.resource_refs.is_empty() {
        return Err(MoltenError::invalid_harness("typed storage admission requires at least one resource ref"));
    }
    validate_refs(&admission.resource_refs, "typed storage resource ref")?;
    validate_refs(&admission.evidence_refs, "typed storage admission evidence ref")
}

fn validate_namespace_key(namespace: &str, key: &str) -> Result<()> {
    validate_namespace(namespace)?;
    if key.is_empty() {
        return Err(MoltenError::invalid_harness("typed storage key must not be empty"));
    }
    Ok(())
}

fn validate_namespace(namespace: &str) -> Result<()> {
    if namespace.is_empty() {
        return Err(MoltenError::invalid_harness("typed storage namespace must not be empty"));
    }
    if namespace.chars().any(char::is_whitespace) {
        return Err(MoltenError::invalid_harness("typed storage namespace must not contain whitespace"));
    }
    Ok(())
}

fn validate_operations(operations: &[String]) -> Result<()> {
    if operations.is_empty() {
        return Err(MoltenError::invalid_harness("storage effect manifest operations must not be empty"));
    }
    let mut seen = BTreeSet::new();
    for operation in operations {
        validate_operation(operation)?;
        if !seen.insert(operation.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate storage operation {operation}")));
        }
    }
    Ok(())
}

fn validate_operation(operation: &str) -> Result<()> {
    if !matches!(operation, "put" | "get" | "verify" | "migrate") {
        return Err(MoltenError::invalid_harness(format!("unsupported typed storage operation {operation}")));
    }
    Ok(())
}

fn validate_transformer_kind(kind: &str) -> Result<()> {
    if !matches!(kind, "identity" | "schema-rename") {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported typed storage migration transformer kind {kind}"
        )));
    }
    Ok(())
}

fn validate_migration_mode(mode: &str) -> Result<()> {
    if !matches!(mode, "explicit" | "lazy-on-read" | "batch") {
        return Err(MoltenError::invalid_harness(format!("unsupported typed storage migration mode {mode}")));
    }
    Ok(())
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

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}

fn require_ref(reference: &str, field: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })
}

fn required_ref(value: &Value<IOValue>, field: &str) -> Result<String> {
    let reference = required_string(value, field)?;
    require_ref(&reference, field)?;
    Ok(reference)
}

fn checks_value(checks: &[&str]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|check| record("check", vec![string(*check), string("pass")])).collect(),
    )])
}

fn checks_record(checks: Vec<(&str, &str)>) -> IOValue {
    record("checks", vec![sequence(
        checks
            .into_iter()
            .map(|(check, status)| record("check", vec![string(check), string(status)]))
            .collect(),
    )])
}

fn refs_record(label: &'static str, refs: &[String]) -> IOValue {
    record(label, vec![sequence(refs.iter().map(string).collect())])
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_string_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn parse_optional_string_value(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_string(&some[0], "optional string").map(Some);
    }
    required_string(value, "optional string").map(Some)
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

fn record_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_u64(&record[0], label)
}

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        refs.push(required_ref(item, label)?);
    }
    Ok(refs)
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "checks")?;
    let mut parsed = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("typed storage check {name} has status {status}")));
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

fn required_u64(value: &Value<IOValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn local_ref(kind: &str, label: &str) -> String {
    let value = record("typed-storage-local-ref", vec![string(kind), string(label)]);
    match canonical_hash(&value) {
        Ok(reference) => reference,
        Err(error) => content_ref_from_bytes(format!("typed-storage-local-ref-error:{error}").as_bytes()),
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
    use crate::preserves_rail::to_text;

    #[test]
    fn typed_storage_roundtrip_schema_tagged_preserves_value() {
        let root = temp_dir("typed-storage-roundtrip");
        let value = parse_text("<profile \"alice\" 7>").expect("parse value");
        let producer_ref = test_ref("producer");
        let put = put_value(&root, &TypedStoragePutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            value: value.clone(),
            producer_ref: producer_ref.clone(),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: TypedStorageAdmission::local_fixture("roundtrip"),
        })
        .expect("put value");
        let get = get_value(
            &root,
            "profiles",
            "alice",
            Some(&put.schema_ref),
            &TypedStorageAdmission::local_fixture("roundtrip"),
        )
        .expect("get value");
        assert_eq!(get.value, value);
        assert_eq!(get.storage_ref, put.storage_ref);
        let verify = verify_ref(&root, &put.storage_ref, Some(&put.schema_ref)).expect("verify ref");
        assert_eq!(verify.storage_ref, put.storage_ref);
        let receipt_refs = list_receipt_refs(&root).expect("receipt refs");
        assert!(receipt_refs.len() >= 3);
        assert!(
            parse_receipt_value(&put.receipt_value, None)
                .expect("parse put receipt")
                .checks
                .contains(&"write-admission".to_string())
        );
    }

    #[test]
    fn typed_storage_schema_mismatch_is_denied_before_write_or_load() {
        let root = temp_dir("typed-storage-schema-mismatch");
        let value = parse_text("\"alice\"").expect("parse value");
        let wrong_schema_ref = test_ref("wrong-schema");
        let error = put_value(&root, &TypedStoragePutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: Some(wrong_schema_ref),
            value: value.clone(),
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: TypedStorageAdmission::local_fixture("schema-mismatch"),
        })
        .expect_err("wrong write schema denied");
        assert!(error.to_string().contains("schema ref"));
        assert_eq!(list_receipt_refs(&root).expect("receipt refs").len(), 1);

        let put = put_value(&root, &TypedStoragePutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            value,
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: TypedStorageAdmission::local_fixture("schema-mismatch"),
        })
        .expect("put inferred");
        let load_error = get_value(
            &root,
            "profiles",
            "alice",
            Some(&test_ref("other-schema")),
            &TypedStorageAdmission::local_fixture("schema-mismatch"),
        )
        .expect_err("wrong load schema denied");
        assert!(load_error.to_string().contains("schema ref"));
        assert!(verify_ref(&root, &put.storage_ref, Some(&put.schema_ref)).is_ok());
    }

    #[test]
    fn explicit_and_lazy_migrations_preserve_value_hash_and_trace_refs() {
        let root = temp_dir("typed-storage-migration");
        let value = parse_text("<profile \"alice\" 7>").expect("parse value");
        let admission = TypedStorageAdmission::local_fixture("migration");
        let put = put_value(&root, &TypedStoragePutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            value: value.clone(),
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: admission.clone(),
        })
        .expect("put source");
        let target_schema_ref = test_ref("profile-v2-schema");
        let recipe = migration_recipe_value(&StorageMigrationRecipeInput {
            source_schema_ref: put.schema_ref.clone(),
            target_schema_ref: target_schema_ref.clone(),
            transformer_ref: test_ref("schema-rename-transformer"),
            transformer_kind: "schema-rename".to_string(),
            mode: "explicit".to_string(),
            policy_refs: vec![test_ref("migration-policy")],
            evidence_refs: vec![test_ref("migration-evidence")],
        })
        .expect("recipe");
        let before_error = get_value(&root, "profiles", "alice", Some(&target_schema_ref), &admission)
            .expect_err("target schema rejected before migration");
        assert!(before_error.to_string().contains("schema ref"), "{before_error}");
        let migrated = migrate_value(&root, "profiles", "alice", &recipe, &admission).expect("migrate");
        assert_eq!(migrated.old_storage_ref, put.storage_ref);
        assert_eq!(migrated.old_value_ref, put.value_ref);
        assert_eq!(migrated.new_value_ref, put.value_ref);
        let parsed_receipt = parse_receipt_value(&migrated.receipt_value, None).expect("parse migrate receipt");
        assert!(parsed_receipt.checks.contains(&"migration-trace".to_string()));
        assert!(parsed_receipt.checks.contains(&"original-value-hash".to_string()));
        let loaded =
            get_value(&root, "profiles", "alice", Some(&target_schema_ref), &admission).expect("load migrated target");
        assert_eq!(loaded.value, value);
        assert_eq!(loaded.typed_ref.schema_ref, target_schema_ref);

        let lazy_value = parse_text("<profile \"bob\" 9>").expect("parse lazy value");
        let lazy_put = put_value(&root, &TypedStoragePutInput {
            namespace: "profiles".to_string(),
            key: "bob".to_string(),
            schema_ref: None,
            value: lazy_value.clone(),
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: admission.clone(),
        })
        .expect("put lazy source");
        let lazy_target_schema_ref = test_ref("profile-v3-schema");
        let lazy_recipe = migration_recipe_value(&StorageMigrationRecipeInput {
            source_schema_ref: lazy_put.schema_ref,
            target_schema_ref: lazy_target_schema_ref.clone(),
            transformer_ref: test_ref("lazy-transformer"),
            transformer_kind: "identity".to_string(),
            mode: "lazy-on-read".to_string(),
            policy_refs: vec![test_ref("lazy-policy")],
            evidence_refs: vec![test_ref("lazy-evidence")],
        })
        .expect("lazy recipe");
        let lazy_loaded = get_value_with_migration(MigrationGetInput {
            root: &root,
            namespace: "profiles",
            key: "bob",
            expected_schema_ref: &lazy_target_schema_ref,
            migration_recipe_value: &lazy_recipe,
            admission: &admission,
        })
        .expect("lazy migration load");
        assert_eq!(lazy_loaded.value, lazy_value);
        assert_eq!(lazy_loaded.typed_ref.schema_ref, lazy_target_schema_ref);
    }

    #[test]
    fn migration_requires_matching_source_schema_and_admitted_recipe() {
        let root = temp_dir("typed-storage-migration-deny");
        let value = parse_text("\"alice\"").expect("parse value");
        let admission = TypedStorageAdmission::local_fixture("migration-deny");
        let put = put_value(&root, &TypedStoragePutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            value,
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: admission.clone(),
        })
        .expect("put source");
        let wrong_source_recipe = migration_recipe_value(&StorageMigrationRecipeInput {
            source_schema_ref: test_ref("wrong-source"),
            target_schema_ref: test_ref("target"),
            transformer_ref: test_ref("transformer"),
            transformer_kind: "identity".to_string(),
            mode: "explicit".to_string(),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("wrong source recipe");
        let error = migrate_value(&root, "profiles", "alice", &wrong_source_recipe, &admission)
            .expect_err("wrong source denied");
        assert!(error.to_string().contains("source schema"), "{error}");

        let unsupported_transformer = record("storage-migration-recipe-v1", vec![
            string(TYPED_STORAGE_MIGRATION_RECIPE_SCHEMA),
            record("source-schema-ref", vec![string(&put.schema_ref)]),
            record("target-schema-ref", vec![string(test_ref("target"))]),
            record("transformer", vec![string(test_ref("transformer")), string("ambient-script")]),
            record("mode", vec![string("explicit")]),
            refs_record("policy", &[test_ref("policy")]),
            refs_record("evidence", &[test_ref("evidence")]),
            checks_value(&[
                "migration-recipe-artifact",
                "source-schema-binding",
                "target-schema-binding",
                "transformer-artifact-binding",
                "policy-admission-required",
                "migration-trace-required",
            ]),
        ]);
        let error = parse_migration_recipe_value(&unsupported_transformer).expect_err("unsupported transformer denied");
        assert!(error.to_string().contains("transformer kind"), "{error}");
    }

    #[test]
    fn schema_identity_structural_alias_and_migration_available_integrate_with_loads() {
        let root = temp_dir("typed-storage-schema-identity");
        let value = parse_text("<profile \"alice\" 7>").expect("parse value");
        let admission = TypedStorageAdmission::local_fixture("schema-identity");
        let put = put_value(&root, &TypedStoragePutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            value: value.clone(),
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: admission.clone(),
        })
        .expect("put source");
        let shape = record("shape", vec![
            string("record"),
            string("profile"),
            sequence(vec![
                record("shape", vec![string("field"), string("name"), record("shape", vec![string("string")])]),
                record("shape", vec![string("field"), string("age"), record("shape", vec![string("u64")])]),
            ]),
        ]);
        let actual_identity = schema_identity::parse_schema_identity(
            &schema_identity::schema_identity_value(&schema_identity::SchemaIdentityInput {
                mode: schema_identity::MODE_STRUCTURAL.to_string(),
                schema_ref: put.schema_ref.clone(),
                shape: shape.clone(),
                brand_ref: None,
                metadata_refs: vec![test_ref("actual-metadata")],
                policy_refs: vec![test_ref("schema-policy")],
                evidence_refs: vec![test_ref("schema-evidence")],
            })
            .expect("actual identity"),
        )
        .expect("parse actual identity");
        let expected_schema_ref = test_ref("expected-structural-schema");
        let expected_identity = schema_identity::parse_schema_identity(
            &schema_identity::schema_identity_value(&schema_identity::SchemaIdentityInput {
                mode: schema_identity::MODE_STRUCTURAL.to_string(),
                schema_ref: expected_schema_ref.clone(),
                shape: shape.clone(),
                brand_ref: None,
                metadata_refs: vec![test_ref("expected-metadata")],
                policy_refs: vec![test_ref("schema-policy")],
                evidence_refs: vec![test_ref("schema-evidence")],
            })
            .expect("expected identity"),
        )
        .expect("parse expected identity");
        let compatibility = schema_identity::compatibility_decision_value(&schema_identity::SchemaCompatibilityInput {
            expected: expected_identity,
            actual: actual_identity,
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref("compat-policy")],
            evidence_refs: vec![test_ref("compat-evidence")],
            deny_by_policy: false,
        })
        .expect("structural compatibility");
        let loaded = get_value_with_schema_compatibility(SchemaCompatibilityGetInput {
            root: &root,
            namespace: "profiles",
            key: "alice",
            expected_schema_ref: &expected_schema_ref,
            schema_compatibility_value: &compatibility,
            admission: &admission,
        })
        .expect("load through structural compatibility");
        assert_eq!(loaded.value, value);
        assert!(to_text(&loaded.receipt_value).expect("receipt text").contains("schema-compatibility-value"));

        let unique_expected_schema_ref = test_ref("unique-expected-schema");
        let actual_unique = schema_identity::parse_schema_identity(
            &schema_identity::schema_identity_value(&schema_identity::SchemaIdentityInput {
                mode: schema_identity::MODE_UNIQUE.to_string(),
                schema_ref: put.schema_ref.clone(),
                shape: shape.clone(),
                brand_ref: None,
                metadata_refs: vec![test_ref("actual-unique")],
                policy_refs: vec![test_ref("schema-policy")],
                evidence_refs: vec![test_ref("schema-evidence")],
            })
            .expect("actual unique identity"),
        )
        .expect("parse actual unique");
        let expected_unique = schema_identity::parse_schema_identity(
            &schema_identity::schema_identity_value(&schema_identity::SchemaIdentityInput {
                mode: schema_identity::MODE_UNIQUE.to_string(),
                schema_ref: unique_expected_schema_ref.clone(),
                shape,
                brand_ref: None,
                metadata_refs: vec![test_ref("expected-unique")],
                policy_refs: vec![test_ref("schema-policy")],
                evidence_refs: vec![test_ref("schema-evidence")],
            })
            .expect("expected unique identity"),
        )
        .expect("parse expected unique");
        let mismatch = schema_identity::compatibility_decision_value(&schema_identity::SchemaCompatibilityInput {
            expected: expected_unique.clone(),
            actual: actual_unique.clone(),
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref("compat-policy")],
            evidence_refs: vec![test_ref("compat-evidence")],
            deny_by_policy: false,
        })
        .expect("unique mismatch");
        let error = get_value_with_schema_compatibility(SchemaCompatibilityGetInput {
            root: &root,
            namespace: "profiles",
            key: "alice",
            expected_schema_ref: &unique_expected_schema_ref,
            schema_compatibility_value: &mismatch,
            admission: &admission,
        })
        .expect_err("unique mismatch denied");
        assert!(error.to_string().contains("schema ref"), "{error}");
        let alias = schema_identity::parse_schema_alias(
            &schema_identity::schema_alias_value(&schema_identity::SchemaAliasInput {
                from_schema_ref: put.schema_ref.clone(),
                to_schema_ref: unique_expected_schema_ref.clone(),
                scope: "storage".to_string(),
                policy_refs: vec![test_ref("alias-policy")],
                evidence_refs: vec![test_ref("alias-evidence")],
            })
            .expect("alias"),
        )
        .expect("parse alias");
        let admitted_alias =
            schema_identity::compatibility_decision_value(&schema_identity::SchemaCompatibilityInput {
                expected: expected_unique,
                actual: actual_unique,
                alias: Some(alias),
                migration_ref: None,
                policy_refs: vec![test_ref("compat-policy")],
                evidence_refs: vec![test_ref("compat-evidence")],
                deny_by_policy: false,
            })
            .expect("alias compatibility");
        get_value_with_schema_compatibility(SchemaCompatibilityGetInput {
            root: &root,
            namespace: "profiles",
            key: "alice",
            expected_schema_ref: &unique_expected_schema_ref,
            schema_compatibility_value: &admitted_alias,
            admission: &admission,
        })
        .expect("alias load admitted");
    }

    #[test]
    fn large_values_use_chunk_backed_content_refs() {
        let root = temp_dir("typed-storage-large");
        let large = "x".repeat(INLINE_VALUE_LIMIT + 128);
        let value = IOValue::new(large.clone());
        let put = put_value(&root, &TypedStoragePutInput {
            namespace: "large".to_string(),
            key: "payload".to_string(),
            schema_ref: None,
            value,
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: TypedStorageAdmission::local_fixture("large"),
        })
        .expect("put large");
        let parsed = parse_typed_ref_value(&put.typed_ref_value).expect("parse typed ref");
        let TypedStoragePayload::ContentRef { manifest_ref, .. } = &parsed.payload else {
            panic!("large typed storage value must use chunk manifest ref");
        };
        let manifest =
            chunk_store::read_manifest(&chunk_root(&root), manifest_ref).expect("read typed storage chunk manifest");
        assert_eq!(manifest.object_kind, "typed-storage-value");
        let get =
            get_value(&root, "large", "payload", Some(&put.schema_ref), &TypedStorageAdmission::local_fixture("large"))
                .expect("get large");
        assert_eq!(get.value.as_string().expect("string").as_ref(), large);

        let chunk_hex = crate::preserves_rail::content_ref_hex(&manifest.chunks[0].chunk_ref).expect("chunk hex");
        fs::write(chunk_root(&root).join("chunks").join(format!("blake3_{chunk_hex}.bin")), b"tampered")
            .expect("tamper typed storage chunk");
        let error =
            get_value(&root, "large", "payload", Some(&put.schema_ref), &TypedStorageAdmission::local_fixture("large"))
                .expect_err("tampered chunk denies before typed storage load");
        let message = error.to_string();
        let is_integrity_boundary_mentioned = message.contains("chunk") || message.contains("hash");
        assert!(is_integrity_boundary_mentioned, "{message}");
    }

    #[test]
    fn storage_refs_do_not_mint_authority_from_snapshots() {
        let root = temp_dir("typed-storage-authority");
        let value = parse_text("<snapshot [\"state\"]>").expect("parse snapshot");
        let put = put_value(&root, &TypedStoragePutInput {
            namespace: "snapshots".to_string(),
            key: "actor".to_string(),
            schema_ref: None,
            value,
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: TypedStorageAdmission::local_fixture("authority"),
        })
        .expect("put snapshot");
        let missing_authority = TypedStorageAdmission {
            actor_ref: test_ref("actor"),
            capability_ref: "not-a-ref".to_string(),
            policy_ref: test_ref("policy"),
            resource_refs: vec![test_ref("resource")],
            evidence_refs: vec![test_ref("evidence")],
        };
        let error = get_value(&root, "snapshots", "actor", Some(&put.schema_ref), &missing_authority)
            .expect_err("bad authority denied");
        assert!(error.to_string().contains("capability"));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_typed_ref_hashes_schema_and_revision_are_stable(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let root = temp_dir("typed-storage-hegel");
        let value = IOValue::new(format!("value-{salt}"));
        let input = TypedStoragePutInput {
            namespace: format!("ns-{salt}"),
            key: format!("key-{salt}"),
            schema_ref: None,
            value: value.clone(),
            producer_ref: test_ref(&format!("producer-{salt}")),
            policy_refs: vec![test_ref(&format!("policy-{salt}"))],
            evidence_refs: vec![test_ref(&format!("evidence-{salt}"))],
            admission: TypedStorageAdmission::local_fixture(&format!("hegel-{salt}")),
        };
        let first = put_value(&root, &input).expect("first put");
        let first_ref = parse_typed_ref_value(&first.typed_ref_value).expect("first ref");
        assert_eq!(first.schema_ref, inferred_schema_ref(&value).expect("schema ref"));
        assert_eq!(first_ref.revision, 1);
        let loaded = get_value(&root, &input.namespace, &input.key, Some(&first.schema_ref), &input.admission)
            .expect("load first");
        assert_eq!(canonical_hash(&loaded.value).expect("loaded value ref"), first.value_ref);
        let second = put_value(&root, &input).expect("second put");
        let second_ref = parse_typed_ref_value(&second.typed_ref_value).expect("second ref");
        assert_eq!(second_ref.revision, 2);
        assert_ne!(first.storage_ref, second.storage_ref);
        assert_eq!(first.value_ref, second.value_ref);

        let target_schema_ref = test_ref(&format!("target-schema-{salt}"));
        let recipe = migration_recipe_value(&StorageMigrationRecipeInput {
            source_schema_ref: second.schema_ref.clone(),
            target_schema_ref: target_schema_ref.clone(),
            transformer_ref: test_ref(&format!("transformer-{salt}")),
            transformer_kind: "identity".to_string(),
            mode: "explicit".to_string(),
            policy_refs: vec![test_ref(&format!("migration-policy-{salt}"))],
            evidence_refs: vec![test_ref(&format!("migration-evidence-{salt}"))],
        })
        .expect("migration recipe");
        let migrated = migrate_value(&root, &input.namespace, &input.key, &recipe, &input.admission).expect("migrate");
        assert_eq!(migrated.old_value_ref, second.value_ref);
        assert_eq!(migrated.new_value_ref, second.value_ref);
        let migrated_ref = parse_typed_ref_value(&migrated.typed_ref_value).expect("migrated ref");
        assert_eq!(migrated_ref.schema_ref, target_schema_ref);
        assert_eq!(migrated_ref.revision, 3);
        let receipt = parse_receipt_value(&migrated.receipt_value, None).expect("migration receipt");
        assert!(receipt.checks.contains(&"migration-trace".to_string()));
        assert!(receipt.checks.contains(&"result-value-hash".to_string()));
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("typed-storage-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
