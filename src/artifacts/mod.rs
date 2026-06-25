use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use preserves::IOValue;
use preserves::Value;
use redb::Database;
use redb::ReadableDatabase;
use redb::ReadableTable;
use redb::TableDefinition;

use crate::chunk_store;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::ARTIFACT_CLOSURE_SCHEMA;
use crate::preserves_rail::ARTIFACT_NAME_POINTER_SCHEMA;
use crate::preserves_rail::ARTIFACT_RECEIPT_SCHEMA;
use crate::preserves_rail::ARTIFACT_SCHEMA;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::parse_canonical_bytes;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::to_text;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;

pub const INLINE_PAYLOAD_LIMIT: usize = 4096;

const MAX_ARTIFACT_REF_LIST: usize = 4096;
const MAX_ARTIFACT_RECORDS: usize = 100_000;
const MAX_ARTIFACT_POINTERS: usize = 100_000;
const MAX_ARTIFACT_RECEIPTS: usize = 100_000;
const MAX_ARTIFACT_DIAGNOSTICS: usize = 256;
const MAX_ARTIFACT_CHECKS: usize = 64;

const _: () = assert!(INLINE_PAYLOAD_LIMIT <= 1_048_576);
const _: () = assert!(MAX_ARTIFACT_REF_LIST <= 100_000);
const _: () = assert!(MAX_ARTIFACT_RECORDS <= 1_000_000);
const _: () = assert!(MAX_ARTIFACT_POINTERS <= 1_000_000);
const _: () = assert!(MAX_ARTIFACT_RECEIPTS <= 1_000_000);
const _: () = assert!(MAX_ARTIFACT_DIAGNOSTICS <= 10_000);
const _: () = assert!(MAX_ARTIFACT_CHECKS <= 1_000);

