type ArtifactPayloadRef = crate::artifacts::ArtifactPayloadRef;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type PreservesRecord<T> = preserves::Record<T>;
type PreservesValue<T> = preserves::Value<T>;
type Result<T> = crate::error::Result<T>;
type Set<T> = std::collections::BTreeSet<T>;

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

#[cfg(test)]
fn parse_text(source: &str) -> Result<IoValue> {
    crate::preserves_rail::parse_text(source)
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

fn to_text(value: &IoValue) -> Result<String> {
    crate::preserves_rail::to_text(value)
}

// r[impl molten.runtime_spine.canonical_content_refs.migration]
fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &PreservesValue<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

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
pub struct VisibilityInput {
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub hidden_refs: Vec<String>,
    pub redaction_profile_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filter {
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
pub struct ListInput {
    pub kind: Option<String>,
    pub visibility: VisibilityInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchInput {
    pub root_refs: Vec<String>,
    pub include_dependencies: bool,
    pub include_dependents: bool,
    pub filters: Vec<Filter>,
    pub visibility: VisibilityInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewInput {
    pub reference: String,
    pub include_payload: bool,
    pub redacted: bool,
    pub visibility: VisibilityInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphInput {
    pub reference: String,
    pub transitive: bool,
    pub visibility: VisibilityInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortIdInput {
    pub prefix: String,
    pub min_length: usize,
    pub visibility: VisibilityInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoreInput {
    pub visibility: VisibilityInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    pub query_ref: String,
    pub result_ref: String,
    pub decision: String,
    pub items: Vec<IoValue>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortIdResolution {
    pub prefix: String,
    pub full_ref: Option<String>,
    pub candidates: Vec<String>,
    pub decision: String,
    pub value: IoValue,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub query_ref: String,
    pub result_ref: Option<String>,
    pub refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

pub fn list(registry_root: &Path, ledger_root: Option<&Path>, input: &ListInput) -> Result<QueryResult> {
    validate_visibility(&input.visibility)?;
    if let Some(kind) = input.kind.as_deref() {
        validate_non_empty(kind, "catalog list kind")?;
    }
    let filters = input.kind.as_ref().map(|kind| vec![Filter::ArtifactKind(kind.clone())]).unwrap_or_default();
    let query_value = build_query_value(&QueryValueInput {
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

pub fn search(registry_root: &Path, ledger_root: Option<&Path>, input: &SearchInput) -> Result<QueryResult> {
    validate_visibility(&input.visibility)?;
    validate_refs(&input.root_refs, "catalog search root ref")?;
    validate_filters(&input.filters)?;
    let query_value = build_query_value(&QueryValueInput {
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

pub fn view(registry_root: &Path, ledger_root: Option<&Path>, input: &ViewInput) -> Result<QueryResult> {
    validate_visibility(&input.visibility)?;
    let full_ref = resolve_reference(registry_root, ledger_root, &input.reference, &input.visibility)?;
    let query_value = build_query_value(&QueryValueInput {
        operation: "view",
        root_refs: std::slice::from_ref(&full_ref),
        include_dependencies: false,
        include_dependents: false,
        filters: &[Filter::Ref(full_ref.clone())],
        visibility: &input.visibility,
        render_mode: if input.redacted { "redacted" } else { "raw" },
        include_payload: input.include_payload,
    })?;
    let item = if let Ok(artifact) = crate::artifacts::read_artifact(registry_root, &full_ref) {
        let summary = registry_summary(registry_root, ledger_root, artifact, &input.visibility)?;
        let payload = if input.include_payload {
            let payload = crate::artifacts::read_payload(registry_root, &full_ref)?;
            if input.redacted {
                maybe_redacted_value(&payload, input.visibility.redaction_profile_ref.as_deref())?
            } else {
                payload
            }
        } else {
            record("none", Vec::new())
        };
        view_value(&summary, &summary.value, &payload, input.include_payload, input.redacted)?
    } else if let Some(ledger_root) = ledger_root {
        let value = crate::ledger::read_artifact(ledger_root, &full_ref)?;
        let summary = ledger_summary(registry_root, ledger_root, &full_ref, value.clone(), &input.visibility)?;
        let rendered = if input.redacted {
            maybe_redacted_value(&value, input.visibility.redaction_profile_ref.as_deref())?
        } else {
            value
        };
        view_value(&summary, &summary.value, &rendered, true, input.redacted)?
    } else {
        return Err(MoltenError::invalid_harness(format!(
            "catalog ref {full_ref} not found in registry and no ledger was supplied"
        )));
    };
    finish_query("view", query_value, vec![item], Vec::new())
}

pub fn dependencies(registry_root: &Path, ledger_root: Option<&Path>, input: &GraphInput) -> Result<QueryResult> {
    graph_query(registry_root, ledger_root, input, "deps")
}

pub fn dependents(registry_root: &Path, ledger_root: Option<&Path>, input: &GraphInput) -> Result<QueryResult> {
    graph_query(registry_root, ledger_root, input, "dependents")
}

// r[impl molten.catalog.share_like_linked_views]
// r[impl molten.catalog.redaction_authorization]
pub fn impact(registry_root: &Path, ledger_root: Option<&Path>, input: &GraphInput) -> Result<QueryResult> {
    validate_visibility(&input.visibility)?;
    let full_ref = resolve_reference(registry_root, ledger_root, &input.reference, &input.visibility)?;
    let query_value = build_query_value(&QueryValueInput {
        operation: "impact",
        root_refs: std::slice::from_ref(&full_ref),
        include_dependencies: false,
        include_dependents: input.transitive,
        filters: &[Filter::Ref(full_ref.clone())],
        visibility: &input.visibility,
        render_mode: "impact-query",
        include_payload: false,
    })?;
    let impact = crate::artifacts::impact_query(registry_root, &crate::artifacts::ArtifactImpactQueryInput {
        subject_ref: full_ref,
        relation_filters: Vec::new(),
        include_transitive: input.transitive,
        hidden_refs: input.visibility.hidden_refs.clone(),
    })?;
    let mut items = vec![impact.receipt_value];
    let summaries = collect_summaries(registry_root, ledger_root, &input.visibility)?;
    for reference in impact.direct_dependents.iter().chain(impact.transitive_dependents.iter()) {
        if let Some(summary) = summaries.iter().find(|summary| &summary.artifact_ref == reference) {
            push_bounded(&mut items, summary.value.clone(), MAX_CATALOG_ITEMS, "catalog impact items")?;
        }
    }
    finish_query("impact", query_value, items, impact.diagnostics)
}
