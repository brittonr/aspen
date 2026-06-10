use std::collections::BTreeSet;
use std::path::Path;

use preserves::IOValue;
use preserves::Record;
use preserves::Value;

use crate::artifacts;
use crate::artifacts::ArtifactPayloadRef;
use crate::error::MoltenError;
use crate::error::Result;
use crate::ledger;
use crate::preserves_rail::CATALOG_QUERY_SCHEMA;
use crate::preserves_rail::CATALOG_RECEIPT_SCHEMA;
use crate::preserves_rail::CATALOG_RESULT_SCHEMA;
use crate::preserves_rail::CATALOG_SHORT_ID_SCHEMA;
use crate::preserves_rail::CATALOG_SUMMARY_SCHEMA;
use crate::preserves_rail::CATALOG_VIEW_SCHEMA;
use crate::preserves_rail::PROVENANCE_RECEIPT_SCHEMA;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::content_ref_hex;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::to_text;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;

pub const TOOL_VERSION: &str = "local-artifact-catalog-v1";
pub const DEFAULT_SHORT_ID_MIN_LENGTH: usize = 8;

const MAX_CATALOG_ITEMS: usize = 100_000;
const MAX_CATALOG_REFS: usize = 4096;
const MAX_CATALOG_CHECKS: usize = 128;