const INDEX_FILE: &str = "artifact-registry.redb";
const INDEX_ARTIFACTS: TableDefinition<&str, &[u8]> = TableDefinition::new("artifact_registry_artifacts_v1");
const INDEX_PAYLOADS: TableDefinition<&str, &[u8]> = TableDefinition::new("artifact_registry_payloads_v1");
const INDEX_NAMES: TableDefinition<&str, &[u8]> = TableDefinition::new("artifact_registry_names_v1");
const INDEX_DEPS: TableDefinition<&str, &[u8]> = TableDefinition::new("artifact_registry_dependencies_v1");
const INDEX_REVERSE: TableDefinition<&str, &[u8]> = TableDefinition::new("artifact_registry_reverse_dependencies_v1");
const INDEX_KIND: TableDefinition<&str, &str> = TableDefinition::new("artifact_registry_kind_v1");
const INDEX_SCHEMA: TableDefinition<&str, &str> = TableDefinition::new("artifact_registry_schema_v1");
const INDEX_EFFECT: TableDefinition<&str, &str> = TableDefinition::new("artifact_registry_effect_v1");
const INDEX_POLICY: TableDefinition<&str, &str> = TableDefinition::new("artifact_registry_policy_v1");
const INDEX_EVIDENCE: TableDefinition<&str, &str> = TableDefinition::new("artifact_registry_evidence_v1");
const INDEX_RECEIPTS: TableDefinition<&str, &[u8]> = TableDefinition::new("artifact_registry_receipts_v1");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactPayloadRef {
    Inline { value_ref: String, length: u64 },
    ContentRef { manifest_ref: String, length: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInstallInput {
    pub kind: String,
    pub payload: IOValue,
    pub schema_refs: Vec<String>,
    pub dependency_refs: Vec<String>,
    pub effect_manifest_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub installer_ref: String,
    pub capability_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ArtifactValueInput<'a> {
    pub kind: &'a str,
    pub payload: &'a ArtifactPayloadRef,
    pub schema_refs: &'a [String],
    pub dependency_refs: &'a [String],
    pub effect_manifest_ref: Option<&'a str>,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct SetNamePointerInput<'a> {
    pub pointer_kind: &'a str,
    pub name: &'a str,
    pub artifact_ref: &'a str,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub artifact_ref: String,
    pub kind: String,
    pub domain: String,
    pub payload: ArtifactPayloadRef,
    pub schema_refs: Vec<String>,
    pub dependency_refs: Vec<String>,
    pub effect_manifest_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInstall {
    pub artifact_ref: String,
    pub decision: String,
    pub artifact: ArtifactRecord,
    pub missing_dependencies: Vec<String>,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactNamePointer {
    pub pointer_ref: String,
    pub pointer_kind: String,
    pub name: String,
    pub artifact_ref: String,
    pub previous_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub receipt_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub subject_ref: String,
    pub name: Option<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactClosure {
    pub roots: Vec<String>,
    pub closure_refs: Vec<String>,
    pub missing_refs: Vec<String>,
    pub closure_hash: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactImpact {
    pub seeds: Vec<String>,
    pub impacted_refs: Vec<String>,
    pub impact_hash: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactIndexRebuild {
    pub artifacts: usize,
    pub names: usize,
    pub receipt_value: IOValue,
}

pub fn install_artifact(root: &Path, input: &ArtifactInstallInput) -> Result<ArtifactInstall> {
    validate_install_input(input)?;
    ensure_dirs(root)?;
    let payload = prepare_install_payload(root, &input.payload)?;
    let artifact = build_install_artifact(input, &payload.payload_ref)?;
    let missing_dependencies = missing_dependencies(root, &input.dependency_refs)?;
    let decision = install_decision(&missing_dependencies);
    let refs = install_refs(input, &artifact, payload.chunk_receipt_ref.as_ref())?;
    let diagnostics = install_diagnostics(&missing_dependencies)?;
    let receipt_value = install_receipt_value(&artifact, decision, &refs, &diagnostics, &missing_dependencies)?;
    commit_install(root, &artifact, &payload.payload_bytes, &receipt_value, missing_dependencies.is_empty())?;
    Ok(ArtifactInstall {
        artifact_ref: artifact.artifact_ref.clone(),
        decision: decision.to_string(),
        artifact,
        missing_dependencies,
        receipt_value,
    })
}

struct InstallPayload {
    payload_bytes: Vec<u8>,
    payload_ref: ArtifactPayloadRef,
    chunk_receipt_ref: Option<String>,
}

fn prepare_install_payload(root: &Path, payload: &IOValue) -> Result<InstallPayload> {
    let payload_bytes = canonical_bytes(payload)?;
    let payload_value_ref = canonical_hash(payload)?;
    let (payload_ref, chunk_receipt_ref) = if payload_bytes.len() <= INLINE_PAYLOAD_LIMIT {
        (
            ArtifactPayloadRef::Inline {
                value_ref: payload_value_ref,
                length: payload_bytes.len() as u64,
            },
            None,
        )
    } else {
        let put = chunk_store::put_bytes(
            &chunk_root(root),
            "artifact-payload",
            &payload_bytes,
            chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE,
        )?;
        (
            ArtifactPayloadRef::ContentRef {
                manifest_ref: put.manifest_ref,
                length: payload_bytes.len() as u64,
            },
            Some(canonical_hash(&put.receipt_value)?),
        )
    };
    Ok(InstallPayload {
        payload_bytes,
        payload_ref,
        chunk_receipt_ref,
    })
}

fn build_install_artifact(input: &ArtifactInstallInput, payload_ref: &ArtifactPayloadRef) -> Result<ArtifactRecord> {
    let value = artifact_value(ArtifactValueInput {
        kind: &input.kind,
        payload: payload_ref,
        schema_refs: &input.schema_refs,
        dependency_refs: &input.dependency_refs,
        effect_manifest_ref: input.effect_manifest_ref.as_deref(),
        policy_refs: &input.policy_refs,
        evidence_refs: &input.evidence_refs,
    })?;
    parse_artifact_value(&value)
}

fn install_decision(missing_dependencies: &[String]) -> &'static str {
    if missing_dependencies.is_empty() {
        "pass"
    } else {
        "deny"
    }
}

fn install_refs(
    input: &ArtifactInstallInput,
    artifact: &ArtifactRecord,
    chunk_receipt_ref: Option<&String>,
) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    push_bounded(&mut refs, artifact.artifact_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    push_bounded(&mut refs, input.installer_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.capability_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.dependency_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.schema_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.policy_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    extend_cloned_bounded(&mut refs, &input.evidence_refs, MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref.as_ref() {
        push_bounded(&mut refs, effect_manifest_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    }
    if let Some(chunk_receipt_ref) = chunk_receipt_ref {
        push_bounded(&mut refs, chunk_receipt_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact install refs")?;
    }
    Ok(refs)
}

fn install_diagnostics(missing_dependencies: &[String]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for dependency in missing_dependencies {
        push_bounded(
            &mut diagnostics,
            format!("missing dependency {dependency}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact install diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn install_receipt_value(
    artifact: &ArtifactRecord,
    decision: &str,
    refs: &[String],
    diagnostics: &[String],
    missing_dependencies: &[String],
) -> Result<IOValue> {
    artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "install",
        decision,
        subject_ref: &artifact.artifact_ref,
        name: None,
        refs,
        diagnostics,
        checks: &[
            ("domain-separated-identity", "pass"),
            ("canonical-payload-ref", "pass"),
            ("dependency-closure", dependency_check(missing_dependencies)),
            ("policy-admission", "pass"),
            ("capability-admission", "pass"),
            ("names-are-metadata", "pass"),
        ],
    })
}

fn dependency_check(missing_dependencies: &[String]) -> &'static str {
    if missing_dependencies.is_empty() {
        "pass"
    } else {
        "fail"
    }
}

fn commit_install(
    root: &Path,
    artifact: &ArtifactRecord,
    payload_bytes: &[u8],
    receipt_value: &IOValue,
    should_store_artifact: bool,
) -> Result<()> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    if should_store_artifact {
        store_artifact_in_tx(&write_txn, artifact, payload_bytes)?;
    }
    store_receipt_in_tx(&write_txn, receipt_value)?;
    write_txn.commit().map_err(index_error)
}

pub fn artifact_value(input: ArtifactValueInput<'_>) -> Result<IOValue> {
    validate_kind(input.kind)?;
    validate_refs(input.schema_refs, "artifact schema ref")?;
    validate_refs(input.dependency_refs, "artifact dependency ref")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref {
        validate_ref(effect_manifest_ref, "artifact effect manifest ref")?;
    }
    validate_refs(input.policy_refs, "artifact policy ref")?;
    validate_refs(input.evidence_refs, "artifact evidence ref")?;
    Ok(record("artifact-v1", vec![
        string(ARTIFACT_SCHEMA),
        record("kind", vec![string(input.kind)]),
        record("domain", vec![string(domain_for_kind(input.kind))]),
        payload_value(input.payload)?,
        record("schemas", vec![refs_sequence(input.schema_refs)]),
        record("dependencies", vec![refs_sequence(input.dependency_refs)]),
        record("effects", vec![optional_ref_value(input.effect_manifest_ref)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
        checks_value(&[
            "domain-separated-identity",
            "canonical-payload-ref",
            "explicit-dependency-edges",
            "names-are-metadata",
            "content-addressing-is-not-trust",
        ]),
    ]))
}

pub fn parse_artifact_value(value: &IOValue) -> Result<ArtifactRecord> {
    let fields = value
        .collect_simple_record("artifact-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-v1 ...>"))?;
    require_schema(&fields[0], ARTIFACT_SCHEMA, "artifact")?;
    let kind = record_string(&fields[1], "kind")?;
    let domain = record_string(&fields[2], "domain")?;
    if domain != domain_for_kind(&kind) {
        return Err(MoltenError::invalid_harness(format!("artifact domain {domain} does not match kind {kind}")));
    }
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "domain-separated-identity", "artifact")?;
    require_check(&checks, "names-are-metadata", "artifact")?;
    Ok(ArtifactRecord {
        artifact_ref: canonical_hash(value)?,
        kind,
        domain,
        payload: parse_payload_ref(&fields[3])?,
        schema_refs: record_ref_sequence(&fields[4], "schemas")?,
        dependency_refs: record_ref_sequence(&fields[5], "dependencies")?,
        effect_manifest_ref: record_optional_ref(&fields[6], "effects")?,
        policy_refs: record_ref_sequence(&fields[7], "policy")?,
        evidence_refs: record_ref_sequence(&fields[8], "evidence")?,
        value: value.clone(),
    })
}

pub fn read_artifact(root: &Path, artifact_ref: &str) -> Result<ArtifactRecord> {
    validate_ref(artifact_ref, "artifact ref")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
    let Some(bytes) = table.get(artifact_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("artifact {artifact_ref} not found")));
    };
    let value = parse_canonical_bytes(bytes.value())?;
    let artifact = parse_artifact_value(&value)?;
    if artifact.artifact_ref != artifact_ref {
        return Err(MoltenError::invalid_harness(format!(
            "artifact registry content hash mismatch: got {}, expected {artifact_ref}",
            artifact.artifact_ref
        )));
    }
    Ok(artifact)
}

pub fn read_payload(root: &Path, artifact_ref: &str) -> Result<IOValue> {
    let artifact = read_artifact(root, artifact_ref)?;
    match &artifact.payload {
        ArtifactPayloadRef::Inline { value_ref, .. } => {
            let db = ensure_index_tables(root)?;
            let read_txn = db.begin_read().map_err(index_error)?;
            let table = read_txn.open_table(INDEX_PAYLOADS).map_err(index_error)?;
            let Some(bytes) = table.get(artifact_ref).map_err(index_error)? else {
                return Err(MoltenError::invalid_harness(format!(
                    "inline payload for artifact {artifact_ref} not found"
                )));
            };
            let value = parse_canonical_bytes(bytes.value())?;
            let actual_ref = canonical_hash(&value)?;
            if &actual_ref != value_ref {
                return Err(MoltenError::invalid_harness(format!(
                    "artifact payload hash mismatch: got {actual_ref}, expected {value_ref}"
                )));
            }
            Ok(value)
        }
        ArtifactPayloadRef::ContentRef { manifest_ref, .. } => {
            let read = chunk_store::read_object(&chunk_root(root), manifest_ref)?;
            let value = parse_canonical_bytes(&read.bytes)?;
            Ok(value)
        }
    }
}

pub fn list_artifacts(root: &Path, kind_filter: Option<&str>) -> Result<Vec<ArtifactRecord>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
    let mut artifacts = Vec::new();
    for item in table.iter().map_err(index_error)? {
        let (_key, bytes) = item.map_err(index_error)?;
        let value = parse_canonical_bytes(bytes.value())?;
        let artifact = parse_artifact_value(&value)?;
        if kind_filter.is_none_or(|kind| kind == artifact.kind) {
            push_bounded(&mut artifacts, artifact, MAX_ARTIFACT_RECORDS, "artifact registry list artifacts")?;
        }
    }
    artifacts.sort_by(|left, right| left.artifact_ref.cmp(&right.artifact_ref));
    Ok(artifacts)
}

pub fn set_name_pointer(root: &Path, input: &SetNamePointerInput<'_>) -> Result<ArtifactNamePointer> {
    validate_pointer_kind(input.pointer_kind)?;
    validate_non_empty(input.name, "artifact pointer name")?;
    validate_ref(input.artifact_ref, "artifact pointer ref")?;
    validate_refs(input.policy_refs, "artifact pointer policy ref")?;
    validate_refs(input.evidence_refs, "artifact pointer evidence ref")?;
    read_artifact(root, input.artifact_ref)?;
    let previous = read_name_pointer(root, input.pointer_kind, input.name)?.map(|pointer| pointer.artifact_ref);
    let mut refs = Vec::new();
    push_bounded(&mut refs, input.artifact_ref.to_string(), MAX_ARTIFACT_REF_LIST, "artifact name pointer refs")?;
    extend_cloned_bounded(&mut refs, input.policy_refs, MAX_ARTIFACT_REF_LIST, "artifact name pointer refs")?;
    extend_cloned_bounded(&mut refs, input.evidence_refs, MAX_ARTIFACT_REF_LIST, "artifact name pointer refs")?;
    if let Some(previous) = previous.as_ref() {
        push_bounded(&mut refs, previous.clone(), MAX_ARTIFACT_REF_LIST, "artifact name pointer refs")?;
    }
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "name-set",
        decision: "pass",
        subject_ref: input.artifact_ref,
        name: Some(input.name),
        refs: &refs,
        diagnostics: &[],
        checks: &[("names-are-metadata", "pass"), ("artifact-content-immutable", "pass")],
    })?;
    let receipt = parse_artifact_receipt(&receipt_value)?;
    let pointer = name_pointer_value(&NamePointerValueInput {
        pointer_kind: input.pointer_kind,
        name: input.name,
        artifact_ref: input.artifact_ref,
        previous_ref: previous.as_deref(),
        policy_refs: input.policy_refs,
        receipt_ref: &receipt.receipt_ref,
    })?;
    let parsed = parse_name_pointer_value(&pointer)?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        let mut names = write_txn.open_table(INDEX_NAMES).map_err(index_error)?;
        names
            .insert(name_key(input.pointer_kind, input.name)?.as_str(), canonical_bytes(&pointer)?.as_slice())
            .map_err(index_error)?;
    }
    store_receipt_in_tx(&write_txn, &receipt_value)?;
    write_txn.commit().map_err(index_error)?;
    Ok(parsed)
}

pub fn read_name_pointer(root: &Path, pointer_kind: &str, name: &str) -> Result<Option<ArtifactNamePointer>> {
    validate_pointer_kind(pointer_kind)?;
    validate_non_empty(name, "artifact pointer name")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_NAMES).map_err(index_error)?;
    let Some(bytes) = table.get(name_key(pointer_kind, name)?.as_str()).map_err(index_error)? else {
        return Ok(None);
    };
    let value = parse_canonical_bytes(bytes.value())?;
    parse_name_pointer_value(&value).map(Some)
}

pub fn direct_dependencies(root: &Path, artifact_ref: &str) -> Result<Vec<String>> {
    Ok(read_artifact(root, artifact_ref)?.dependency_refs)
}

pub fn dependency_closure(root: &Path, roots: &[String]) -> Result<ArtifactClosure> {
    let (closure_refs, missing_refs) = compute_closure_refs(root, roots)?;
    let closure_value = closure_value(roots, &closure_refs, &missing_refs)?;
    let closure_hash = canonical_hash(&closure_value)?;
    let decision = if missing_refs.is_empty() { "pass" } else { "deny" };
    let mut diagnostics = Vec::new();
    for missing in &missing_refs {
        push_bounded(
            &mut diagnostics,
            format!("missing dependency {missing}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact dependency closure diagnostics",
        )?;
    }
    let mut refs = Vec::new();
    extend_cloned_bounded(&mut refs, roots, MAX_ARTIFACT_REF_LIST, "artifact dependency closure refs")?;
    extend_cloned_bounded(&mut refs, &closure_refs, MAX_ARTIFACT_REF_LIST, "artifact dependency closure refs")?;
    extend_cloned_bounded(&mut refs, &missing_refs, MAX_ARTIFACT_REF_LIST, "artifact dependency closure refs")?;
    push_bounded(&mut refs, closure_hash.clone(), MAX_ARTIFACT_REF_LIST, "artifact dependency closure refs")?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "dependency-closure",
        decision,
        subject_ref: &closure_hash,
        name: None,
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &[
            ("dependency-closure", if missing_refs.is_empty() { "pass" } else { "fail" }),
            ("closure-hash", "pass"),
            ("ordered-refs", "pass"),
        ],
    })?;
    store_receipt(root, &receipt_value)?;
    Ok(ArtifactClosure {
        roots: sorted_unique(roots),
        closure_refs,
        missing_refs,
        closure_hash,
        receipt_value,
    })
}

pub fn impact(root: &Path, seeds: &[String]) -> Result<ArtifactImpact> {
    let impacted_refs = impact_refs(root, seeds)?;
    let impact_value = record("artifact-impact-v1", vec![
        refs_record("seeds", &sorted_unique(seeds)),
        refs_record("impacted", &impacted_refs),
    ]);
    let impact_hash = canonical_hash(&impact_value)?;
    let mut refs = sorted_unique(seeds);
    extend_cloned_bounded(&mut refs, &impacted_refs, MAX_ARTIFACT_REF_LIST, "artifact impact refs")?;
    push_bounded(&mut refs, impact_hash.clone(), MAX_ARTIFACT_REF_LIST, "artifact impact refs")?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "impact",
        decision: "pass",
        subject_ref: &impact_hash,
        name: None,
        refs: &refs,
        diagnostics: &[],
        checks: &[("reverse-dependency-impact", "pass"), ("impact-hash", "pass")],
    })?;
    store_receipt(root, &receipt_value)?;
    Ok(ArtifactImpact {
        seeds: sorted_unique(seeds),
        impacted_refs,
        impact_hash,
        receipt_value,
    })
}

