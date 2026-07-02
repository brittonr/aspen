type IoValue = preserves::IOValue;
use preserves::ValueImpl;

type OrderedSet<T> = std::collections::BTreeSet<T>;
type Path = std::path::Path;
type CompoundClass = preserves::CompoundClass;
type Record<T> = preserves::Record<T>;
type Value<T> = preserves::Value<T>;
type ValueClass = preserves::ValueClass;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

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

fn to_text(value: &IoValue) -> Result<String> {
    crate::preserves_rail::to_text(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const TOOL_VERSION: &str = "local-structured-rewrite-v1";

const MAX_REWRITE_ITEMS: usize = 4_096;
const _: () = assert!(MAX_REWRITE_ITEMS > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewritePattern {
    Any,
    ArtifactKind(String),
    RecordLabel(String),
    StringEquals(String),
    StringContains(String),
    SchemaShapeKind(String),
    RefContains(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteQueryInput {
    pub artifact_kinds: Vec<String>,
    pub root_refs: Vec<String>,
    pub include_dependencies: bool,
    pub pattern: RewritePattern,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub hidden_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteReplacement {
    StringValue { from: String, to: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewritePlanInput {
    pub query: RewriteQueryInput,
    pub replacement: RewriteReplacement,
    pub planner_ref: String,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub transcript_refs: Vec<String>,
    pub schema_migration_recipe_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteBinding {
    pub path: String,
    pub value_ref: String,
    pub preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteMatch {
    pub artifact_ref: String,
    pub kind: String,
    pub payload_ref: String,
    pub bindings: Vec<RewriteBinding>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteQuery {
    pub query_ref: String,
    pub query_value: IoValue,
    pub matches: Vec<RewriteMatch>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteDiff {
    pub artifact_ref: String,
    pub kind: String,
    pub old_payload_ref: String,
    pub new_payload_ref: String,
    pub paths: Vec<String>,
    pub old_preview: String,
    pub new_preview: String,
    pub new_payload: IoValue,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewritePreview {
    pub query: RewriteQuery,
    pub plan_ref: String,
    pub plan_value: IoValue,
    pub diffs: Vec<RewriteDiff>,
    pub impacted_refs: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteInstalledArtifact {
    pub old_artifact_ref: String,
    pub new_artifact_ref: String,
    pub install_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteApply {
    pub preview: RewritePreview,
    pub installed: Vec<RewriteInstalledArtifact>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub subject_ref: String,
    pub refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

pub fn default_local_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("rewrite-local-ref", vec![string(kind), string(label)]))
}

pub fn find(root: &Path, input: &RewriteQueryInput) -> Result<RewriteQuery> {
    validate_query_input(input)?;
    let query_value = rewrite_query_value(input)?;
    let query_ref = canonical_hash(&query_value)?;
    let matches = found_items(root, input)?;
    let refs = found_refs(&query_ref, input, &matches);
    let receipt_value = rewrite_receipt_value(&RewriteReceiptValueInput {
        operation: "query",
        decision: "pass",
        subject_ref: &query_ref,
        refs: &refs,
        diagnostics: &[],
        checks: &[
            ("canonical-query-ref", "pass"),
            ("visibility-filter", "pass"),
            ("bounded-preserves-pattern", "pass"),
            ("no-text-only-bypass", "pass"),
        ],
    })?;
    Ok(RewriteQuery {
        query_ref,
        query_value,
        matches,
        receipt_value,
    })
}

pub fn preview(root: &Path, input: &RewritePlanInput) -> Result<RewritePreview> {
    validate_plan_input(input)?;
    let query = find(root, &input.query)?;
    let diffs = diff_items(root, input, &query)?;
    let impacted_refs = impacted_refs(root, &diffs)?;
    let plan_value = rewrite_plan_value(input, &query, &diffs, &impacted_refs)?;
    let plan_ref = canonical_hash(&plan_value)?;
    let refs = plan_refs(&PlanRefs {
        plan_ref: &plan_ref,
        query: &query,
        diffs: &diffs,
        impacted_refs: &impacted_refs,
        input,
    })?;
    let decision = if diffs.is_empty() { "deny" } else { "pass" };
    let diagnostics = if diffs.is_empty() {
        vec!["rewrite preview produced no structural diffs".to_string()]
    } else {
        Vec::new()
    };
    let receipt_value = rewrite_receipt_value(&RewriteReceiptValueInput {
        operation: "preview",
        decision,
        subject_ref: &plan_ref,
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &[
            ("dry-run-only", "pass"),
            ("immutable-old-artifacts", "pass"),
            ("structural-diff", if diffs.is_empty() { "fail" } else { "pass" }),
            ("impact-set", "pass"),
            ("policy-admission", "pass"),
            ("capability-admission", "pass"),
        ],
    })?;
    Ok(RewritePreview {
        query,
        plan_ref,
        plan_value,
        diffs,
        impacted_refs,
        receipt_value,
    })
}

fn found_items(root: &Path, input: &RewriteQueryInput) -> Result<Vec<RewriteMatch>> {
    let scope = scoped_refs(root, &input.root_refs, input.include_dependencies)?;
    let hidden = input.hidden_refs.as_slice().iter().cloned().collect::<OrderedSet<_>>();
    let kind_filter = input.artifact_kinds.as_slice().iter().cloned().collect::<OrderedSet<_>>();
    let mut matches = Vec::new();
    for artifact in crate::artifacts::list_artifacts(root, None)? {
        if hidden.contains(&artifact.artifact_ref) {
            continue;
        }
        if !scope.is_empty() && !scope.contains(&artifact.artifact_ref) {
            continue;
        }
        if !kind_filter.is_empty() && !kind_filter.contains(&artifact.kind) {
            continue;
        }
        if let RewritePattern::ArtifactKind(kind) = &input.pattern
            && &artifact.kind != kind
        {
            continue;
        }
        let payload = crate::artifacts::read_payload(root, &artifact.artifact_ref)?;
        let mut bindings = Vec::new();
        collect_bindings(&payload, &input.pattern, "$", &mut bindings)?;
        if !bindings.is_empty() || matches!(&input.pattern, RewritePattern::ArtifactKind(_)) {
            if bindings.is_empty() {
                push_bounded(
                    &mut bindings,
                    RewriteBinding {
                        path: "$".to_string(),
                        value_ref: canonical_hash(&payload)?,
                        preview: preview_text(&payload)?,
                    },
                    MAX_REWRITE_ITEMS,
                    "rewrite bindings",
                )?;
            }
            let payload_ref = canonical_hash(&payload)?;
            let value = rewrite_match_value(&artifact.artifact_ref, &artifact.kind, &payload_ref, &bindings)?;
            push_bounded(
                &mut matches,
                RewriteMatch {
                    artifact_ref: artifact.artifact_ref,
                    kind: artifact.kind,
                    payload_ref,
                    bindings,
                    value,
                },
                MAX_REWRITE_ITEMS,
                "rewrite matches",
            )?;
        }
    }
    matches.sort_by(|left, right| left.artifact_ref.cmp(&right.artifact_ref));
    Ok(matches)
}

fn found_refs(query_ref: &str, input: &RewriteQueryInput, matches: &[RewriteMatch]) -> Vec<String> {
    let mut refs = vec![query_ref.to_string()];
    refs.extend(matches.iter().map(|rewrite_match| rewrite_match.artifact_ref.clone()));
    refs.extend(input.root_refs.as_slice().iter().cloned());
    refs.extend(input.policy_refs.as_slice().iter().cloned());
    refs.extend(input.capability_refs.as_slice().iter().cloned());
    refs.extend(input.hidden_refs.as_slice().iter().cloned());
    refs
}