const _: () = assert!(DEFAULT_SHORT_ID_MIN_LENGTH <= 64);
const _: () = assert!(MAX_CATALOG_ITEMS <= 1_000_000);
const _: () = assert!(MAX_CATALOG_REFS <= 100_000);
const _: () = assert!(MAX_CATALOG_CHECKS <= 1_000);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogVisibilityInput {
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub hidden_refs: Vec<String>,
    pub redaction_profile_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogFilter {
    Ref(String),
    ArtifactKind(String),
    LedgerKind(String),
    SchemaRef(String),
    StructuralFingerprint(String),
    EffectRef(String),
    PolicyRef(String),
    CapabilityRef(String),
    EvidenceRef(String),
    DependencyRef(String),
    DependentRef(String),
    ReceiptOperation(String),
    ReceiptDecision(String),
    TranscriptStatus(String),
    UpgradeStatus(String),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogListInput {
    pub kind: Option<String>,
    pub visibility: CatalogVisibilityInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSearchInput {
    pub root_refs: Vec<String>,
    pub include_dependencies: bool,
    pub include_dependents: bool,
    pub filters: Vec<CatalogFilter>,
    pub visibility: CatalogVisibilityInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogViewInput {
    pub reference: String,
    pub include_payload: bool,
    pub redacted: bool,
    pub visibility: CatalogVisibilityInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogGraphInput {
    pub reference: String,
    pub transitive: bool,
    pub visibility: CatalogVisibilityInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogShortIdInput {
    pub prefix: String,
    pub min_length: usize,
    pub visibility: CatalogVisibilityInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSummary {
    pub artifact_ref: String,
    pub artifact_kind: String,
    pub payload_ref: String,
    pub name_refs: Vec<String>,
    pub schema_refs: Vec<String>,
    pub dependency_refs: Vec<String>,
    pub dependent_refs: Vec<String>,
    pub effect_manifest_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub classifications: Vec<String>,
    pub visibility_decision: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogQueryResult {
    pub query_ref: String,
    pub result_ref: String,
    pub decision: String,
    pub items: Vec<IOValue>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogShortIdResolution {
    pub prefix: String,
    pub full_ref: Option<String>,
    pub candidates: Vec<String>,
    pub decision: String,
    pub value: IOValue,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub query_ref: String,
    pub result_ref: Option<String>,
    pub refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

pub fn list(registry_root: &Path, ledger_root: Option<&Path>, input: &CatalogListInput) -> Result<CatalogQueryResult> {
    validate_visibility(&input.visibility)?;
    if let Some(kind) = input.kind.as_deref() {
        validate_non_empty(kind, "catalog list kind")?;
    }
    let filters = input.kind.as_ref().map(|kind| vec![CatalogFilter::ArtifactKind(kind.clone())]).unwrap_or_default();
    let query_value = catalog_query_value(&CatalogQueryValueInput {
        operation: "list",
        root_refs: &[],
        include_dependencies: true,
        include_dependents: false,
        filters: &filters,
        visibility: &input.visibility,
        render_mode: "summary",
        include_payload: false,
    })?;
    let summaries = collect_summaries(registry_root, ledger_root, &input.visibility)?;
    let mut items = Vec::new();
    for summary in summaries {
        if input.kind.as_ref().is_none_or(|kind| &summary.artifact_kind == kind) {
            push_bounded(&mut items, summary.value, MAX_CATALOG_ITEMS, "catalog list items")?;
        }
    }
    finish_query("list", query_value, items, Vec::new())
}

pub fn search(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    input: &CatalogSearchInput,
) -> Result<CatalogQueryResult> {
    validate_visibility(&input.visibility)?;
    validate_refs(&input.root_refs, "catalog search root ref")?;
    validate_filters(&input.filters)?;
    let query_value = catalog_query_value(&CatalogQueryValueInput {
        operation: "search",
        root_refs: &input.root_refs,
        include_dependencies: input.include_dependencies,
        include_dependents: input.include_dependents,
        filters: &input.filters,
        visibility: &input.visibility,
        render_mode: "summary",
        include_payload: false,
    })?;
    let scoped = scoped_refs(registry_root, &input.root_refs, input.include_dependencies, input.include_dependents)?;
    let summaries = collect_summaries(registry_root, ledger_root, &input.visibility)?;
    let mut items = Vec::new();
    for summary in summaries {
        if !scoped.is_empty() && !scoped.contains(&summary.artifact_ref) {
            continue;
        }
        if summary_matches_filters(registry_root, ledger_root, &summary, &input.filters, &input.visibility)? {
            push_bounded(&mut items, summary.value, MAX_CATALOG_ITEMS, "catalog search items")?;
        }
    }
    finish_query("search", query_value, items, Vec::new())
}

pub fn view(registry_root: &Path, ledger_root: Option<&Path>, input: &CatalogViewInput) -> Result<CatalogQueryResult> {
    validate_visibility(&input.visibility)?;
    let full_ref = resolve_reference(registry_root, ledger_root, &input.reference, &input.visibility)?;
    let query_value = catalog_query_value(&CatalogQueryValueInput {
        operation: "view",
        root_refs: std::slice::from_ref(&full_ref),
        include_dependencies: false,
        include_dependents: false,
        filters: &[CatalogFilter::Ref(full_ref.clone())],
        visibility: &input.visibility,
        render_mode: if input.redacted { "redacted" } else { "raw" },
        include_payload: input.include_payload,
    })?;
    let item = if let Ok(artifact) = artifacts::read_artifact(registry_root, &full_ref) {
        let summary = registry_summary(registry_root, ledger_root, artifact, &input.visibility)?;
        let payload = if input.include_payload {
            let payload = artifacts::read_payload(registry_root, &full_ref)?;
            if input.redacted {
                maybe_redacted_value(&payload, input.visibility.redaction_profile_ref.as_deref())?
            } else {
                payload
            }
        } else {
            record("none", Vec::new())
        };
        catalog_view_value(&summary, &summary.value, &payload, input.include_payload, input.redacted)?
    } else if let Some(ledger_root) = ledger_root {
        let value = ledger::read_artifact(ledger_root, &full_ref)?;
        let summary = ledger_summary(registry_root, ledger_root, &full_ref, value.clone(), &input.visibility)?;
        let rendered = if input.redacted {
            maybe_redacted_value(&value, input.visibility.redaction_profile_ref.as_deref())?
        } else {
            value
        };
        catalog_view_value(&summary, &summary.value, &rendered, true, input.redacted)?
    } else {
        return Err(MoltenError::invalid_harness(format!(
            "catalog ref {full_ref} not found in registry and no ledger was supplied"
        )));
    };
    finish_query("view", query_value, vec![item], Vec::new())
}

pub fn dependencies(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    input: &CatalogGraphInput,
) -> Result<CatalogQueryResult> {
    graph_query(registry_root, ledger_root, input, "deps")
}

pub fn dependents(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    input: &CatalogGraphInput,
) -> Result<CatalogQueryResult> {
    graph_query(registry_root, ledger_root, input, "dependents")
}

pub fn receipts(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    input: &CatalogGraphInput,
) -> Result<CatalogQueryResult> {
    validate_visibility(&input.visibility)?;
    let full_ref = resolve_reference(registry_root, ledger_root, &input.reference, &input.visibility)?;
    let query_value = catalog_query_value(&CatalogQueryValueInput {
        operation: "receipts",
        root_refs: std::slice::from_ref(&full_ref),
        include_dependencies: false,
        include_dependents: false,
        filters: &[CatalogFilter::Ref(full_ref.clone())],
        visibility: &input.visibility,
        render_mode: "redacted-receipts",
        include_payload: true,
    })?;
    let mut items = Vec::new();
    append_registry_receipt_views(registry_root, &full_ref, &input.visibility, &mut items)?;
    if let Some(ledger_root) = ledger_root {
        append_ledger_receipt_views(ledger_root, &full_ref, &input.visibility, &mut items)?;
    }
    finish_query("receipts", query_value, items, Vec::new())
}

pub fn resolve_short_id(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    input: &CatalogShortIdInput,
) -> Result<CatalogShortIdResolution> {
    validate_visibility(&input.visibility)?;
    validate_non_empty(&input.prefix, "catalog short id prefix")?;
    let query_value = catalog_query_value(&CatalogQueryValueInput {
        operation: "short-id",
        root_refs: &[],
        include_dependencies: false,
        include_dependents: false,
        filters: &[CatalogFilter::Text(input.prefix.clone())],
        visibility: &input.visibility,
        render_mode: "resolution",
        include_payload: false,
    })?;
    let query_ref = canonical_hash(&query_value)?;
    let normalized = normalize_prefix(&input.prefix);
    let visible_candidates = visible_candidate_refs(registry_root, ledger_root, &input.visibility)?;
    let candidates = if is_full_ref(&input.prefix) {
        visible_candidates.into_iter().filter(|candidate| candidate == &input.prefix).collect::<Vec<_>>()
    } else if normalized.len() < input.min_length {
        Vec::new()
    } else {
        visible_candidates
            .into_iter()
            .filter(|candidate| canonical_ref_matches_prefix(candidate, &normalized))
            .collect::<Vec<_>>()
    };
    let (decision, full_ref, diagnostics) = if normalized.len() < input.min_length && !is_full_ref(&input.prefix) {
        ("deny".to_string(), None, vec![format!(
            "short id prefix requires at least {} hex characters",
            input.min_length
        )])
    } else if candidates.len() == 1 {
        ("pass".to_string(), Some(candidates[0].clone()), Vec::new())
    } else if candidates.is_empty() {
        ("deny".to_string(), None, vec!["short id prefix matched no visible refs".to_string()])
    } else {
        ("deny".to_string(), None, vec![format!(
            "short id prefix is ambiguous across {} visible refs",
            candidates.len()
        )])
    };
    let value = short_id_resolution_value(&input.prefix, full_ref.as_deref(), &candidates, &decision, &diagnostics)?;
    let result_value = catalog_result_value(&query_ref, &decision, std::slice::from_ref(&value), &diagnostics, &[
        (
            "short-id-minimum",
            if normalized.len() >= input.min_length || is_full_ref(&input.prefix) {
                "pass"
            } else {
                "fail"
            },
        ),
        ("ambiguity-denial", if candidates.len() <= 1 { "pass" } else { "fail" }),
        ("visible-candidates-only", "pass"),
    ])?;
    let result_ref = canonical_hash(&result_value)?;
    let mut refs = Vec::new();
    for candidate in &candidates {
        push_bounded(&mut refs, candidate.clone(), MAX_CATALOG_REFS, "catalog short-id refs")?;
    }
    push_bounded(&mut refs, query_ref.clone(), MAX_CATALOG_REFS, "catalog short-id refs")?;
    push_bounded(&mut refs, result_ref.clone(), MAX_CATALOG_REFS, "catalog short-id refs")?;
    let receipt_value = catalog_receipt_value(&CatalogReceiptValueInput {
        operation: "short-id",
        decision: &decision,
        query_ref: &query_ref,
        result_ref: Some(&result_ref),
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &[
            ("canonical-result-ref", "pass"),
            ("full-ref-expansion", if full_ref.is_some() { "pass" } else { "fail" }),
            ("no-name-identity", "pass"),
        ],
    })?;
    Ok(CatalogShortIdResolution {
        prefix: input.prefix.clone(),
        full_ref,
        candidates,
        decision,
        value,
        receipt_value,
    })
}

pub fn parse_catalog_receipt(value: &IOValue) -> Result<CatalogReceipt> {
    let fields = value
        .collect_simple_record("catalog-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <catalog-receipt-v1 ...>"))?;
    require_schema(&fields[0], CATALOG_RECEIPT_SCHEMA, "catalog receipt")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-receipt", "catalog receipt")?;
    Ok(CatalogReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        query_ref: record_ref(&fields[3], "query")?,
        result_ref: record_optional_ref(&fields[4], "result")?,
        refs: record_ref_sequence(&fields[5], "refs")?,
        diagnostics: record_string_sequence(&fields[6], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn catalog_summary(value: &IOValue) -> Result<String> {
    if let Ok(receipt) = parse_catalog_receipt(value) {
        return Ok(format!(
            "catalog receipt operation={} decision={} query={} result={}",
            receipt.operation,
            receipt.decision,
            receipt.query_ref,
            receipt.result_ref.as_deref().unwrap_or("<none>")
        ));
    }
    if let Some(fields) = value.collect_simple_record("catalog-result-v1", Some(6)) {
        require_schema(&fields[0], CATALOG_RESULT_SCHEMA, "catalog result")?;
        let items_value = value_to_iovalue(&fields[3]);
        let items = simple_record(&items_value, "results", 1)?;
        let count = required_sequence(&items[0], "catalog result items")?.len();
        return Ok(format!("catalog result ref={} items={count}", canonical_hash(value)?));
    }
    if value.collect_simple_record("catalog-summary-v1", Some(11)).is_some() {
        return Ok(format!("catalog summary ref={}", canonical_hash(value)?));
    }
    if value.collect_simple_record("catalog-view-v1", Some(7)).is_some() {
        return Ok(format!("catalog view ref={}", canonical_hash(value)?));
    }
    Err(MoltenError::invalid_harness("unsupported catalog artifact for show"))
}

fn graph_query(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    input: &CatalogGraphInput,
    operation: &str,
) -> Result<CatalogQueryResult> {
    validate_visibility(&input.visibility)?;
    let full_ref = resolve_reference(registry_root, ledger_root, &input.reference, &input.visibility)?;
    let query_value = catalog_query_value(&CatalogQueryValueInput {
        operation,
        root_refs: std::slice::from_ref(&full_ref),
        include_dependencies: operation == "deps" && input.transitive,
        include_dependents: operation == "dependents" && input.transitive,
        filters: &[CatalogFilter::Ref(full_ref.clone())],
        visibility: &input.visibility,
        render_mode: "summary",
        include_payload: false,
    })?;
    let refs = if operation == "deps" {
        if input.transitive {
            let closure = artifacts::dependency_closure(registry_root, std::slice::from_ref(&full_ref))?;
            closure.closure_refs.into_iter().filter(|item| item != &full_ref).collect::<Vec<_>>()
        } else {
            artifacts::direct_dependencies(registry_root, &full_ref)?
        }
    } else if input.transitive {
        artifacts::impact_refs(registry_root, std::slice::from_ref(&full_ref))?
            .into_iter()
            .filter(|item| item != &full_ref)
            .collect::<Vec<_>>()
    } else {
        direct_dependents(registry_root, &full_ref)?
    };
    let hidden = hidden_set(&input.visibility);
    let summaries = collect_summaries(registry_root, ledger_root, &input.visibility)?;
    let items = refs
        .into_iter()
        .filter(|item| !hidden.contains(item))
        .filter_map(|reference| summaries.iter().find(|summary| summary.artifact_ref == reference).cloned())
        .map(|summary| summary.value)
        .collect::<Vec<_>>();
    finish_query(operation, query_value, items, Vec::new())
}

fn append_registry_receipt_views(
    registry_root: &Path,
    subject_ref: &str,
    visibility: &CatalogVisibilityInput,
    items: &mut impl crate::bounded::VecSink<IOValue>,
) -> Result<()> {
    for receipt in artifacts::list_receipts(registry_root)? {
        if receipt.subject_ref != subject_ref && !to_text(&receipt.value)?.contains(subject_ref) {
            continue;
        }
        let text = to_text(&receipt.value)?;
        if contains_hidden_ref(&text, visibility) {
            continue;
        }
        push_bounded(
            items,
            maybe_redacted_value(&receipt.value, visibility.redaction_profile_ref.as_deref())?,
            MAX_CATALOG_ITEMS,
            "catalog receipt items",
        )?;
    }
    Ok(())
}

fn append_ledger_receipt_views(
    ledger_root: &Path,
    subject_ref: &str,
    visibility: &CatalogVisibilityInput,
    items: &mut impl crate::bounded::VecSink<IOValue>,
) -> Result<()> {
    for entry in ledger::list_artifacts(ledger_root)? {
        if hidden_set(visibility).contains(&entry.artifact_ref) {
            continue;
        }
        let value = ledger::read_artifact(ledger_root, &entry.artifact_ref)?;
        let kind = ledger::artifact_kind(&value);
        if !kind.contains("receipt") {
            continue;
        }
        let text = to_text(&value)?;
        if !text.contains(subject_ref) || contains_hidden_ref(&text, visibility) {
            continue;
        }
        push_bounded(
            items,
            maybe_redacted_value(&value, visibility.redaction_profile_ref.as_deref())?,
            MAX_CATALOG_ITEMS,
            "catalog receipt items",
        )?;
    }
    Ok(())
}

fn collect_summaries(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    visibility: &CatalogVisibilityInput,
) -> Result<Vec<CatalogSummary>> {
    let hidden = hidden_set(visibility);
    let mut summaries = Vec::new();
    let mut seen = BTreeSet::new();
    for artifact in artifacts::list_artifacts(registry_root, None)? {
        if hidden.contains(&artifact.artifact_ref) {
            continue;
        }
        checked_count_sum(seen.len(), 1, MAX_CATALOG_ITEMS, "catalog summary refs")?;
        seen.insert(artifact.artifact_ref.clone());
        push_bounded(
            &mut summaries,
            registry_summary(registry_root, ledger_root, artifact, visibility)?,
            MAX_CATALOG_ITEMS,
            "catalog summaries",
        )?;
    }
    if let Some(ledger_root) = ledger_root {
        for entry in ledger::list_artifacts(ledger_root)? {
            if seen.contains(&entry.artifact_ref) || hidden.contains(&entry.artifact_ref) {
                continue;
            }
            let value = ledger::read_artifact(ledger_root, &entry.artifact_ref)?;
            push_bounded(
                &mut summaries,
                ledger_summary(registry_root, ledger_root, &entry.artifact_ref, value, visibility)?,
                MAX_CATALOG_ITEMS,
                "catalog summaries",
            )?;
        }
    }
    summaries.sort_by(|left, right| left.artifact_ref.cmp(&right.artifact_ref));
    Ok(summaries)
}

fn registry_summary(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    artifact: artifacts::ArtifactRecord,
    visibility: &CatalogVisibilityInput,
) -> Result<CatalogSummary> {
    let payload_ref = payload_identity(&artifact.payload);
    let mut name_refs = Vec::new();
    for pointer in artifacts::list_name_pointers(registry_root)? {
        if pointer.artifact_ref == artifact.artifact_ref {
            push_bounded(&mut name_refs, pointer.pointer_ref, MAX_CATALOG_REFS, "catalog name refs")?;
        }
    }
    let dependent_refs = direct_dependents(registry_root, &artifact.artifact_ref)?;
    let mut classifications = Vec::new();
    push_bounded(&mut classifications, "registry-artifact".to_string(), MAX_CATALOG_REFS, "catalog classifications")?;
    push_bounded(
        &mut classifications,
        format!("artifact-kind:{}", artifact.kind),
        MAX_CATALOG_REFS,
        "catalog classifications",
    )?;
    if let Ok(payload) = artifacts::read_payload(registry_root, &artifact.artifact_ref) {
        for classification in known_catalog_classifications(&payload) {
            push_bounded(&mut classifications, classification, MAX_CATALOG_REFS, "catalog classifications")?;
        }
    }
    if let Some(ledger_root) = ledger_root
        && let Ok(value) = ledger::read_artifact(ledger_root, &artifact.artifact_ref)
    {
        push_bounded(
            &mut classifications,
            format!("ledger-kind:{}", ledger::artifact_kind(&value)),
            MAX_CATALOG_REFS,
            "catalog classifications",
        )?;
    }
    let value = catalog_summary_value(&CatalogSummaryValueInput {
        artifact_ref: &artifact.artifact_ref,
        artifact_kind: &artifact.kind,
        payload_ref: &payload_ref,
        name_refs: &name_refs,
        schema_refs: &artifact.schema_refs,
        dependency_refs: &artifact.dependency_refs,
        dependent_refs: &dependent_refs,
        effect_manifest_ref: artifact.effect_manifest_ref.as_deref(),
        policy_refs: &artifact.policy_refs,
        evidence_refs: &artifact.evidence_refs,
        classifications: &classifications,
        visibility_decision: "visible",
        redaction_profile_ref: visibility.redaction_profile_ref.as_deref(),
    })?;
    Ok(CatalogSummary {
        artifact_ref: artifact.artifact_ref,
        artifact_kind: artifact.kind,
        payload_ref,
        name_refs,
        schema_refs: artifact.schema_refs,
        dependency_refs: artifact.dependency_refs,
        dependent_refs,
        effect_manifest_ref: artifact.effect_manifest_ref,
        policy_refs: artifact.policy_refs,
        evidence_refs: artifact.evidence_refs,
        classifications,
        visibility_decision: "visible".to_string(),
        value,
    })
}

fn ledger_summary(
    registry_root: &Path,
    ledger_root: &Path,
    artifact_ref: &str,
    value: IOValue,
    visibility: &CatalogVisibilityInput,
) -> Result<CatalogSummary> {
    let kind = ledger::artifact_kind(&value).to_string();
    let mut classifications = Vec::new();
    push_bounded(&mut classifications, "ledger-artifact".to_string(), MAX_CATALOG_REFS, "catalog classifications")?;
    push_bounded(&mut classifications, format!("ledger-kind:{kind}"), MAX_CATALOG_REFS, "catalog classifications")?;
    for classification in known_catalog_classifications(&value) {
        push_bounded(&mut classifications, classification, MAX_CATALOG_REFS, "catalog classifications")?;
    }
    let dependent_refs = artifacts::impact_refs(registry_root, &[artifact_ref.to_string()]).unwrap_or_default();
    let mut name_refs = Vec::new();
    for pointer in artifacts::list_name_pointers(registry_root).unwrap_or_default() {
        if pointer.artifact_ref == artifact_ref {
            push_bounded(&mut name_refs, pointer.pointer_ref, MAX_CATALOG_REFS, "catalog name refs")?;
        }
    }
    let value = catalog_summary_value(&CatalogSummaryValueInput {
        artifact_ref,
        artifact_kind: &kind,
        payload_ref: artifact_ref,
        name_refs: &name_refs,
        schema_refs: &[],
        dependency_refs: &[],
        dependent_refs: &dependent_refs,
        effect_manifest_ref: None,
        policy_refs: &[],
        evidence_refs: &[],
        classifications: &classifications,
        visibility_decision: "visible",
        redaction_profile_ref: visibility.redaction_profile_ref.as_deref(),
    })?;
    let _ = ledger_root;
    Ok(CatalogSummary {
        artifact_ref: artifact_ref.to_string(),
        artifact_kind: kind,
        payload_ref: artifact_ref.to_string(),
        name_refs,
        schema_refs: Vec::new(),
        dependency_refs: Vec::new(),
        dependent_refs,
        effect_manifest_ref: None,
        policy_refs: Vec::new(),
        evidence_refs: Vec::new(),
        classifications,
        visibility_decision: "visible".to_string(),
        value,
    })
}

fn known_catalog_classifications(value: &IOValue) -> Vec<String> {
    known_catalog_classifications_result(value).unwrap_or_default()
}

fn known_catalog_classifications_result(value: &IOValue) -> Result<Vec<String>> {
    if let Ok(receipt) = artifacts::parse_artifact_receipt(value) {
        return Ok(vec![
            "artifact-receipt:registry".to_string(),
            format!("receipt-operation:{}", receipt.operation),
            format!("receipt-decision:{}", receipt.decision),
        ]);
    }
    if let Ok(receipt) = crate::transcripts::parse_transcript_run_receipt(value) {
        return Ok(vec![
            "transcript:run-receipt".to_string(),
            format!("transcript-status:{}", receipt.decision),
            format!("transcript-mode:{}", receipt.mode),
        ]);
    }
    if let Ok(profile) = crate::retention::parse_retention_class_profile(value) {
        return Ok(vec![
            "retention:class".to_string(),
            format!("retention-class:{}", profile.class_name),
            format!("retention-policies:{}", profile.policy_refs.len()),
        ]);
    }
    if let Ok(pin) = crate::retention::parse_retention_pin(value) {
        return Ok(vec![
            "retention:pin".to_string(),
            format!("retention-object:{}", pin.object_ref),
            format!("retention-class:{}", pin.retention_class),
            format!("retention-source:{}", pin.source),
        ]);
    }
    if let Ok(index) = crate::retention::parse_reference_index(value) {
        return Ok(vec![
            "retention:index".to_string(),
            format!("retention-object:{}", index.object_ref),
            format!("retention-pins:{}", index.pin_refs.len()),
            format!("retention-complete:{}", index.is_complete),
        ]);
    }
    if let Ok(plan) = crate::retention::parse_retention_gc_plan(value) {
        return Ok(vec![
            "retention-gc:plan".to_string(),
            "retention-gc-stage:plan".to_string(),
            format!("retention-gc-decision:{}", plan.decision),
            format!("retention-gc-subsystem:{}", plan.subsystem),
            format!("retention-gc-action:{}", plan.action),
            format!("retention-gc-object:{}", plan.object_ref),
            format!("retention-gc-class:{}", plan.retention_class),
            format!("retention-gc-plan:{}", plan.plan_ref),
        ]);
    }
    if let Ok(apply) = crate::retention::parse_retention_gc_apply(value) {
        let mut classifications = vec![
            "retention-gc:apply".to_string(),
            "retention-gc-stage:apply".to_string(),
            format!("retention-gc-decision:{}", apply.decision),
            format!("retention-gc-subsystem:{}", apply.subsystem),
            format!("retention-gc-action:{}", apply.action),
            format!("retention-gc-object:{}", apply.object_ref),
            format!("retention-gc-class:{}", apply.retention_class),
            format!("retention-gc-plan:{}", apply.plan_ref),
            format!("retention-gc-apply:{}", apply.apply_ref),
        ];
        push_optional_classification(
            &mut classifications,
            "retention-gc-receipt",
            apply.retention_receipt_ref.as_deref(),
        )?;
        push_optional_classification(&mut classifications, "retention-gc-tombstone", apply.tombstone_ref.as_deref())?;
        return Ok(classifications);
    }
    if let Ok(execute) = crate::retention::parse_retention_gc_execution_gate(value) {
        let mut classifications = vec![
            "retention-gc:execute".to_string(),
            "retention-gc-stage:execute".to_string(),
            format!("retention-gc-decision:{}", execute.decision),
            format!("retention-gc-subsystem:{}", execute.subsystem),
            format!("retention-gc-action:{}", execute.action),
            format!("retention-gc-object:{}", execute.object_ref),
            format!("retention-gc-class:{}", execute.retention_class),
            format!("retention-gc-execution:{}", execute.execution_ref),
        ];
        push_optional_classification(&mut classifications, "retention-gc-plan", execute.plan_ref.as_deref())?;
        push_optional_classification(&mut classifications, "retention-gc-apply", execute.apply_ref.as_deref())?;
        push_optional_classification(
            &mut classifications,
            "retention-gc-receipt",
            execute.retention_receipt_ref.as_deref(),
        )?;
        push_optional_classification(&mut classifications, "retention-gc-tombstone", execute.tombstone_ref.as_deref())?;
        return Ok(classifications);
    }
    if let Ok(audit) = crate::retention::parse_retention_gc_audit(value) {
        let mut classifications = vec![
            "retention-gc:audit".to_string(),
            "retention-gc-stage:audit".to_string(),
            format!("retention-gc-decision:{}", audit.decision),
            format!("retention-gc-subsystem:{}", audit.subsystem),
            format!("retention-gc-action:{}", audit.action),
            format!("retention-gc-object:{}", audit.object_ref),
            format!("retention-gc-class:{}", audit.retention_class),
            format!("retention-gc-execution:{}", audit.execution_ref),
        ];
        push_optional_classification(&mut classifications, "retention-gc-plan", audit.plan_ref.as_deref())?;
        push_optional_classification(&mut classifications, "retention-gc-apply", audit.apply_ref.as_deref())?;
        push_optional_classification(
            &mut classifications,
            "retention-gc-receipt",
            audit.retention_receipt_ref.as_deref(),
        )?;
        push_optional_classification(&mut classifications, "retention-gc-tombstone", audit.tombstone_ref.as_deref())?;
        return Ok(classifications);
    }
    if let Ok(explain) = crate::retention::parse_retention_candidate_explain(value) {
        let mut classifications = vec![
            "retention:explain".to_string(),
            "retention-candidate:explain".to_string(),
            format!("retention-object:{}", explain.object_ref),
            format!("retention-explain-pins:{}", explain.pin_refs.len()),
            format!("retention-explain-admissions:{}", explain.admission_refs.len()),
            format!("retention-explain-clearances:{}", explain.remote_clearance_refs.len()),
            format!("retention-explain-plans:{}", explain.gc_plan_refs.len()),
            format!("retention-explain-applies:{}", explain.gc_apply_refs.len()),
            format!("retention-explain-executes:{}", explain.gc_execution_refs.len()),
            format!("retention-explain-audits:{}", explain.gc_audit_refs.len()),
        ];
        push_optional_classification(&mut classifications, "retention-kind", explain.object_kind.as_deref())?;
        push_optional_classification(&mut classifications, "retention-class", explain.retention_class.as_deref())?;
        push_optional_classification(&mut classifications, "retention-action", explain.action.as_deref())?;
        push_optional_classification(&mut classifications, "retention-subsystem", explain.subsystem.as_deref())?;
        return Ok(classifications);
    }
    if let Ok(bundle) = crate::retention::parse_retention_candidate_bundle(value) {
        let mut classifications = vec![
            "retention:bundle".to_string(),
            "retention-candidate:bundle".to_string(),
            format!("retention-object:{}", bundle.object_ref),
            format!("retention-bundle-artifacts:{}", bundle.artifact_refs.len()),
            format!("retention-bundle-plans:{}", bundle.gc_plan_refs.len()),
            format!("retention-bundle-applies:{}", bundle.gc_apply_refs.len()),
            format!("retention-bundle-executes:{}", bundle.gc_execution_refs.len()),
            format!("retention-bundle-audits:{}", bundle.gc_audit_refs.len()),
        ];
        push_optional_classification(&mut classifications, "retention-kind", bundle.object_kind.as_deref())?;
        push_optional_classification(&mut classifications, "retention-class", bundle.retention_class.as_deref())?;
        push_optional_classification(&mut classifications, "retention-action", bundle.action.as_deref())?;
        push_optional_classification(&mut classifications, "retention-subsystem", bundle.subsystem.as_deref())?;
        return Ok(classifications);
    }
    if let Ok(profile) = crate::retention::parse_retention_candidate_bundle_profile(value) {
        return Ok(vec![
            "retention:bundle-profile".to_string(),
            "retention-candidate:bundle-profile".to_string(),
            format!("retention-bundle-profile:{}", profile.profile),
            format!("retention-bundle-decision:{}", profile.decision),
            format!("retention-bundle:{}", profile.bundle_ref),
            format!("retention-bundle-markers:{}", profile.marker_refs.len()),
        ]);
    }
    if let Ok(verify) = crate::retention::parse_retention_candidate_bundle_verify(value) {
        let mut classifications = vec![
            "retention:bundle-verify".to_string(),
            "retention-candidate:bundle-verify".to_string(),
            format!("retention-bundle-decision:{}", verify.decision),
            format!("retention-object:{}", verify.object_ref),
            format!("retention-bundle:{}", verify.bundle_ref),
            format!("retention-explain:{}", verify.explain_ref),
            format!("retention-bundle-artifacts:{}", verify.artifact_refs.len()),
            format!("retention-bundle-files:{}", verify.file_refs.len()),
        ];
        push_optional_classification(&mut classifications, "retention-kind", verify.object_kind.as_deref())?;
        push_optional_classification(&mut classifications, "retention-class", verify.retention_class.as_deref())?;
        push_optional_classification(&mut classifications, "retention-action", verify.action.as_deref())?;
        push_optional_classification(&mut classifications, "retention-subsystem", verify.subsystem.as_deref())?;
        return Ok(classifications);
    }
    if let Ok(receipt) = crate::retention::parse_retention_receipt(value) {
        return Ok(vec![
            "retention:receipt".to_string(),
            format!("retention-decision:{}", receipt.decision),
            format!("retention-action:{}", receipt.action),
            format!("retention-object:{}", receipt.object_ref),
            format!("retention-pins:{}", receipt.pin_refs.len()),
        ]);
    }
    if let Ok(tombstone) = crate::retention::parse_tombstone(value) {
        return Ok(vec![
            "retention:tombstone".to_string(),
            format!("retention-action:{}", tombstone.action),
            format!("retention-object:{}", tombstone.object_ref),
            format!("retention-class:{}", tombstone.retention_class),
        ]);
    }
    if crate::transcripts::parse_transcript_artifact(value).is_ok() {
        return Ok(vec![
            "transcript:artifact".to_string(),
            "transcript-status:document".to_string(),
        ]);
    }
    if let Ok(plan) = crate::upgrades::parse_upgrade_plan(value) {
        return Ok(vec![
            "upgrade:plan".to_string(),
            "upgrade-status:planned".to_string(),
            format!("upgrade-session:{}", plan.session_id),
        ]);
    }
    if let Some(fields) = value.collect_simple_record("upgrade-receipt-v1", Some(8)) {
        let decision = record_string(&fields[2], "decision")?;
        return Ok(vec![
            "upgrade:receipt".to_string(),
            format!("upgrade-status:{decision}"),
            format!("receipt-decision:{decision}"),
        ]);
    }
    if let Ok(record) = crate::provenance::parse_provenance_record(value) {
        return Ok(vec![
            "provenance:record".to_string(),
            format!("provenance-trust-state:{}", record.trust_state),
            format!("provenance-artifact:{}", record.artifact_ref),
            format!("provenance-build-records:{}", record.build_record_refs.len()),
        ]);
    }
    if let Ok(record) = crate::provenance::parse_provenance_build_record(value) {
        return Ok(vec![
            "provenance:build-record".to_string(),
            format!("provenance-expected-artifact:{}", record.expected_artifact_ref),
            format!("provenance-build-sources:{}", record.source_refs.len()),
            format!("provenance-build-toolchains:{}", record.toolchain_refs.len()),
        ]);
    }
    if let Ok(receipt) = crate::provenance::parse_provenance_build_verification_receipt(value) {
        return Ok(vec![
            "provenance:build-verify-receipt".to_string(),
            format!("provenance-build-decision:{}", receipt.decision),
            format!("provenance-expected-artifact:{}", receipt.expected_artifact_ref),
            format!("provenance-actual-artifact:{}", receipt.actual_artifact_ref),
            format!("receipt-decision:{}", receipt.decision),
        ]);
    }
    if let Some(fields) = value.collect_simple_record("provenance-receipt-v1", Some(10)) {
        require_schema(&fields[0], PROVENANCE_RECEIPT_SCHEMA, "provenance receipt")?;
        let decision = record_string(&fields[1], "decision")?;
        let operation = record_string(&fields[2], "operation")?;
        let profile = record_string(&fields[3], "profile")?;
        let trust_state = record_string(&fields[5], "trust-state")?;
        let build_verification_count = record_sequence_len(&fields[9], "build-verifications")?;
        return Ok(vec![
            "provenance:receipt".to_string(),
            format!("provenance-decision:{decision}"),
            format!("provenance-operation:{operation}"),
            format!("provenance-profile:{profile}"),
            format!("provenance-trust-state:{trust_state}"),
            format!("provenance-build-verifications:{build_verification_count}"),
            format!("receipt-operation:{operation}"),
            format!("receipt-decision:{decision}"),
        ]);
    }
    if let Some(fields) = value.collect_simple_record("provenance-receipt-v1", Some(9)) {
        require_schema(&fields[0], PROVENANCE_RECEIPT_SCHEMA, "provenance receipt")?;
        let decision = record_string(&fields[1], "decision")?;
        let operation = record_string(&fields[2], "operation")?;
        let profile = record_string(&fields[3], "profile")?;
        let trust_state = record_string(&fields[5], "trust-state")?;
        return Ok(vec![
            "provenance:receipt".to_string(),
            format!("provenance-decision:{decision}"),
            format!("provenance-operation:{operation}"),
            format!("provenance-profile:{profile}"),
            format!("provenance-trust-state:{trust_state}"),
            format!("receipt-operation:{operation}"),
            format!("receipt-decision:{decision}"),
        ]);
    }
    if let Some(fields) = value.collect_simple_record("octet-structured-findings-v1", Some(7)) {
        let counts = value_to_iovalue(&fields[4]);
        let count_fields = simple_record(&counts, "counts", 4)?;
        let total = record_u64(&count_fields[0], "total")?;
        let parsed = record_u64(&count_fields[1], "parsed")?;
        let unkeyed = record_u64(&count_fields[2], "unkeyed")?;
        let critical = record_u64(&count_fields[3], "critical")?;
        return Ok(vec![
            "octet-structured-findings:summary-index".to_string(),
            format!("octet-findings-total:{total}"),
            format!("octet-findings-parsed:{parsed}"),
            format!("octet-findings-unkeyed:{unkeyed}"),
            format!("octet-findings-critical:{critical}"),
        ]);
    }
    if let Some(fields) = value.collect_simple_record("octet-fingerprint-evidence-v1", Some(7)) {
        let source_paths = record_sequence_len(&fields[3], "source-paths")?;
        let object_count = record_u64(&fields[4], "object-count")?;
        let pure_cache_blocked = record_u64(&fields[5], "pure-cache-blocked")?;
        return Ok(vec![
            "octet-fingerprint-evidence:object-corpus".to_string(),
            format!("octet-fingerprint-source-paths:{source_paths}"),
            format!("octet-fingerprint-object-count:{object_count}"),
            format!("octet-fingerprint-pure-cache-blocked:{pure_cache_blocked}"),
        ]);
    }
    if let Some(fields) = value.collect_simple_record("octet-warning-baseline-v1", Some(14)) {
        let expires_at = record_string(&fields[3], "expires-at")?;
        let finding_count = record_sequence_len(&fields[8], "finding-keys")?;
        let critical_count = record_sequence_len(&fields[9], "critical-finding-keys")?;
        let review_refs = record_string_sequence(&fields[12], "review-refs")?;
        let burn_down = value_to_iovalue(&fields[11]);
        let burn_down_fields = simple_record(&burn_down, "burn-down", 3)?;
        let total = record_u64(&burn_down_fields[0], "total")?;
        let target_next = record_u64(&burn_down_fields[1], "target-next")?;
        let deadline = record_string(&burn_down_fields[2], "deadline")?;
        let mut classifications = vec![
            "octet-baseline:warning-quarantine".to_string(),
            format!("octet-baseline-findings:{finding_count}"),
            format!("octet-baseline-critical:{critical_count}"),
            format!("octet-baseline-expires-at:{expires_at}"),
            format!("octet-baseline-burn-down-total:{total}"),
            format!("octet-baseline-burn-down-target-next:{target_next}"),
            format!("octet-baseline-burn-down-deadline:{deadline}"),
        ];
        ensure_count_at_most(classifications.len(), MAX_CATALOG_REFS, "catalog octet classifications")?;
        for review_ref in &review_refs {
            push_bounded(
                &mut classifications,
                format!("octet-review-ref:{review_ref}"),
                MAX_CATALOG_REFS,
                "catalog octet classifications",
            )?;
        }
        return Ok(classifications);
    }
    if let Some(fields) = value.collect_simple_record("octet-baseline-receipt-v1", Some(12)) {
        let decision = record_string(&fields[1], "decision")?;
        let new_count = record_sequence_len(&fields[4], "new-findings")?;
        let removed_count = record_sequence_len(&fields[5], "removed-findings")?;
        let unchanged_count = record_sequence_len(&fields[6], "unchanged-findings")?;
        let critical_unreviewed = record_sequence_len(&fields[7], "critical-unreviewed")?;
        let review_refs = record_string_sequence(&fields[8], "review-refs")?;
        let mut classifications = vec![
            "octet-baseline-receipt:quarantine-check".to_string(),
            format!("octet-baseline-decision:{decision}"),
            format!("octet-baseline-new-findings:{new_count}"),
            format!("octet-baseline-removed-findings:{removed_count}"),
            format!("octet-baseline-unchanged-findings:{unchanged_count}"),
            format!("octet-baseline-critical-unreviewed:{critical_unreviewed}"),
        ];
        ensure_count_at_most(classifications.len(), MAX_CATALOG_REFS, "catalog octet classifications")?;
        for review_ref in &review_refs {
            push_bounded(
                &mut classifications,
                format!("octet-review-ref:{review_ref}"),
                MAX_CATALOG_REFS,
                "catalog octet classifications",
            )?;
        }
        return Ok(classifications);
    }
    if let Some(fields) = value.collect_simple_record("octet-review-manifest-v1", Some(6)) {
        let profile = record_string(&fields[1], "profile")?;
        let expires_at = record_string(&fields[2], "expires-at")?;
        let finding_count = record_sequence_len(&fields[3], "finding-keys")?;
        return Ok(vec![
            "octet-review-manifest:critical-finding-review".to_string(),
            format!("octet-review-profile:{profile}"),
            format!("octet-review-expires-at:{expires_at}"),
            format!("octet-review-finding-count:{finding_count}"),
        ]);
    }
    if let Some(fields) = value.collect_simple_record("octet-gate-policy-v1", Some(8)) {
        let profile = record_string(&fields[1], "profile")?;
        let required_artifacts = record_sequence_len(&fields[3], "required-artifacts")?;
        let critical_lints = record_sequence_len(&fields[5], "critical-lints")?;
        return Ok(vec![
            "octet-gate-policy:strict-source-gate".to_string(),
            format!("octet-gate-profile:{profile}"),
            format!("octet-gate-required-artifacts:{required_artifacts}"),
            format!("octet-gate-critical-lints:{critical_lints}"),
        ]);
    }
    if let Some(fields) = value.collect_simple_record("octet-gate-receipt-v1", Some(15)) {
        let decision = record_string(&fields[1], "decision")?;
        let counts = value_to_iovalue(&fields[12]);
        let count_fields = simple_record(&counts, "counts", 6)?;
        let findings = record_u64(&count_fields[0], "findings")?;
        let warnings = record_u64(&count_fields[1], "warnings")?;
        let errors = record_u64(&count_fields[2], "errors")?;
        let critical = record_u64(&count_fields[4], "critical")?;
        return Ok(vec![
            "octet-gate-receipt:strict-source-gate".to_string(),
            format!("octet-gate-decision:{decision}"),
            format!("octet-gate-findings:{findings}"),
            format!("octet-gate-warnings:{warnings}"),
            format!("octet-gate-errors:{errors}"),
            format!("octet-gate-critical:{critical}"),
        ]);
    }
    if let Some(fields) = value.collect_simple_record("octet-source-gate-requirement-v1", Some(10)) {
        let consumer = record_string(&fields[1], "consumer")?;
        let source_scope = record_sequence_len(&fields[4], "source-scope")?;
        return Ok(vec![
            "octet-source-gate-requirement:downstream-consumer".to_string(),
            format!("octet-source-gate-consumer:{consumer}"),
            format!("octet-source-gate-scope-paths:{source_scope}"),
        ]);
    }
    if let Some(fields) = value.collect_simple_record("octet-source-gate-validation-v1", Some(13)) {
        let decision = record_string(&fields[1], "decision")?;
        let counts = value_to_iovalue(&fields[10]);
        let count_fields = simple_record(&counts, "counts", 6)?;
        let findings = record_u64(&count_fields[0], "findings")?;
        let critical = record_u64(&count_fields[4], "critical")?;
        return Ok(vec![
            "octet-source-gate-validation:strict-receipt-content".to_string(),
            format!("octet-source-gate-decision:{decision}"),
            format!("octet-source-gate-findings:{findings}"),
            format!("octet-source-gate-critical:{critical}"),
        ]);
    }
    Ok(Vec::new())
}

fn summary_matches_filters(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    summary: &CatalogSummary,
    filters: &[CatalogFilter],
    visibility: &CatalogVisibilityInput,
) -> Result<bool> {
    if filters.is_empty() {
        return Ok(true);
    }
    let public_text = summary_public_text(registry_root, ledger_root, summary, visibility)?;
    for filter in filters {
        let has_matching_filter = match filter {
            CatalogFilter::Ref(value_ref) => &summary.artifact_ref == value_ref || public_text.contains(value_ref),
            CatalogFilter::ArtifactKind(kind) => &summary.artifact_kind == kind,
            CatalogFilter::LedgerKind(kind) => {
                summary.classifications.iter().any(|item| item == &format!("ledger-kind:{kind}"))
            }
            CatalogFilter::SchemaRef(value_ref) => summary.schema_refs.contains(value_ref),
            CatalogFilter::StructuralFingerprint(value_ref) => public_text.contains(value_ref),
            CatalogFilter::EffectRef(value_ref) => summary.effect_manifest_ref.as_deref() == Some(value_ref.as_str()),
            CatalogFilter::PolicyRef(value_ref) => {
                summary.policy_refs.contains(value_ref) || public_text.contains(value_ref)
            }
            CatalogFilter::CapabilityRef(value_ref) => public_text.contains(value_ref),
            CatalogFilter::EvidenceRef(value_ref) => {
                summary.evidence_refs.contains(value_ref) || public_text.contains(value_ref)
            }
            CatalogFilter::DependencyRef(value_ref) => summary.dependency_refs.contains(value_ref),
            CatalogFilter::DependentRef(value_ref) => summary.dependent_refs.contains(value_ref),
            CatalogFilter::ReceiptOperation(operation) => {
                receipt_field_matches(&public_text, "operation", operation)
                    || public_text.contains(&format!("receipt-operation:{operation}"))
            }
            CatalogFilter::ReceiptDecision(decision) => {
                receipt_field_matches(&public_text, "decision", decision)
                    || public_text.contains(&format!("receipt-decision:{decision}"))
            }
            CatalogFilter::TranscriptStatus(status) => public_text.contains(&format!("transcript-status:{status}")),
            CatalogFilter::UpgradeStatus(status) => public_text.contains(&format!("upgrade-status:{status}")),
            CatalogFilter::Text(term) => !term.is_empty() && public_text.contains(term),
        };
        if !has_matching_filter {
            return Ok(false);
        }
    }
    Ok(true)
}

fn summary_public_text(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    summary: &CatalogSummary,
    visibility: &CatalogVisibilityInput,
) -> Result<String> {
    let mut parts = Vec::new();
    push_bounded(&mut parts, to_text(&summary.value)?, MAX_CATALOG_ITEMS, "catalog public text parts")?;
    if let Ok(artifact) = artifacts::read_artifact(registry_root, &summary.artifact_ref) {
        push_bounded(&mut parts, to_text(&artifact.value)?, MAX_CATALOG_ITEMS, "catalog public text parts")?;
        let payload = artifacts::read_payload(registry_root, &summary.artifact_ref)?;
        push_bounded(
            &mut parts,
            to_text(&maybe_redacted_value(&payload, visibility.redaction_profile_ref.as_deref())?)?,
            MAX_CATALOG_ITEMS,
            "catalog public text parts",
        )?;
    } else if let Some(ledger_root) = ledger_root
        && let Ok(value) = ledger::read_artifact(ledger_root, &summary.artifact_ref)
    {
        push_bounded(
            &mut parts,
            to_text(&maybe_redacted_value(&value, visibility.redaction_profile_ref.as_deref())?)?,
            MAX_CATALOG_ITEMS,
            "catalog public text parts",
        )?;
    }
    Ok(parts.join("\n"))
}

fn direct_dependents(registry_root: &Path, artifact_ref: &str) -> Result<Vec<String>> {
    validate_ref(artifact_ref, "catalog dependent ref")?;
    let mut dependents = Vec::new();
    for artifact in artifacts::list_artifacts(registry_root, None)? {
        if artifact.dependency_refs.iter().any(|dependency| dependency == artifact_ref) {
            push_bounded(&mut dependents, artifact.artifact_ref, MAX_CATALOG_REFS, "catalog dependents")?;
        }
    }
    dependents.sort();
    Ok(dependents)
}

fn scoped_refs(
    registry_root: &Path,
    root_refs: &[String],
    include_dependencies: bool,
    include_dependents: bool,
) -> Result<BTreeSet<String>> {
    validate_refs(root_refs, "catalog scope ref")?;
    let mut scoped = BTreeSet::new();
    ensure_count_at_most(root_refs.len(), MAX_CATALOG_REFS, "catalog scope roots")?;
    let mut frontier = root_refs.to_vec();
    while let Some(current) = frontier.pop() {
        if scoped.contains(&current) {
            continue;
        }
        insert_bounded(&mut scoped, current.clone(), MAX_CATALOG_REFS, "catalog scoped refs")?;
        if include_dependencies && let Ok(deps) = artifacts::direct_dependencies(registry_root, &current) {
            for dependency in deps {
                push_bounded(&mut frontier, dependency, MAX_CATALOG_REFS, "catalog scope frontier")?;
            }
        }
        if include_dependents && let Ok(dependents) = direct_dependents(registry_root, &current) {
            for dependent in dependents {
                push_bounded(&mut frontier, dependent, MAX_CATALOG_REFS, "catalog scope frontier")?;
            }
        }
    }
    Ok(scoped)
}

fn resolve_reference(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    reference: &str,
    visibility: &CatalogVisibilityInput,
) -> Result<String> {
    if is_full_ref(reference) {
        if hidden_set(visibility).contains(reference) {
            return Err(MoltenError::invalid_harness(format!("catalog ref {reference} is hidden")));
        }
        return Ok(reference.to_string());
    }
    let resolution = resolve_short_id(registry_root, ledger_root, &CatalogShortIdInput {
        prefix: reference.to_string(),
        min_length: DEFAULT_SHORT_ID_MIN_LENGTH,
        visibility: visibility.clone(),
    })?;
    resolution
        .full_ref
        .ok_or_else(|| MoltenError::invalid_harness(format!("short id {} did not resolve", reference)))
}

fn visible_candidate_refs(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    visibility: &CatalogVisibilityInput,
) -> Result<Vec<String>> {
    let hidden = hidden_set(visibility);
    let mut candidates = BTreeSet::new();
    for artifact in artifacts::list_artifacts(registry_root, None)? {
        if !hidden.contains(&artifact.artifact_ref) {
            insert_bounded(&mut candidates, artifact.artifact_ref, MAX_CATALOG_REFS, "catalog visible candidates")?;
        }
    }
    if let Some(ledger_root) = ledger_root {
        for entry in ledger::list_artifacts(ledger_root)? {
            if !hidden.contains(&entry.artifact_ref) {
                insert_bounded(&mut candidates, entry.artifact_ref, MAX_CATALOG_REFS, "catalog visible candidates")?;
            }
        }
    }
    let mut candidate_refs = Vec::new();
    for candidate in candidates {
        push_bounded(&mut candidate_refs, candidate, MAX_CATALOG_REFS, "catalog visible candidates")?;
    }
    Ok(candidate_refs)
}

fn finish_query(
    operation: &str,
    query_value: IOValue,
    items: Vec<IOValue>,
    diagnostics: Vec<String>,
) -> Result<CatalogQueryResult> {
    let query_ref = canonical_hash(&query_value)?;
    let decision = "pass";
    let result_value = catalog_result_value(&query_ref, decision, &items, &diagnostics, &[
        ("visibility-filtered", "pass"),
        ("canonical-result-ref", "pass"),
        ("no-name-identity", "pass"),
    ])?;
    let result_ref = canonical_hash(&result_value)?;
    let mut refs = Vec::new();
    push_bounded(&mut refs, query_ref.clone(), MAX_CATALOG_REFS, "catalog receipt refs")?;
    push_bounded(&mut refs, result_ref.clone(), MAX_CATALOG_REFS, "catalog receipt refs")?;
    for item in &items {
        push_bounded(&mut refs, canonical_hash(item)?, MAX_CATALOG_REFS, "catalog receipt refs")?;
    }
    let receipt_value = catalog_receipt_value(&CatalogReceiptValueInput {
        operation,
        decision,
        query_ref: &query_ref,
        result_ref: Some(&result_ref),
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &[
            ("canonical-result-ref", "pass"),
            ("visibility-filtered", "pass"),
            ("no-name-identity", "pass"),
        ],
    })?;
    Ok(CatalogQueryResult {
        query_ref,
        result_ref,
        decision: decision.to_string(),
        items,
        diagnostics,
        value: result_value,
        receipt_value,
    })
}

struct CatalogSummaryValueInput<'a> {
    artifact_ref: &'a str,
    artifact_kind: &'a str,
    payload_ref: &'a str,
    name_refs: &'a [String],
    schema_refs: &'a [String],
    dependency_refs: &'a [String],
    dependent_refs: &'a [String],
    effect_manifest_ref: Option<&'a str>,
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
    classifications: &'a [String],
    visibility_decision: &'a str,
    redaction_profile_ref: Option<&'a str>,
}

struct CatalogQueryValueInput<'a> {
    operation: &'a str,
    root_refs: &'a [String],
    include_dependencies: bool,
    include_dependents: bool,
    filters: &'a [CatalogFilter],
    visibility: &'a CatalogVisibilityInput,
    render_mode: &'a str,
    include_payload: bool,
}

struct CatalogReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    query_ref: &'a str,
    result_ref: Option<&'a str>,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

fn catalog_summary_value(input: &CatalogSummaryValueInput<'_>) -> Result<IOValue> {
    validate_ref(input.artifact_ref, "catalog artifact ref")?;
    validate_non_empty(input.artifact_kind, "catalog artifact kind")?;
    validate_ref(input.payload_ref, "catalog payload ref")?;
    validate_refs(input.name_refs, "catalog name ref")?;
    validate_refs(input.schema_refs, "catalog schema ref")?;
    validate_refs(input.dependency_refs, "catalog dependency ref")?;
    validate_refs(input.dependent_refs, "catalog dependent ref")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref {
        validate_ref(effect_manifest_ref, "catalog effect ref")?;
    }
    validate_refs(input.policy_refs, "catalog policy ref")?;
    validate_refs(input.evidence_refs, "catalog evidence ref")?;
    Ok(record("catalog-summary-v1", vec![
        string(CATALOG_SUMMARY_SCHEMA),
        record("artifact", vec![
            string(input.artifact_ref),
            string(input.artifact_kind),
            string(input.payload_ref),
        ]),
        record("names", vec![refs_sequence(input.name_refs)]),
        record("schemas", vec![refs_sequence(input.schema_refs)]),
        record("dependencies", vec![refs_sequence(input.dependency_refs)]),
        record("dependents", vec![refs_sequence(input.dependent_refs)]),
        record("effects", vec![optional_ref_value(input.effect_manifest_ref)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
        record("classifications", vec![sequence(input.classifications.iter().map(string).collect())]),
        record("visibility", vec![
            string(input.visibility_decision),
            optional_ref_value(input.redaction_profile_ref),
        ]),
        checks_value(&[
            "full-ref-identity",
            "names-are-metadata",
            "visibility-filtered",
            "redaction-profile-bound",
        ]),
    ]))
}

fn catalog_view_value(
    summary: &CatalogSummary,
    summary_value: &IOValue,
    payload_or_value: &IOValue,
    include_payload: bool,
    redacted: bool,
) -> Result<IOValue> {
    Ok(record("catalog-view-v1", vec![
        string(CATALOG_VIEW_SCHEMA),
        record("artifact", vec![string(&summary.artifact_ref), string(&summary.artifact_kind)]),
        record("summary", vec![summary_value.clone()]),
        record("content", vec![payload_or_value.clone()]),
        record("render", vec![
            bool_value(include_payload),
            string(if redacted { "redacted" } else { "raw" }),
        ]),
        record("classifications", vec![sequence(summary.classifications.iter().map(string).collect())]),
        checks_value(&["full-ref-identity", "redacted-before-render", "no-name-identity"]),
    ]))
}

fn catalog_query_value(input: &CatalogQueryValueInput<'_>) -> Result<IOValue> {
    validate_non_empty(input.operation, "catalog operation")?;
    validate_refs(input.root_refs, "catalog query root ref")?;
    validate_filters(input.filters)?;
    validate_visibility(input.visibility)?;
    Ok(record("catalog-query-v1", vec![
        string(CATALOG_QUERY_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("scope", vec![
            refs_sequence(input.root_refs),
            bool_value(input.include_dependencies),
            bool_value(input.include_dependents),
        ]),
        record("filters", vec![sequence(
            input.filters.iter().map(filter_value).collect::<Result<Vec<_>>>()?,
        )]),
        record("visibility", vec![
            refs_sequence(&input.visibility.policy_refs),
            refs_sequence(&input.visibility.capability_refs),
            refs_sequence(&input.visibility.hidden_refs),
            optional_ref_value(input.visibility.redaction_profile_ref.as_deref()),
        ]),
        record("render", vec![string(input.render_mode), bool_value(input.include_payload)]),
        checks_value(&[
            "no-name-identity",
            "visibility-filtered",
            "bounded-query",
            "short-id-ui-only",
        ]),
    ]))
}

fn catalog_result_value(
    query_ref: &str,
    decision: &str,
    items: &[IOValue],
    diagnostics: &[String],
    checks: &[(&str, &str)],
) -> Result<IOValue> {
    validate_ref(query_ref, "catalog result query ref")?;
    validate_decision(decision)?;
    Ok(record("catalog-result-v1", vec![
        string(CATALOG_RESULT_SCHEMA),
        record("query", vec![string(query_ref)]),
        record("decision", vec![string(decision)]),
        record("results", vec![sequence(items.to_vec())]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(checks),
    ]))
}

fn catalog_receipt_value(input: &CatalogReceiptValueInput<'_>) -> Result<IOValue> {
    validate_non_empty(input.operation, "catalog receipt operation")?;
    validate_decision(input.decision)?;
    validate_ref(input.query_ref, "catalog receipt query ref")?;
    if let Some(result_ref) = input.result_ref {
        validate_ref(result_ref, "catalog receipt result ref")?;
    }
    validate_refs(input.refs, "catalog receipt ref")?;
    ensure_count_at_most(input.checks.len(), MAX_CATALOG_CHECKS, "catalog receipt checks")?;
    let mut all_checks = Vec::new();
    push_bounded(&mut all_checks, ("canonical-receipt", "pass"), MAX_CATALOG_CHECKS, "catalog receipt checks")?;
    for check in input.checks {
        push_bounded(&mut all_checks, *check, MAX_CATALOG_CHECKS, "catalog receipt checks")?;
    }
    Ok(record("catalog-receipt-v1", vec![
        string(CATALOG_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("query", vec![string(input.query_ref)]),
        record("result", vec![optional_ref_value(input.result_ref)]),
        record("refs", vec![refs_sequence(&sorted_unique(input.refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(&all_checks),
    ]))
}

fn short_id_resolution_value(
    prefix: &str,
    full_ref: Option<&str>,
    candidates: &[String],
    decision: &str,
    diagnostics: &[String],
) -> Result<IOValue> {
    validate_non_empty(prefix, "catalog short id prefix")?;
    if let Some(full_ref) = full_ref {
        validate_ref(full_ref, "catalog short id full ref")?;
    }
    validate_refs(candidates, "catalog short id candidate ref")?;
    validate_decision(decision)?;
    Ok(record("short-id-resolution-v1", vec![
        string(CATALOG_SHORT_ID_SCHEMA),
        record("prefix", vec![string(prefix)]),
        record("full-ref", vec![optional_ref_value(full_ref)]),
        record("candidates", vec![refs_sequence(candidates)]),
        record("candidate-count", vec![crate::preserves_rail::u64_value(candidates.len() as u64)]),
        record("decision", vec![string(decision)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(&["short-id-ui-only", "visible-candidates-only", "ambiguity-denial"]),
    ]))
}

fn filter_value(filter: &CatalogFilter) -> Result<IOValue> {
    let (kind, value) = match filter {
        CatalogFilter::Ref(value) => ("ref", value.as_str()),
        CatalogFilter::ArtifactKind(value) => ("artifact-kind", value.as_str()),
        CatalogFilter::LedgerKind(value) => ("ledger-kind", value.as_str()),
        CatalogFilter::SchemaRef(value) => ("schema-ref", value.as_str()),
        CatalogFilter::StructuralFingerprint(value) => ("structural-fingerprint", value.as_str()),
        CatalogFilter::EffectRef(value) => ("effect-ref", value.as_str()),
        CatalogFilter::PolicyRef(value) => ("policy-ref", value.as_str()),
        CatalogFilter::CapabilityRef(value) => ("capability-ref", value.as_str()),
        CatalogFilter::EvidenceRef(value) => ("evidence-ref", value.as_str()),
        CatalogFilter::DependencyRef(value) => ("dependency-ref", value.as_str()),
        CatalogFilter::DependentRef(value) => ("dependent-ref", value.as_str()),
        CatalogFilter::ReceiptOperation(value) => ("receipt-operation", value.as_str()),
        CatalogFilter::ReceiptDecision(value) => ("receipt-decision", value.as_str()),
        CatalogFilter::TranscriptStatus(value) => ("transcript-status", value.as_str()),
        CatalogFilter::UpgradeStatus(value) => ("upgrade-status", value.as_str()),
        CatalogFilter::Text(value) => ("text", value.as_str()),
    };
    Ok(record("filter", vec![string(kind), string(value)]))
}

fn maybe_redacted_value(value: &IOValue, redaction_profile_ref: Option<&str>) -> Result<IOValue> {
    crate::secrets::redacted_value(value, redaction_profile_ref)
}

fn payload_identity(payload: &ArtifactPayloadRef) -> String {
    match payload {
        ArtifactPayloadRef::Inline { value_ref, .. } => value_ref.clone(),
        ArtifactPayloadRef::ContentRef { manifest_ref, .. } => manifest_ref.clone(),
    }
}

fn receipt_field_matches(text: &str, field: &str, value: &str) -> bool {
    text.contains(&format!("<{field} \"{value}\">")) || text.contains(&format!("<{field} {value}"))
}

fn hidden_set(visibility: &CatalogVisibilityInput) -> BTreeSet<String> {
    visibility.hidden_refs.iter().cloned().collect()
}

fn contains_hidden_ref(text: &str, visibility: &CatalogVisibilityInput) -> bool {
    visibility.hidden_refs.iter().any(|hidden_ref| text.contains(hidden_ref))
}

fn is_full_ref(value: &str) -> bool {
    validate_content_ref(value).is_ok()
}

fn normalize_prefix(prefix: &str) -> String {
    prefix.to_ascii_lowercase()
}

fn canonical_ref_matches_prefix(candidate: &str, normalized_prefix: &str) -> bool {
    content_ref_hex(candidate).is_ok_and(|hex| hex.starts_with(normalized_prefix))
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
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
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
    let items = required_sequence(&checks[0], "catalog checks")?;
    ensure_count_at_most(items.len(), MAX_CATALOG_CHECKS, "catalog checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "catalog check name")?;
        let status = required_string(&check[1], "catalog check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("catalog check {name} has status {status}")));
        }
        push_bounded(&mut parsed, name, MAX_CATALOG_CHECKS, "catalog checks")?;
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

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_ref(&fields[0], label)
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&fields[0])
}

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&fields[0], label)
}

fn record_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    items.iter().map(|item| required_string(item, label)).collect()
}

fn record_sequence_len(value: &Value<IOValue>, label: &str) -> Result<usize> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    Ok(required_sequence(&fields[0], label)?.len())
}

fn record_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_u64(&fields[0], label)
}

fn parse_ref_sequence_value(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    items.iter().map(|item| required_ref(item, label)).collect()
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

fn validate_filters(filters: &[CatalogFilter]) -> Result<()> {
    for filter in filters {
        match filter {
            CatalogFilter::Ref(value)
            | CatalogFilter::SchemaRef(value)
            | CatalogFilter::StructuralFingerprint(value)
            | CatalogFilter::EffectRef(value)
            | CatalogFilter::PolicyRef(value)
            | CatalogFilter::CapabilityRef(value)
            | CatalogFilter::EvidenceRef(value)
            | CatalogFilter::DependencyRef(value)
            | CatalogFilter::DependentRef(value) => validate_ref(value, "catalog filter ref")?,
            CatalogFilter::ArtifactKind(value)
            | CatalogFilter::LedgerKind(value)
            | CatalogFilter::ReceiptOperation(value)
            | CatalogFilter::ReceiptDecision(value)
            | CatalogFilter::TranscriptStatus(value)
            | CatalogFilter::UpgradeStatus(value)
            | CatalogFilter::Text(value) => validate_non_empty(value, "catalog filter value")?,
        }
    }
    Ok(())
}

fn validate_visibility(visibility: &CatalogVisibilityInput) -> Result<()> {
    validate_refs(&visibility.policy_refs, "catalog visibility policy ref")?;
    validate_refs(&visibility.capability_refs, "catalog visibility capability ref")?;
    validate_refs(&visibility.hidden_refs, "catalog visibility hidden ref")?;
    if let Some(redaction_profile_ref) = visibility.redaction_profile_ref.as_ref() {
        validate_ref(redaction_profile_ref, "catalog redaction profile ref")?;
    }
    Ok(())
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported catalog decision {decision}")))
    }
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref, got {value_ref}: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_CATALOG_REFS, field)?;
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

fn insert_bounded<T: Ord>(values: &mut BTreeSet<T>, value: T, maximum: usize, label: &str) -> Result<bool> {
    if values.contains(&value) {
        return Ok(false);
    }
    checked_count_sum(values.len(), 1, maximum, label)?;
    Ok(values.insert(value))
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<BTreeSet<_>>().into_iter().collect()
}

fn push_optional_classification(
    values: &mut impl crate::bounded::VecSink<String>,
    prefix: &str,
    value: Option<&str>,
) -> Result<()> {
    if let Some(value) = value {
        push_bounded(values, format!("{prefix}:{value}"), MAX_CATALOG_REFS, "catalog classifications")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::preserves_rail::OCTET_WARNING_BASELINE_SCHEMA;
    use crate::preserves_rail::parse_text;
    use crate::preserves_rail::u64_value;

    #[test]
    fn summaries_include_registry_names_dependencies_and_ledger_classification() {
        let dir = temp_dir("catalog-summary");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let schema_ref = test_ref("schema");
        let base =
            install_fixture(&registry, "schema", parse_text("<schema \"base\">").expect("schema"), &[], &[schema_ref]);
        let dependent = install_fixture(
            &registry,
            "doc",
            parse_text("<doc \"hello\">").expect("doc"),
            std::slice::from_ref(&base.artifact_ref),
            &[],
        );
        artifacts::set_name_pointer(&registry, &artifacts::SetNamePointerInput {
            pointer_kind: "name",
            name: "docs/main",
            artifact_ref: &dependent.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("set name");
        ledger::import_artifact(&ledger_root, &dependent.artifact.value).expect("ledger import");
        let listed = list(&registry, Some(&ledger_root), &CatalogListInput {
            kind: Some("doc".to_string()),
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("catalog list");
        assert_eq!(listed.items.len(), 1);
        let text = to_text(&listed.items[0]).expect("render summary");
        assert!(["docs/main", "catalog-summary-v1"].iter().any(|needle| text.contains(needle)));
        assert!(text.contains(&base.artifact_ref));
        assert!(text.contains("ledger-kind:artifact-registry-artifact"));
    }

    #[test]
    fn search_filters_schema_dependency_receipt_decision_text_and_visibility() {
        let dir = temp_dir("catalog-search");
        let registry = dir.join("registry");
        let schema_ref = test_ref("schema-search");
        let base = install_fixture(
            &registry,
            "schema",
            parse_text("<schema \"search\">").expect("schema"),
            &[],
            std::slice::from_ref(&schema_ref),
        );
        let receipt_payload = record("rewrite-receipt-v1", vec![
            string("molten.rewrite.receipt.v1"),
            record("operation", vec![string("apply")]),
            record("decision", vec![string("pass")]),
            record("subject", vec![string(test_ref("subject"))]),
            record("refs", vec![sequence(Vec::new())]),
            record("diagnostics", vec![sequence(Vec::new())]),
            record("tool", vec![string("test")]),
            checks_value(&["canonical-receipt"]),
        ]);
        let receipt =
            install_fixture(&registry, "receipt", receipt_payload, std::slice::from_ref(&base.artifact_ref), &[]);
        let found = search(&registry, None, &CatalogSearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![
                CatalogFilter::ArtifactKind("receipt".to_string()),
                CatalogFilter::DependencyRef(base.artifact_ref.clone()),
                CatalogFilter::ReceiptDecision("pass".to_string()),
                CatalogFilter::Text("apply".to_string()),
            ],
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("search receipt");
        assert_eq!(found.items.len(), 1);
        let hidden = search(&registry, None, &CatalogSearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![CatalogFilter::Text("apply".to_string())],
            visibility: CatalogVisibilityInput {
                hidden_refs: vec![receipt.artifact_ref],
                ..CatalogVisibilityInput::default()
            },
        })
        .expect("hidden search");
        assert!(hidden.items.is_empty());
    }

    #[test]
    fn semantic_search_covers_transcript_upgrade_and_receipt_views() {
        let dir = temp_dir("catalog-unison-views");
        let registry = dir.join("registry");
        let transcript_ref = test_ref("transcript");
        let transcript_receipt = record("transcript-run-receipt-v1", vec![
            string(crate::preserves_rail::TRANSCRIPT_RUN_RECEIPT_SCHEMA),
            record("operation", vec![string("run")]),
            record("decision", vec![string("pass")]),
            record("transcript", vec![string(&transcript_ref)]),
            record("mode", vec![string("check")]),
            record("outcomes", vec![sequence(Vec::new())]),
            record("output", vec![record("none", Vec::new())]),
            record("refs", vec![sequence(vec![string(&transcript_ref)])]),
            record("diagnostics", vec![sequence(Vec::new())]),
            record("outcome-values", vec![sequence(Vec::new())]),
            checks_value(&["canonical-run"]),
        ]);
        let transcript_artifact = install_fixture(&registry, "transcript-run-receipt", transcript_receipt, &[], &[]);
        let upgrade_receipt = record("upgrade-receipt-v1", vec![
            string(crate::preserves_rail::UPGRADE_RECEIPT_SCHEMA),
            record("operation", vec![string("session-create")]),
            record("decision", vec![string("pass")]),
            record("session", vec![string("session-catalog")]),
            record("plan", vec![string(test_ref("plan"))]),
            record("task", vec![record("none", Vec::new())]),
            record("refs", vec![sequence(Vec::new())]),
            checks_value(&["canonical-receipt"]),
        ]);
        let upgrade_artifact = install_fixture(&registry, "upgrade-receipt", upgrade_receipt, &[], &[]);
        let transcript = search(&registry, None, &CatalogSearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![CatalogFilter::TranscriptStatus("pass".to_string())],
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("transcript search");
        assert_eq!(transcript.items.len(), 1);
        assert!(
            to_text(&transcript.value)
                .expect("transcript result text")
                .contains(&transcript_artifact.artifact_ref)
        );
        let upgrade = search(&registry, None, &CatalogSearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![CatalogFilter::UpgradeStatus("pass".to_string())],
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("upgrade search");
        assert_eq!(upgrade.items.len(), 1);
        assert!(to_text(&upgrade.value).expect("upgrade result text").contains(&upgrade_artifact.artifact_ref));
        let receipt_view = receipts(&registry, None, &CatalogGraphInput {
            reference: transcript_artifact.artifact_ref,
            transitive: false,
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("receipt view");
        assert!(!receipt_view.items.is_empty());
    }

    #[test]
    fn catalog_classifies_provenance_records_receipts_and_build_evidence() {
        let dir = temp_dir("catalog-provenance");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let artifact_ref = test_ref("provenance-artifact");
        let record = crate::provenance::synthetic_reviewed_provenance_record(&artifact_ref).expect("record");
        let evaluation = crate::provenance::evaluate_provenance(&crate::provenance::ProvenanceEvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&record),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("evaluate provenance");
        ledger::import_artifact(&ledger_root, &record).expect("import record");
        ledger::import_artifact(&ledger_root, &evaluation.receipt_value).expect("import receipt");
        let found = search(&registry, Some(&ledger_root), &CatalogSearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![CatalogFilter::Text("provenance-trust-state:reviewed".to_string())],
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("provenance search");
        assert!(!found.items.is_empty());
        let text = to_text(&found.value).expect("provenance result text");
        assert!(text.contains("provenance:record"));
        assert!(text.contains("provenance:receipt"));
    }

    #[test]
    fn retention_gc_chain_artifacts_are_catalog_searchable() {
        let dir = temp_dir("catalog-retention-gc");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let retention_root = dir.join("retention");
        let fixture = retention_gc_audit_fixture(&retention_root, "catalog-retention-gc", "ledger-gc");
        ledger::import_artifact(&ledger_root, &fixture.plan.value).expect("import plan");
        ledger::import_artifact(&ledger_root, &fixture.apply.value).expect("import apply");
        ledger::import_artifact(&ledger_root, &fixture.execution.value).expect("import execution");
        ledger::import_artifact(&ledger_root, &fixture.audit.value).expect("import audit");

        let found = search(&registry, Some(&ledger_root), &CatalogSearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![
                CatalogFilter::Text(format!("retention-gc-object:{}", fixture.object_ref)),
                CatalogFilter::Text("retention-gc-subsystem:ledger-gc".to_string()),
            ],
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("search retention GC chain");
        assert_eq!(found.items.len(), 4);
        let text = to_text(&found.value).expect("retention GC catalog text");
        assert!(text.contains("retention-gc:plan"));
        assert!(text.contains("retention-gc:apply"));
        assert!(text.contains("retention-gc:execute"));
        assert!(text.contains("retention-gc:audit"));
        assert!(text.contains(&fixture.plan.plan_ref));
        assert!(text.contains(&fixture.apply.apply_ref));
        assert!(text.contains(&fixture.execution.execution_ref));

        let audit = search(&registry, Some(&ledger_root), &CatalogSearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![CatalogFilter::LedgerKind("retention-gc-audit".to_string())],
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("search retention GC audit by ledger kind");
        assert_eq!(audit.items.len(), 1);
        assert!(to_text(&audit.value).expect("audit search text").contains("retention-gc:audit"));
    }

    #[test]
    fn short_id_resolution_denies_too_short_ambiguous_and_hidden_candidates() {
        let dir = temp_dir("catalog-short");
        let registry = dir.join("registry");
        let mut refs_by_first_hex = Vec::<(char, String)>::with_capacity(32);
        let mut ambiguous_pair = None;
        for index in 0..32 {
            let installed =
                install_fixture(&registry, "doc", parse_text(&format!("<doc {index}>")).expect("doc"), &[], &[]);
            let first_hex = installed.artifact_ref.as_bytes()[7] as char;
            if let Some((_, existing_ref)) = refs_by_first_hex.iter().find(|(hex, _)| *hex == first_hex) {
                ambiguous_pair = Some((existing_ref.clone(), installed.artifact_ref.clone()));
                break;
            }
            refs_by_first_hex.push((first_hex, installed.artifact_ref));
        }
        let (first_ref, second_ref) = ambiguous_pair.expect("fixture collision within hex alphabet");
        let shared_prefix = first_ref[7..8].to_string();
        let too_short = resolve_short_id(&registry, None, &CatalogShortIdInput {
            prefix: shared_prefix.clone(),
            min_length: DEFAULT_SHORT_ID_MIN_LENGTH,
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("too short resolution receipt");
        assert_eq!(too_short.decision, "deny");
        let ambiguous = resolve_short_id(&registry, None, &CatalogShortIdInput {
            prefix: shared_prefix.clone(),
            min_length: 0,
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("ambiguous resolution receipt");
        assert_eq!(ambiguous.decision, "deny");
        assert!(ambiguous.candidates.len() >= 2);
        let visible = resolve_short_id(&registry, None, &CatalogShortIdInput {
            prefix: shared_prefix,
            min_length: 0,
            visibility: CatalogVisibilityInput {
                hidden_refs: vec![second_ref],
                ..CatalogVisibilityInput::default()
            },
        })
        .expect("hidden candidate filtered");
        assert_eq!(visible.full_ref.as_deref(), Some(first_ref.as_str()));
    }

    #[test]
    fn view_redacts_sensitive_payloads_before_rendering() {
        let dir = temp_dir("catalog-redact");
        let registry = dir.join("registry");
        let secret =
            install_fixture(&registry, "doc", parse_text("<doc <secret \"do-not-render\">>").expect("secret"), &[], &[
            ]);
        let viewed = view(&registry, None, &CatalogViewInput {
            reference: secret.artifact_ref,
            include_payload: true,
            redacted: true,
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("view redacted");
        let text = to_text(&viewed.value).expect("render view");
        assert!(text.contains("redaction-marker-v1"));
        assert!(!text.contains("do-not-render"));
    }

    #[test]
    fn catalog_view_renders_octet_baseline_quarantine_metadata() {
        let dir = temp_dir("catalog-octet-baseline");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        fs::create_dir_all(&registry).expect("create registry");
        let review_ref = test_ref("octet-review");
        let baseline = record("octet-warning-baseline-v1", vec![
            string(OCTET_WARNING_BASELINE_SCHEMA),
            record("scope", vec![string("workspace")]),
            record("created-at", vec![string("2026-05-31T00:00:00Z")]),
            record("expires-at", vec![string("2026-06-30T00:00:00Z")]),
            record("octet-config-hash", vec![string("b3:config")]),
            record("octet-profile-hash", vec![string("b3:profile")]),
            record("toolchain", vec![string("rustc-test")]),
            record("source-snapshot", vec![string(test_ref("source-snapshot"))]),
            record("finding-keys", vec![sequence(vec![
                record("finding-key", vec![
                    string("b3:finding-a"),
                    string("no_unwrap"),
                    string("molten"),
                    string("src/main.rs:1"),
                    u64_value(1),
                ]),
                record("finding-key", vec![
                    string("b3:finding-b"),
                    string("bool_naming"),
                    string("molten"),
                    string("src/lib.rs:1"),
                    u64_value(1),
                ]),
            ])]),
            record("critical-finding-keys", vec![sequence(vec![string("b3:finding-a")])]),
            record("allowed-profiles", vec![sequence(vec![string("quarantine-ci")])]),
            record("burn-down", vec![
                record("total", vec![u64_value(2)]),
                record("target-next", vec![u64_value(1)]),
                record("deadline", vec![string("2026-06-30T00:00:00Z")]),
            ]),
            record("review-refs", vec![sequence(vec![string(&review_ref)])]),
            checks_value(&["baseline-findings-keyed"]),
        ]);
        let imported = ledger::import_artifact(&ledger_root, &baseline).expect("import baseline");
        let viewed = view(&registry, Some(&ledger_root), &CatalogViewInput {
            reference: imported.artifact_ref,
            include_payload: true,
            redacted: true,
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("view octet baseline");
        let text = to_text(&viewed.value).expect("render catalog view");

        assert!(text.contains("octet-baseline:warning-quarantine"));
        assert!(text.contains("octet-baseline-findings:2"));
        assert!(text.contains("octet-baseline-critical:1"));
        assert!(text.contains("octet-baseline-expires-at:2026-06-30T00:00:00Z"));
        assert!(text.contains("octet-baseline-burn-down-target-next:1"));
        assert!(text.contains(&format!("octet-review-ref:{review_ref}")));
    }

    #[hegel::test(test_cases = 12)]
    fn hegel_catalog_identity_short_ids_and_visibility_are_stable(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let dir = temp_dir("catalog-hegel");
        let registry = dir.join("registry");
        let payload = record("doc", vec![string(format!("payload-{salt}"))]);
        let installed = install_fixture(&registry, "doc", payload, &[], &[]);
        let first = list(&registry, None, &CatalogListInput {
            kind: Some("doc".to_string()),
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("first list");
        let display_name = format!("display-{salt}");
        artifacts::set_name_pointer(&registry, &artifacts::SetNamePointerInput {
            pointer_kind: "name",
            name: &display_name,
            artifact_ref: &installed.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("set display name");
        let second = list(&registry, None, &CatalogListInput {
            kind: Some("doc".to_string()),
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("second list");
        assert_eq!(first.items.len(), second.items.len());
        assert!(first.items[0].collect_simple_record("catalog-summary-v1", None).is_some());
        let resolved = resolve_short_id(&registry, None, &CatalogShortIdInput {
            prefix: installed.artifact_ref[7..19].to_string(),
            min_length: 8,
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("resolve stable short id");
        assert_eq!(resolved.full_ref, Some(installed.artifact_ref.clone()));
        let hidden = resolve_short_id(&registry, None, &CatalogShortIdInput {
            prefix: installed.artifact_ref[7..19].to_string(),
            min_length: 8,
            visibility: CatalogVisibilityInput {
                hidden_refs: vec![installed.artifact_ref],
                ..CatalogVisibilityInput::default()
            },
        })
        .expect("hidden short id");
        assert_eq!(hidden.decision, "deny");
    }

    struct RetentionGcAuditFixture {
        object_ref: String,
        plan: crate::retention::RetentionGcPlan,
        apply: crate::retention::RetentionGcApply,
        execution: crate::retention::RetentionGcExecutionGate,
        audit: crate::retention::RetentionGcAudit,
    }

    fn retention_gc_audit_fixture(root: &Path, label: &str, subsystem: &str) -> RetentionGcAuditFixture {
        let requester_ref = test_ref(&format!("{label}-requester"));
        let object_ref = test_ref(&format!("{label}-object"));
        let object_kind = "chunk";
        let retention_class = crate::retention::CLASS_DURABLE_VALUE;
        let action = crate::retention::ACTION_DELETE;
        let store_admission = |kind: &str, suffix: &str| -> String {
            crate::retention::store_retention_evidence_admission(
                root,
                &crate::retention::RetentionEvidenceAdmissionInput {
                    kind,
                    decision: "pass",
                    requester_ref: &requester_ref,
                    object_ref: &object_ref,
                    object_kind,
                    retention_class,
                    action,
                    bound_refs: &[test_ref(&format!("{label}-{suffix}"))],
                    retained_refs: &[],
                    remote_refs: &[],
                    is_reference_index_complete: true,
                    is_current: true,
                    revoked_refs: &[],
                    diagnostics: &[],
                },
            )
            .expect("store retention GC catalog admission")
            .admission_ref
        };
        let evidence = crate::retention::DestructiveRetentionEvidence {
            requester_ref: Some(requester_ref.clone()),
            policy_refs: vec![store_admission(crate::retention::ADMISSION_KIND_POLICY, "policy")],
            authority_refs: vec![store_admission(crate::retention::ADMISSION_KIND_AUTHORITY, "authority")],
            evidence_refs: vec![store_admission(
                crate::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
                "support",
            )],
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs: vec![store_admission(
                crate::retention::ADMISSION_KIND_REFERENCE_INDEX,
                "index",
            )],
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        };
        let plan = crate::retention::store_retention_gc_plan(crate::retention::RetentionGcPlanInput {
            root,
            subsystem,
            object_ref: &object_ref,
            object_kind,
            retention_class,
            action,
            evidence: &evidence,
        })
        .expect("store retention GC catalog plan");
        let apply = crate::retention::apply_retention_gc_plan(crate::retention::RetentionGcApplyFromPlanInput {
            root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply retention GC catalog plan");
        let execution =
            crate::retention::store_retention_gc_execution_gate(crate::retention::RetentionGcExecutionGateInput {
                root,
                subsystem,
                action,
                object_ref: &object_ref,
                object_kind,
                retention_class,
                apply_ref: Some(&apply.apply_ref),
            })
            .expect("store retention GC catalog execution");
        let audit = crate::retention::audit_retention_gc_execution(crate::retention::RetentionGcAuditInput {
            root,
            execution_ref: &execution.execution_ref,
        })
        .expect("audit retention GC catalog execution");
        RetentionGcAuditFixture {
            object_ref,
            plan,
            apply,
            execution,
            audit,
        }
    }

    fn install_fixture(
        root: &Path,
        kind: &str,
        payload: IOValue,
        dependency_refs: &[String],
        schema_refs: &[String],
    ) -> artifacts::ArtifactInstall {
        artifacts::install_artifact(root, &artifacts::ArtifactInstallInput {
            kind: kind.to_string(),
            payload,
            schema_refs: schema_refs.to_vec(),
            dependency_refs: dependency_refs.to_vec(),
            effect_manifest_ref: Some(test_ref("effect")),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install fixture")
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("catalog-test-ref", vec![string(label)])).expect("test ref")
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