pub fn impact_refs(root: &Path, seeds: &[String]) -> Result<Vec<String>> {
    validate_refs(seeds, "artifact impact seed ref")?;
    let db = ensure_index_tables(root)?;
    let mut impacted: BTreeSet<String> = seeds.iter().cloned().collect();
    let mut frontier: Vec<String> = seeds.to_vec();
    while let Some(current) = frontier.pop() {
        let dependents = {
            let read_txn = db.begin_read().map_err(index_error)?;
            let reverse = read_txn.open_table(INDEX_REVERSE).map_err(index_error)?;
            if let Some(bytes) = reverse.get(current.as_str()).map_err(index_error)? {
                parse_refs_value(&parse_canonical_bytes(bytes.value())?, "reverse")?
            } else {
                Vec::new()
            }
        };
        for dependent in dependents {
            if impacted.insert(dependent.clone()) {
                push_bounded(&mut frontier, dependent, MAX_ARTIFACT_RECORDS, "artifact impact frontier")?;
            }
        }
    }
    Ok(impacted.into_iter().collect())
}

pub fn reference_diagnostics(root: &Path, target_ref: &str) -> Result<Vec<String>> {
    validate_ref(target_ref, "artifact reference diagnostic ref")?;
    let mut diagnostics = Vec::new();
    if let Ok(impact) = impact_refs(root, &[target_ref.to_string()])
        && impact.iter().any(|reference| reference != target_ref)
    {
        push_bounded(
            &mut diagnostics,
            format!("registry reverse dependencies retain {target_ref}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact reference diagnostics",
        )?;
    }
    for pointer in all_name_pointers(root)? {
        if pointer.artifact_ref == target_ref || pointer.previous_ref.as_deref() == Some(target_ref) {
            push_bounded(
                &mut diagnostics,
                format!("registry pointer {}:{} retains {target_ref}", pointer.pointer_kind, pointer.name),
                MAX_ARTIFACT_DIAGNOSTICS,
                "artifact reference diagnostics",
            )?;
        }
    }
    if registry_text_contains_ref(root, target_ref)? {
        push_bounded(
            &mut diagnostics,
            format!("registry receipts or metadata retain {target_ref}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact reference diagnostics",
        )?;
    }
    Ok(diagnostics)
}

pub fn rebuild_index(root: &Path) -> Result<ArtifactIndexRebuild> {
    ensure_dirs(root)?;
    let artifacts = list_artifacts(root, None)?;
    let names = all_name_pointers(root)?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    clear_derived_index_tables_in_tx(&write_txn)?;
    for artifact in &artifacts {
        store_derived_indexes_in_tx(&write_txn, artifact)?;
    }
    for pointer in &names {
        let mut table = write_txn.open_table(INDEX_NAMES).map_err(index_error)?;
        table
            .insert(
                name_key(&pointer.pointer_kind, &pointer.name)?.as_str(),
                canonical_bytes(&pointer.value)?.as_slice(),
            )
            .map_err(index_error)?;
    }
    let mut refs = Vec::new();
    for artifact in &artifacts {
        push_bounded(&mut refs, artifact.artifact_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact index rebuild refs")?;
    }
    let rebuild_ref = local_ref("artifact-index-rebuild", &refs)?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "index-rebuild",
        decision: "pass",
        subject_ref: &rebuild_ref,
        name: None,
        refs: &refs,
        diagnostics: &[],
        checks: &[
            ("redb-index-artifacts", "pass"),
            ("redb-index-dependencies", "pass"),
            ("redb-index-reverse-dependencies", "pass"),
            ("redb-index-semantic", "pass"),
        ],
    })?;
    store_receipt_in_tx(&write_txn, &receipt_value)?;
    write_txn.commit().map_err(index_error)?;
    Ok(ArtifactIndexRebuild {
        artifacts: artifacts.len(),
        names: names.len(),
        receipt_value,
    })
}

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<ArtifactReceipt> {
    validate_ref(receipt_ref, "artifact receipt ref")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let receipts = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let Some(bytes) = receipts.get(receipt_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("artifact receipt {receipt_ref} not found")));
    };
    let value = parse_canonical_bytes(bytes.value())?;
    parse_artifact_receipt(&value)
}

pub fn list_receipts(root: &Path) -> Result<Vec<ArtifactReceipt>> {
    let mut receipts = Vec::new();
    for value in receipt_values(root)? {
        push_bounded(
            &mut receipts,
            parse_artifact_receipt(&value)?,
            MAX_ARTIFACT_RECEIPTS,
            "artifact registry receipts",
        )?;
    }
    receipts.sort_by(|left, right| left.receipt_ref.cmp(&right.receipt_ref));
    Ok(receipts)
}

pub fn parse_artifact_receipt(value: &IOValue) -> Result<ArtifactReceipt> {
    let fields = value
        .collect_simple_record("artifact-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-receipt-v1 ...>"))?;
    require_schema(&fields[0], ARTIFACT_RECEIPT_SCHEMA, "artifact receipt")?;
    let checks = parse_checks(&fields[7])?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("artifact receipt missing checks"));
    }
    Ok(ArtifactReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        subject_ref: record_ref(&fields[3], "subject")?,
        name: record_optional_string(&fields[4], "name")?,
        value: value.clone(),
    })
}

fn store_artifact_in_tx(
    write_txn: &redb::WriteTransaction,
    artifact: &ArtifactRecord,
    payload_bytes: &[u8],
) -> Result<()> {
    {
        let artifact_bytes = canonical_bytes(&artifact.value)?;
        let mut artifacts = write_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
        artifacts.insert(artifact.artifact_ref.as_str(), artifact_bytes.as_slice()).map_err(index_error)?;
    }
    if matches!(artifact.payload, ArtifactPayloadRef::Inline { .. }) {
        let mut payloads = write_txn.open_table(INDEX_PAYLOADS).map_err(index_error)?;
        payloads.insert(artifact.artifact_ref.as_str(), payload_bytes).map_err(index_error)?;
    }
    store_derived_indexes_in_tx(write_txn, artifact)
}

fn store_derived_indexes_in_tx(write_txn: &redb::WriteTransaction, artifact: &ArtifactRecord) -> Result<()> {
    {
        let deps_value = refs_value(&artifact.dependency_refs);
        let mut deps = write_txn.open_table(INDEX_DEPS).map_err(index_error)?;
        deps.insert(artifact.artifact_ref.as_str(), canonical_bytes(&deps_value)?.as_slice())
            .map_err(index_error)?;
    }
    for dependency_ref in &artifact.dependency_refs {
        let mut existing = {
            let reverse = write_txn.open_table(INDEX_REVERSE).map_err(index_error)?;
            if let Some(bytes) = reverse.get(dependency_ref.as_str()).map_err(index_error)? {
                parse_refs_value(&parse_canonical_bytes(bytes.value())?, "reverse")?
            } else {
                Vec::new()
            }
        };
        if !existing.iter().any(|value| value == &artifact.artifact_ref) {
            push_bounded(
                &mut existing,
                artifact.artifact_ref.clone(),
                MAX_ARTIFACT_REF_LIST,
                "artifact reverse dependency refs",
            )?;
            existing.sort();
        }
        let mut reverse = write_txn.open_table(INDEX_REVERSE).map_err(index_error)?;
        reverse
            .insert(dependency_ref.as_str(), canonical_bytes(&refs_value(&existing))?.as_slice())
            .map_err(index_error)?;
    }
    insert_str_index(write_txn, INDEX_KIND, "kind", &artifact.kind, &artifact.artifact_ref)?;
    for schema_ref in &artifact.schema_refs {
        insert_str_index(write_txn, INDEX_SCHEMA, "schema", schema_ref, &artifact.artifact_ref)?;
    }
    if let Some(effect_manifest_ref) = artifact.effect_manifest_ref.as_ref() {
        insert_str_index(write_txn, INDEX_EFFECT, "effect", effect_manifest_ref, &artifact.artifact_ref)?;
    }
    for policy_ref in &artifact.policy_refs {
        insert_str_index(write_txn, INDEX_POLICY, "policy", policy_ref, &artifact.artifact_ref)?;
    }
    for evidence_ref in &artifact.evidence_refs {
        insert_str_index(write_txn, INDEX_EVIDENCE, "evidence", evidence_ref, &artifact.artifact_ref)?;
    }
    Ok(())
}

fn insert_str_index(
    write_txn: &redb::WriteTransaction,
    table_definition: TableDefinition<&str, &str>,
    index_name: &str,
    indexed_ref: &str,
    artifact_ref: &str,
) -> Result<()> {
    let key = canonical_hash(&record("artifact-semantic-index-key", vec![
        string(index_name),
        string(indexed_ref),
        string(artifact_ref),
    ]))?;
    let mut table = write_txn.open_table(table_definition).map_err(index_error)?;
    table.insert(key.as_str(), artifact_ref).map_err(index_error)?;
    Ok(())
}

fn missing_dependencies(root: &Path, dependency_refs: &[String]) -> Result<Vec<String>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let artifacts = read_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
    let mut missing = Vec::new();
    for dependency_ref in dependency_refs {
        if artifacts.get(dependency_ref.as_str()).map_err(index_error)?.is_none() {
            push_bounded(&mut missing, dependency_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact missing dependencies")?;
        }
    }
    Ok(missing)
}

fn compute_closure_refs(root: &Path, roots: &[String]) -> Result<(Vec<String>, Vec<String>)> {
    validate_refs(roots, "artifact closure root ref")?;
    let db = ensure_index_tables(root)?;
    let mut closure = BTreeSet::new();
    let mut missing = BTreeSet::new();
    ensure_count_at_most(roots.len(), MAX_ARTIFACT_REF_LIST, "artifact closure roots")?;
    let mut stack = roots.to_vec();
    while let Some(current) = stack.pop() {
        if closure.contains(&current) || missing.contains(&current) {
            continue;
        }
        let deps = {
            let read_txn = db.begin_read().map_err(index_error)?;
            let artifacts = read_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
            if artifacts.get(current.as_str()).map_err(index_error)?.is_none() {
                checked_count_sum(missing.len(), 1, MAX_ARTIFACT_REF_LIST, "artifact closure missing refs")?;
                missing.insert(current.clone());
                Vec::new()
            } else {
                let deps = read_txn.open_table(INDEX_DEPS).map_err(index_error)?;
                if let Some(bytes) = deps.get(current.as_str()).map_err(index_error)? {
                    parse_refs_value(&parse_canonical_bytes(bytes.value())?, "dependencies")?
                } else {
                    Vec::new()
                }
            }
        };
        if !missing.contains(&current) {
            checked_count_sum(closure.len(), 1, MAX_ARTIFACT_REF_LIST, "artifact closure refs")?;
            closure.insert(current);
        }
        for dependency in deps {
            push_bounded(&mut stack, dependency, MAX_ARTIFACT_REF_LIST, "artifact closure traversal stack")?;
        }
    }
    Ok((closure.into_iter().collect(), missing.into_iter().collect()))
}

struct ArtifactReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    subject_ref: &'a str,
    name: Option<&'a str>,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct NamePointerValueInput<'a> {
    pointer_kind: &'a str,
    name: &'a str,
    artifact_ref: &'a str,
    previous_ref: Option<&'a str>,
    policy_refs: &'a [String],
    receipt_ref: &'a str,
}

fn artifact_receipt_value(input: &ArtifactReceiptValueInput<'_>) -> Result<IOValue> {
    validate_non_empty(input.operation, "artifact receipt operation")?;
    if !matches!(input.decision, "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!("unsupported artifact receipt decision {}", input.decision)));
    }
    validate_ref(input.subject_ref, "artifact receipt subject ref")?;
    validate_refs(input.refs, "artifact receipt ref")?;
    Ok(record("artifact-receipt-v1", vec![
        string(ARTIFACT_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("subject", vec![string(input.subject_ref)]),
        record("name", vec![optional_string_value(input.name)]),
        record("refs", vec![refs_sequence(input.refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(input.checks),
    ]))
}

fn name_pointer_value(input: &NamePointerValueInput<'_>) -> Result<IOValue> {
    validate_pointer_kind(input.pointer_kind)?;
    validate_non_empty(input.name, "artifact pointer name")?;
    validate_ref(input.artifact_ref, "artifact pointer artifact ref")?;
    if let Some(previous_ref) = input.previous_ref {
        validate_ref(previous_ref, "artifact pointer previous ref")?;
    }
    validate_refs(input.policy_refs, "artifact pointer policy ref")?;
    validate_ref(input.receipt_ref, "artifact pointer receipt ref")?;
    Ok(record("artifact-name-pointer-v1", vec![
        string(ARTIFACT_NAME_POINTER_SCHEMA),
        record("kind", vec![string(input.pointer_kind)]),
        record("name", vec![string(input.name)]),
        record("artifact", vec![string(input.artifact_ref)]),
        record("previous", vec![optional_ref_value(input.previous_ref)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("receipt", vec![string(input.receipt_ref)]),
        checks_value(&["names-are-metadata", "artifact-content-immutable"]),
    ]))
}

fn parse_name_pointer_value(value: &IOValue) -> Result<ArtifactNamePointer> {
    let fields = value
        .collect_simple_record("artifact-name-pointer-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-name-pointer-v1 ...>"))?;
    require_schema(&fields[0], ARTIFACT_NAME_POINTER_SCHEMA, "artifact name pointer")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "names-are-metadata", "artifact name pointer")?;
    Ok(ArtifactNamePointer {
        pointer_ref: canonical_hash(value)?,
        pointer_kind: record_string(&fields[1], "kind")?,
        name: record_string(&fields[2], "name")?,
        artifact_ref: record_ref(&fields[3], "artifact")?,
        previous_ref: record_optional_ref(&fields[4], "previous")?,
        policy_refs: record_ref_sequence(&fields[5], "policy")?,
        receipt_ref: record_ref(&fields[6], "receipt")?,
        value: value.clone(),
    })
}

pub fn list_name_pointers(root: &Path) -> Result<Vec<ArtifactNamePointer>> {
    all_name_pointers(root)
}

fn all_name_pointers(root: &Path) -> Result<Vec<ArtifactNamePointer>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let names = read_txn.open_table(INDEX_NAMES).map_err(index_error)?;
    let mut pointers = Vec::new();
    for item in names.iter().map_err(index_error)? {
        let (_key, bytes) = item.map_err(index_error)?;
        push_bounded(
            &mut pointers,
            parse_name_pointer_value(&parse_canonical_bytes(bytes.value())?)?,
            MAX_ARTIFACT_POINTERS,
            "artifact name pointers",
        )?;
    }
    Ok(pointers)
}

fn closure_value(roots: &[String], closure_refs: &[String], missing_refs: &[String]) -> Result<IOValue> {
    validate_refs(roots, "artifact closure root")?;
    validate_refs(closure_refs, "artifact closure ref")?;
    validate_refs(missing_refs, "artifact closure missing ref")?;
    Ok(record("artifact-closure-v1", vec![
        string(ARTIFACT_CLOSURE_SCHEMA),
        refs_record("roots", &sorted_unique(roots)),
        refs_record("closure", closure_refs),
        refs_record("missing", missing_refs),
        checks_value(&["ordered-refs", "closure-hash", "missing-dependency-denial"]),
    ]))
}

fn payload_value(payload: &ArtifactPayloadRef) -> Result<IOValue> {
    Ok(record("payload", vec![match payload {
        ArtifactPayloadRef::Inline { value_ref, length } => {
            validate_ref(value_ref, "inline payload value ref")?;
            record("inline", vec![string(value_ref), u64_value(*length)])
        }
        ArtifactPayloadRef::ContentRef { manifest_ref, length } => {
            validate_ref(manifest_ref, "content payload manifest ref")?;
            record("content-ref", vec![string(manifest_ref), u64_value(*length)])
        }
    }]))
}

fn parse_payload_ref(value: &Value<IOValue>) -> Result<ArtifactPayloadRef> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "payload", 1)?;
    let payload = value_to_iovalue(&fields[0]);
    if let Some(inline) = payload.collect_simple_record("inline", Some(2)) {
        return Ok(ArtifactPayloadRef::Inline {
            value_ref: required_ref(&inline[0], "inline payload ref")?,
            length: required_u64(&inline[1], "inline payload length")?,
        });
    }
    if let Some(content) = payload.collect_simple_record("content-ref", Some(2)) {
        return Ok(ArtifactPayloadRef::ContentRef {
            manifest_ref: required_ref(&content[0], "content payload manifest ref")?,
            length: required_u64(&content[1], "content payload length")?,
        });
    }
    Err(MoltenError::invalid_harness("artifact payload must be inline or content-ref"))
}

fn refs_value(refs: &[String]) -> IOValue {
    record("refs", vec![refs_sequence(refs)])
}

fn parse_refs_value(value: &IOValue, label: &str) -> Result<Vec<String>> {
    let fields = simple_record(value, "refs", 1)?;
    parse_ref_sequence_value(&fields[0], label)
}

fn refs_record(label: &'static str, refs: &[String]) -> IOValue {
    record(label, vec![refs_sequence(refs)])
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<BTreeSet<_>>().into_iter().collect()
}

fn registry_text_contains_ref(root: &Path, target_ref: &str) -> Result<bool> {
    for receipt in receipt_values(root)? {
        if to_text(&receipt)?.contains(target_ref) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn receipt_values(root: &Path) -> Result<Vec<IOValue>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let receipts = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let mut values = Vec::new();
    for item in receipts.iter().map_err(index_error)? {
        let (_key, bytes) = item.map_err(index_error)?;
        push_bounded(
            &mut values,
            parse_canonical_bytes(bytes.value())?,
            MAX_ARTIFACT_RECEIPTS,
            "artifact registry receipts",
        )?;
    }
    Ok(values)
}

fn store_receipt(root: &Path, receipt_value: &IOValue) -> Result<()> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    store_receipt_in_tx(&write_txn, receipt_value)?;
    write_txn.commit().map_err(index_error)
}

fn store_receipt_in_tx(write_txn: &redb::WriteTransaction, receipt_value: &IOValue) -> Result<()> {
    let parsed = parse_artifact_receipt(receipt_value)?;
    let mut receipts = write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    receipts
        .insert(parsed.receipt_ref.as_str(), canonical_bytes(receipt_value)?.as_slice())
        .map_err(index_error)?;
    Ok(())
}

fn clear_derived_index_tables_in_tx(write_txn: &redb::WriteTransaction) -> Result<()> {
    clear_bytes_table(write_txn, INDEX_DEPS)?;
    clear_bytes_table(write_txn, INDEX_REVERSE)?;
    clear_str_table(write_txn, INDEX_KIND)?;
    clear_str_table(write_txn, INDEX_SCHEMA)?;
    clear_str_table(write_txn, INDEX_EFFECT)?;
    clear_str_table(write_txn, INDEX_POLICY)?;
    clear_str_table(write_txn, INDEX_EVIDENCE)
}

fn clear_bytes_table(write_txn: &redb::WriteTransaction, table_definition: TableDefinition<&str, &[u8]>) -> Result<()> {
    let mut table = write_txn.open_table(table_definition).map_err(index_error)?;
    let keys = bytes_table_keys(&table)?;
    for key in keys {
        table.remove(key.as_str()).map_err(index_error)?;
    }
    Ok(())
}

fn clear_str_table(write_txn: &redb::WriteTransaction, table_definition: TableDefinition<&str, &str>) -> Result<()> {
    let mut table = write_txn.open_table(table_definition).map_err(index_error)?;
    let keys = str_table_keys(&table)?;
    for key in keys {
        table.remove(key.as_str()).map_err(index_error)?;
    }
    Ok(())
}

fn bytes_table_keys(table: &redb::Table<'_, &str, &[u8]>) -> Result<Vec<String>> {
    let mut keys = Vec::new();
    for item in table.iter().map_err(index_error)? {
        let (key, _) = item.map_err(index_error)?;
        push_bounded(&mut keys, key.value().to_string(), MAX_ARTIFACT_RECORDS, "artifact byte-table keys")?;
    }
    Ok(keys)
}

fn str_table_keys(table: &redb::Table<'_, &str, &str>) -> Result<Vec<String>> {
    let mut keys = Vec::new();
    for item in table.iter().map_err(index_error)? {
        let (key, _) = item.map_err(index_error)?;
        push_bounded(&mut keys, key.value().to_string(), MAX_ARTIFACT_RECORDS, "artifact string-table keys")?;
    }
    Ok(keys)
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
        write_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
        write_txn.open_table(INDEX_PAYLOADS).map_err(index_error)?;
        write_txn.open_table(INDEX_NAMES).map_err(index_error)?;
        write_txn.open_table(INDEX_DEPS).map_err(index_error)?;
        write_txn.open_table(INDEX_REVERSE).map_err(index_error)?;
        write_txn.open_table(INDEX_KIND).map_err(index_error)?;
        write_txn.open_table(INDEX_SCHEMA).map_err(index_error)?;
        write_txn.open_table(INDEX_EFFECT).map_err(index_error)?;
        write_txn.open_table(INDEX_POLICY).map_err(index_error)?;
        write_txn.open_table(INDEX_EVIDENCE).map_err(index_error)?;
        write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(db)
}

fn index_path(root: &Path) -> PathBuf {
    root.join(INDEX_FILE)
}

fn chunk_root(root: &Path) -> PathBuf {
    root.join("chunks")
}

fn name_key(pointer_kind: &str, name: &str) -> Result<String> {
    canonical_hash(&record("artifact-name-key", vec![string(pointer_kind), string(name)]))
}

fn local_ref(kind: &'static str, refs: &[String]) -> Result<String> {
    canonical_hash(&record(kind, vec![refs_sequence(refs)]))
}

fn domain_for_kind(kind: &str) -> String {
    format!("molten.artifacts.domain.v1:{kind}")
}

fn refs_sequence(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
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

fn record_optional_string(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_string_value(&record[0])
}

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}

fn parse_ref_sequence_value(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    ensure_count_at_most(items.len(), MAX_ARTIFACT_REF_LIST, label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut refs, required_ref(item, label)?, MAX_ARTIFACT_REF_LIST, label)?;
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
    ensure_count_at_most(items.len(), MAX_ARTIFACT_CHECKS, "artifact checks")?;
    let mut parsed = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("artifact registry check {name} has status {status}")));
        }
        push_bounded(&mut parsed, name, MAX_ARTIFACT_CHECKS, "artifact checks")?;
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
) -> Result<std::borrow::Cow<'a, preserves::Record<Value<IOValue>>>> {
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

fn required_u64(value: &Value<IOValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn validate_install_input(input: &ArtifactInstallInput) -> Result<()> {
    validate_kind(&input.kind)?;
    validate_refs(&input.schema_refs, "artifact schema ref")?;
    validate_refs(&input.dependency_refs, "artifact dependency ref")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref.as_ref() {
        validate_ref(effect_manifest_ref, "artifact effect manifest ref")?;
    }
    validate_refs(&input.policy_refs, "artifact policy ref")?;
    validate_refs(&input.evidence_refs, "artifact evidence ref")?;
    validate_ref(&input.installer_ref, "artifact installer ref")?;
    if input.capability_refs.is_empty() {
        return Err(MoltenError::invalid_harness("artifact install requires at least one capability ref"));
    }
    validate_refs(&input.capability_refs, "artifact capability ref")
}

fn validate_kind(kind: &str) -> Result<()> {
    validate_non_empty(kind, "artifact kind")?;
    if kind.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "artifact kind {kind} must use lowercase ascii, digits, '-' or '_'"
        )))
    }
}

fn validate_pointer_kind(kind: &str) -> Result<()> {
    if matches!(kind, "name" | "alias" | "tag" | "channel") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported artifact pointer kind {kind}; expected name, alias, tag, or channel"
        )))
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_ARTIFACT_REF_LIST, field)?;
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> Result<usize> {
    let total = left
        .checked_add(right)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    Ok(total)
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    checked_count_sum(values.item_count(), 1, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn extend_cloned_bounded<T: Clone>(
    values: &mut impl crate::bounded::VecSink<T>,
    incoming: &[T],
    maximum: usize,
    label: &str,
) -> Result<()> {
    let final_count = checked_count_sum(values.item_count(), incoming.len(), maximum, label)?;
    values.reserve_items(final_count.saturating_sub(values.item_count()));
    values.extend_cloned_items(incoming);
    Ok(())
}

fn index_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("artifact registry redb index error: {error}"))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use hegel::TestCase;
    use hegel::generators;

    use super::*;

    #[test]
    fn artifact_identity_is_stable_across_names_and_changes_with_payload_kind_or_deps() {
        let root = temp_dir("artifact-identity");
        let payload = record("module", vec![string("v1")]);
        let input = ArtifactInstallInput {
            kind: "steel".to_string(),
            payload: payload.clone(),
            schema_refs: vec![test_ref("schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        };
        let first = install_artifact(&root, &input).expect("install first");
        let duplicate = install_artifact(&root, &input).expect("install duplicate");
        assert_eq!(first.decision, "pass");
        assert_eq!(first.artifact_ref, duplicate.artifact_ref);
        let pointer = set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "name",
            name: "app/main",
            artifact_ref: &first.artifact_ref,
            policy_refs: &input.policy_refs,
            evidence_refs: &input.evidence_refs,
        })
        .expect("set name");
        assert_eq!(pointer.artifact_ref, first.artifact_ref);
        assert_eq!(read_payload(&root, &first.artifact_ref).expect("payload"), payload);

        let changed_payload = install_artifact(&root, &ArtifactInstallInput {
            payload: record("module", vec![string("v2")]),
            ..input.clone()
        })
        .expect("changed payload");
        assert_ne!(first.artifact_ref, changed_payload.artifact_ref);
        let changed_kind = install_artifact(&root, &ArtifactInstallInput {
            kind: "wasm".to_string(),
            ..input.clone()
        })
        .expect("changed kind");
        assert_ne!(first.artifact_ref, changed_kind.artifact_ref);
        let changed_deps = install_artifact(&root, &ArtifactInstallInput {
            dependency_refs: vec![first.artifact_ref.clone()],
            ..input
        })
        .expect("changed deps");
        assert_ne!(first.artifact_ref, changed_deps.artifact_ref);
    }

    #[test]
    fn artifact_registry_rejects_malformed_refs_and_missing_materialization() {
        let root = temp_dir("artifact-ref-shape");
        let mut input = test_input("steel", "bad-ref", &[]);
        input.schema_refs = vec!["blake3:fixture".to_string()];
        let error = install_artifact(&root, &input).expect_err("short schema ref denied");
        assert!(error.to_string().contains("canonical blake3 content ref"));

        let content_payload = ArtifactPayloadRef::ContentRef {
            manifest_ref: "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            length: 128,
        };
        let artifact_error = artifact_value(ArtifactValueInput {
            kind: "doc",
            payload: &content_payload,
            schema_refs: &[test_ref("schema")],
            dependency_refs: &[],
            effect_manifest_ref: None,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect_err("uppercase content manifest ref denied");
        assert!(artifact_error.to_string().contains("canonical blake3 content ref"));

        let missing = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let missing_error = read_artifact(&root, missing).expect_err("valid-shaped missing artifact denied");
        assert!(missing_error.to_string().contains("not found"));
    }

    #[test]
    fn artifact_registry_detects_tampered_materialized_artifact_bytes() {
        let root = temp_dir("artifact-tampered-bytes");
        let first = install_artifact(&root, &test_input("steel", "first", &[])).expect("first artifact");
        let second = install_artifact(&root, &test_input("steel", "second", &[])).expect("second artifact");
        assert_ne!(first.artifact_ref, second.artifact_ref);
        let db = ensure_index_tables(&root).expect("artifact db");
        let write_txn = db.begin_write().expect("write txn");
        {
            let mut artifacts = write_txn.open_table(INDEX_ARTIFACTS).expect("artifacts table");
            let second_bytes = canonical_bytes(&second.artifact.value).expect("second bytes");
            artifacts
                .insert(first.artifact_ref.as_str(), second_bytes.as_slice())
                .expect("tamper artifact bytes");
        }
        write_txn.commit().expect("commit tamper");
        drop(db);
        let error = read_artifact(&root, &first.artifact_ref).expect_err("tampered artifact bytes denied");
        assert!(error.to_string().contains("artifact registry content hash mismatch"), "unexpected error: {error}");
    }

    #[test]
    fn artifact_names_do_not_substitute_for_content_identity() {
        let root = temp_dir("artifact-name-not-identity");
        let first = install_artifact(&root, &test_input("steel", "first-name", &[])).expect("first artifact");
        let second = install_artifact(&root, &test_input("steel", "second-name", &[])).expect("second artifact");
        set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "name",
            name: "app/current",
            artifact_ref: &first.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("first name pointer");
        set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "name",
            name: "app/current",
            artifact_ref: &second.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("second name pointer");
        assert_eq!(
            read_payload(&root, &first.artifact_ref).expect("first payload"),
            record("payload", vec![string("first-name")])
        );
        assert_eq!(
            read_payload(&root, &second.artifact_ref).expect("second payload"),
            record("payload", vec![string("second-name")])
        );
        assert_ne!(first.artifact_ref, second.artifact_ref);
    }

    #[test]
    fn dependency_closure_impact_missing_dependencies_and_rebuild_work() {
        let root = temp_dir("artifact-deps");
        let base = install_artifact(&root, &test_input("schema", "base", &[])).expect("base");
        let dependent =
            install_artifact(&root, &test_input("steel", "dependent", std::slice::from_ref(&base.artifact_ref)))
                .expect("dependent");
        let closure = dependency_closure(&root, std::slice::from_ref(&dependent.artifact_ref)).expect("closure");
        assert_eq!(closure.missing_refs, Vec::<String>::new());
        assert!(closure.closure_refs.contains(&base.artifact_ref));
        assert!(closure.closure_refs.contains(&dependent.artifact_ref));
        let impact = impact(&root, std::slice::from_ref(&base.artifact_ref)).expect("impact");
        assert!(impact.impacted_refs.contains(&base.artifact_ref));
        assert!(impact.impacted_refs.contains(&dependent.artifact_ref));
        let missing = test_ref("missing-dep");
        let denied =
            install_artifact(&root, &test_input("steel", "bad", std::slice::from_ref(&missing))).expect("denied");
        assert_eq!(denied.decision, "deny");
        assert_eq!(denied.missing_dependencies, vec![missing]);
        let rebuild = rebuild_index(&root).expect("rebuild");
        assert!(rebuild.artifacts >= 2);
    }

    #[test]
    fn large_payloads_use_chunk_refs_and_cleanup_diagnostics_see_pointers() {
        let root = temp_dir("artifact-large");
        let large = IOValue::new("x".repeat(INLINE_PAYLOAD_LIMIT + 512));
        let installed = install_artifact(&root, &ArtifactInstallInput {
            kind: "doc".to_string(),
            payload: large.clone(),
            schema_refs: vec![test_ref("schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install large");
        assert!(matches!(installed.artifact.payload, ArtifactPayloadRef::ContentRef { .. }));
        assert_eq!(read_payload(&root, &installed.artifact_ref).expect("read payload"), large);
        set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "alias",
            name: "docs/current",
            artifact_ref: &installed.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("alias");
        let diagnostics = reference_diagnostics(&root, &installed.artifact_ref).expect("diagnostics");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.contains("pointer")));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_artifact_closure_reverse_edges_and_no_name_identity(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let root = temp_dir("artifact-hegel");
        let base = install_artifact(&root, &test_input("schema", &format!("base-{salt}"), &[])).expect("base");
        let middle = install_artifact(
            &root,
            &test_input("steel", &format!("middle-{salt}"), std::slice::from_ref(&base.artifact_ref)),
        )
        .expect("middle");
        let leaf = install_artifact(
            &root,
            &test_input("transcript", &format!("leaf-{salt}"), std::slice::from_ref(&middle.artifact_ref)),
        )
        .expect("leaf");
        let closure_one = dependency_closure(&root, std::slice::from_ref(&leaf.artifact_ref)).expect("closure one");
        let closure_two = dependency_closure(&root, std::slice::from_ref(&leaf.artifact_ref)).expect("closure two");
        assert_eq!(closure_one.closure_hash, closure_two.closure_hash);
        assert!(closure_one.closure_refs.contains(&base.artifact_ref));
        let impact_base = impact_refs(&root, std::slice::from_ref(&base.artifact_ref)).expect("impact base");
        assert!(impact_base.contains(&middle.artifact_ref));
        assert!(impact_base.contains(&leaf.artifact_ref));
        let before_name = leaf.artifact_ref.clone();
        let pointer_name = format!("app/{salt}");
        set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "name",
            name: &pointer_name,
            artifact_ref: &leaf.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("set name");
        let after_name = read_artifact(&root, &leaf.artifact_ref).expect("read after name").artifact_ref;
        assert_eq!(before_name, after_name);
    }

    fn test_input(kind: &str, label: &str, dependency_refs: &[String]) -> ArtifactInstallInput {
        ArtifactInstallInput {
            kind: kind.to_string(),
            payload: record("payload", vec![string(label)]),
            schema_refs: vec![test_ref(&format!("schema-{label}"))],
            dependency_refs: dependency_refs.to_vec(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref(&format!("policy-{label}"))],
            evidence_refs: vec![test_ref(&format!("evidence-{label}"))],
            installer_ref: test_ref(&format!("installer-{label}")),
            capability_refs: vec![test_ref(&format!("capability-{label}"))],
        }
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("artifact-test-ref", vec![string(label)])).expect("test ref")
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
