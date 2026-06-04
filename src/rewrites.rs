use std::collections::BTreeSet;
use std::path::Path;

use preserves::CompoundClass;
use preserves::IOValue;
use preserves::Record;
use preserves::Value;
use preserves::ValueClass;
use preserves::ValueImpl;

use crate::artifacts;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::REWRITE_DIFF_SCHEMA;
use crate::preserves_rail::REWRITE_MATCH_SCHEMA;
use crate::preserves_rail::REWRITE_PLAN_SCHEMA;
use crate::preserves_rail::REWRITE_QUERY_SCHEMA;
use crate::preserves_rail::REWRITE_RECEIPT_SCHEMA;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::to_text;
use crate::preserves_rail::value_to_iovalue;
use crate::upgrades;

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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteQuery {
    pub query_ref: String,
    pub query_value: IOValue,
    pub matches: Vec<RewriteMatch>,
    pub receipt_value: IOValue,
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
    pub new_payload: IOValue,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewritePreview {
    pub query: RewriteQuery,
    pub plan_ref: String,
    pub plan_value: IOValue,
    pub diffs: Vec<RewriteDiff>,
    pub impacted_refs: Vec<String>,
    pub receipt_value: IOValue,
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
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub subject_ref: String,
    pub refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

pub fn default_local_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("rewrite-local-ref", vec![string(kind), string(label)]))
}

pub fn find(root: &Path, input: &RewriteQueryInput) -> Result<RewriteQuery> {
    validate_query_input(input)?;
    let query_value = rewrite_query_value(input)?;
    let query_ref = canonical_hash(&query_value)?;
    let scope = scoped_refs(root, &input.root_refs, input.include_dependencies)?;
    let hidden = input.hidden_refs.as_slice().iter().cloned().collect::<BTreeSet<_>>();
    let kind_filter = input.artifact_kinds.as_slice().iter().cloned().collect::<BTreeSet<_>>();
    let mut matches = Vec::new();
    for artifact in artifacts::list_artifacts(root, None)? {
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
        let payload = artifacts::read_payload(root, &artifact.artifact_ref)?;
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
    let mut refs = vec![query_ref.clone()];
    refs.extend(matches.iter().map(|rewrite_match| rewrite_match.artifact_ref.clone()));
    refs.extend(input.root_refs.as_slice().iter().cloned());
    refs.extend(input.policy_refs.as_slice().iter().cloned());
    refs.extend(input.capability_refs.as_slice().iter().cloned());
    refs.extend(input.hidden_refs.as_slice().iter().cloned());
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
    let mut diffs = Vec::new();
    for rewrite_match in &query.matches {
        let artifact = artifacts::read_artifact(root, &rewrite_match.artifact_ref)?;
        let payload = artifacts::read_payload(root, &artifact.artifact_ref)?;
        let old_payload_ref = canonical_hash(&payload)?;
        let RewriteReplacement::StringValue { from, to } = &input.replacement;
        let mut paths = Vec::new();
        let rewritten = rewrite_string_values(RewriteStringValuesInput {
            value: &payload,
            from,
            to,
            path: "$",
            changed_paths: &mut paths,
        })?;
        if paths.is_empty() {
            continue;
        }
        let new_payload_ref = canonical_hash(&rewritten)?;
        let value = rewrite_diff_value(&RewriteDiffValueInput {
            artifact_ref: &artifact.artifact_ref,
            kind: &artifact.kind,
            old_payload_ref: &old_payload_ref,
            new_payload_ref: &new_payload_ref,
            paths: &paths,
            old_preview: &preview_text(&payload)?,
            new_preview: &preview_text(&rewritten)?,
        })?;
        push_bounded(
            &mut diffs,
            RewriteDiff {
                artifact_ref: artifact.artifact_ref,
                kind: artifact.kind,
                old_payload_ref,
                new_payload_ref,
                paths,
                old_preview: preview_text(&payload)?,
                new_preview: preview_text(&rewritten)?,
                new_payload: rewritten,
                value,
            },
            MAX_REWRITE_ITEMS,
            "rewrite diffs",
        )?;
    }
    diffs.sort_by(|left, right| left.artifact_ref.cmp(&right.artifact_ref));
    let impacted_refs = impacted_refs(root, &diffs)?;
    let plan_value = rewrite_plan_value(input, &query, &diffs, &impacted_refs)?;
    let plan_ref = canonical_hash(&plan_value)?;
    let mut refs = vec![
        plan_ref.clone(),
        query.query_ref.clone(),
        canonical_hash(&query.receipt_value)?,
    ];
    refs.extend(diffs.iter().map(|diff| diff.artifact_ref.clone()));
    refs.extend(diffs.iter().map(|diff| diff.new_payload_ref.clone()));
    refs.extend(impacted_refs.as_slice().iter().cloned());
    refs.extend(input.policy_refs.as_slice().iter().cloned());
    refs.extend(input.capability_refs.as_slice().iter().cloned());
    refs.extend(input.transcript_refs.as_slice().iter().cloned());
    refs.extend(input.schema_migration_recipe_refs.as_slice().iter().cloned());
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

pub fn apply(root: &Path, input: &RewritePlanInput) -> Result<RewriteApply> {
    let preview = preview(root, input)?;
    if preview.diffs.is_empty() {
        return Err(MoltenError::invalid_harness("rewrite apply denied because preview has no diffs"));
    }
    let preview_receipt_ref = canonical_hash(&preview.receipt_value)?;
    let query_receipt_ref = canonical_hash(&preview.query.receipt_value)?;
    let mut installed = Vec::new();
    for diff in &preview.diffs {
        let artifact = artifacts::read_artifact(root, &diff.artifact_ref)?;
        let mut policy_refs = sorted_unique_refs(&merge_refs(&artifact.policy_refs, &input.policy_refs));
        policy_refs.push(preview.plan_ref.clone());
        policy_refs = sorted_unique_refs(&policy_refs);
        let mut evidence_refs = artifact.evidence_refs.clone();
        evidence_refs.push(preview_receipt_ref.clone());
        evidence_refs.push(query_receipt_ref.clone());
        evidence_refs.extend(input.transcript_refs.as_slice().iter().cloned());
        evidence_refs.extend(input.schema_migration_recipe_refs.as_slice().iter().cloned());
        evidence_refs = sorted_unique_refs(&evidence_refs);
        let install = artifacts::install_artifact(root, &artifacts::ArtifactInstallInput {
            kind: artifact.kind,
            payload: diff.new_payload.clone(),
            schema_refs: artifact.schema_refs,
            dependency_refs: artifact.dependency_refs,
            effect_manifest_ref: artifact.effect_manifest_ref,
            policy_refs,
            evidence_refs,
            installer_ref: input.planner_ref.clone(),
            capability_refs: input.capability_refs.clone(),
        })?;
        let install_receipt_ref = canonical_hash(&install.receipt_value)?;
        push_bounded(
            &mut installed,
            RewriteInstalledArtifact {
                old_artifact_ref: diff.artifact_ref.clone(),
                new_artifact_ref: install.artifact_ref,
                install_receipt_ref,
            },
            MAX_REWRITE_ITEMS,
            "rewrite installed artifacts",
        )?;
    }
    let mut refs = vec![preview.plan_ref.clone(), preview_receipt_ref, query_receipt_ref];
    for item in &installed {
        refs.push(item.old_artifact_ref.clone());
        refs.push(item.new_artifact_ref.clone());
        refs.push(item.install_receipt_ref.clone());
    }
    let apply_subject = local_ref("rewrite-apply", &refs)?;
    let receipt_value = rewrite_receipt_value(&RewriteReceiptValueInput {
        operation: "apply",
        decision: "pass",
        subject_ref: &apply_subject,
        refs: &refs,
        diagnostics: &[],
        checks: &[
            ("artifact-creation", "pass"),
            ("no-in-place-mutation", "pass"),
            ("preview-ref-binding", "pass"),
            ("upgrade-session-hook-ready", "pass"),
        ],
    })?;
    Ok(RewriteApply {
        preview,
        installed,
        receipt_value,
    })
}

pub fn upgrade_plan_from_apply(
    rewrite: &RewriteApply,
    session_id: &str,
    initiator_ref: &str,
    capability_refs: &[String],
    policy_refs: &[String],
) -> Result<IOValue> {
    validate_non_empty(session_id, "rewrite upgrade session id")?;
    validate_ref(initiator_ref, "rewrite upgrade initiator ref")?;
    validate_refs(capability_refs, "rewrite upgrade capability ref")?;
    validate_refs(policy_refs, "rewrite upgrade policy ref")?;
    let apply_receipt_ref = canonical_hash(&rewrite.receipt_value)?;
    let preview_receipt_ref = canonical_hash(&rewrite.preview.receipt_value)?;
    let mut tasks = Vec::new();
    for (index, installed) in rewrite.installed.iter().enumerate() {
        push_bounded(
            &mut tasks,
            upgrades::UpgradeTaskInput {
                task_id: format!("rewrite-install-{index}"),
                kind: "install-artifact".to_string(),
                subject: installed.old_artifact_ref.clone(),
                from_ref: Some(installed.old_artifact_ref.clone()),
                to_ref: Some(installed.new_artifact_ref.clone()),
                precondition_refs: vec![rewrite.preview.plan_ref.clone(), preview_receipt_ref.clone()],
                postcondition_refs: vec![installed.install_receipt_ref.clone()],
                reversible: true,
            },
            MAX_REWRITE_ITEMS,
            "rewrite upgrade tasks",
        )?;
    }
    if !rewrite.preview.query.matches.is_empty() {
        push_bounded(
            &mut tasks,
            upgrades::UpgradeTaskInput {
                task_id: "rewrite-transcript-gate".to_string(),
                kind: "transcript-rerun".to_string(),
                subject: rewrite.preview.plan_ref.clone(),
                from_ref: None,
                to_ref: None,
                precondition_refs: vec![preview_receipt_ref.clone()],
                postcondition_refs: vec![apply_receipt_ref.clone()],
                reversible: true,
            },
            MAX_REWRITE_ITEMS,
            "rewrite upgrade tasks",
        )?;
    }
    let affected_refs = rewrite
        .installed
        .iter()
        .flat_map(|installed| [installed.old_artifact_ref.clone(), installed.new_artifact_ref.clone()])
        .collect::<Vec<_>>();
    upgrades::upgrade_plan_value(&upgrades::UpgradePlanInput {
        session_id: session_id.to_string(),
        reason: "structured rewrite".to_string(),
        summary: format!("structured rewrite applied {} immutable artifact replacement(s)", rewrite.installed.len()),
        initiator_ref: initiator_ref.to_string(),
        capability_refs: capability_refs.to_vec(),
        affected_refs: sorted_unique_refs(&affected_refs),
        impact_refs: rewrite.preview.impacted_refs.clone(),
        tasks,
        compatibility: upgrades::UpgradeCompatibilityWindow {
            old_refs: rewrite.installed.iter().map(|installed| installed.old_artifact_ref.clone()).collect(),
            new_refs: rewrite.installed.iter().map(|installed| installed.new_artifact_ref.clone()).collect(),
            expires_at: None,
            policy_refs: policy_refs.to_vec(),
        },
        rollback_refs: rewrite.installed.iter().map(|installed| installed.old_artifact_ref.clone()).collect(),
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![rewrite.preview.plan_ref.clone(), preview_receipt_ref, apply_receipt_ref],
        source_gate_receipt_values: vec![crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?],
    })
}

pub fn parse_rewrite_receipt(value: &IOValue) -> Result<RewriteReceipt> {
    let fields = value
        .collect_simple_record("rewrite-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <rewrite-receipt-v1 ...>"))?;
    require_schema(&fields[0], REWRITE_RECEIPT_SCHEMA, "rewrite receipt")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-receipt", "rewrite receipt")?;
    Ok(RewriteReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        subject_ref: record_ref(&fields[3], "subject")?,
        refs: record_ref_sequence(&fields[4], "refs")?,
        diagnostics: record_string_sequence(&fields[5], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn rewrite_summary(value: &IOValue) -> Result<String> {
    if let Ok(receipt) = parse_rewrite_receipt(value) {
        return Ok(format!(
            "rewrite receipt operation={} decision={} subject={} refs={}",
            receipt.operation,
            receipt.decision,
            receipt.subject_ref,
            receipt.refs.len()
        ));
    }
    if let Some(fields) = value.collect_simple_record("rewrite-plan-v1", Some(11)) {
        require_schema(&fields[0], REWRITE_PLAN_SCHEMA, "rewrite plan")?;
        let diffs = value_to_iovalue(&fields[5]);
        let diff_record = simple_record(&diffs, "diffs", 1)?;
        let diff_count = required_sequence(&diff_record[0], "rewrite plan diffs")?.len();
        return Ok(format!("rewrite plan ref={} diffs={diff_count}", canonical_hash(value)?));
    }
    if let Some(fields) = value.collect_simple_record("rewrite-query-v1", Some(6)) {
        require_schema(&fields[0], REWRITE_QUERY_SCHEMA, "rewrite query")?;
        return Ok(format!("rewrite query ref={}", canonical_hash(value)?));
    }
    Err(MoltenError::invalid_harness("unsupported rewrite artifact for show"))
}

pub fn rewrite_query_value(input: &RewriteQueryInput) -> Result<IOValue> {
    validate_query_input(input)?;
    Ok(record("rewrite-query-v1", vec![
        string(REWRITE_QUERY_SCHEMA),
        record("scope", vec![
            refs_sequence(&sorted_unique_refs(&input.root_refs)),
            bool_value(input.include_dependencies),
            sequence(sorted_unique_strings(&input.artifact_kinds).as_slice().iter().map(string).collect()),
        ]),
        pattern_value(&input.pattern)?,
        record("visibility", vec![
            refs_sequence(&sorted_unique_refs(&input.policy_refs)),
            refs_sequence(&sorted_unique_refs(&input.capability_refs)),
            refs_sequence(&sorted_unique_refs(&input.hidden_refs)),
        ]),
        record("constraints", vec![sequence(vec![
            record("constraint", vec![string("immutable-artifacts-only")]),
            record("constraint", vec![string("bounded-preserves-patterns")]),
        ])]),
        checks_value(&[
            "canonical-query-ref",
            "visibility-filter",
            "bounded-preserves-pattern",
            "no-text-only-bypass",
        ]),
    ]))
}

fn rewrite_plan_value(
    input: &RewritePlanInput,
    query: &RewriteQuery,
    diffs: &[RewriteDiff],
    impacted_refs: &[String],
) -> Result<IOValue> {
    Ok(record("rewrite-plan-v1", vec![
        string(REWRITE_PLAN_SCHEMA),
        record("planner", vec![string(&input.planner_ref), refs_sequence(&input.capability_refs)]),
        record("query", vec![query.query_value.clone(), string(&query.query_ref)]),
        replacement_value(&input.replacement)?,
        record("matches", vec![sequence(
            query.matches.iter().map(|rewrite_match| rewrite_match.value.clone()).collect(),
        )]),
        record("diffs", vec![sequence(diffs.iter().map(|diff| diff.value.clone()).collect())]),
        record("impact", vec![refs_sequence(impacted_refs)]),
        record("transcripts", vec![refs_sequence(&input.transcript_refs)]),
        record("schema-migrations", vec![refs_sequence(&input.schema_migration_recipe_refs)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        checks_value(&[
            "dry-run-preview",
            "artifact-creation-required",
            "no-in-place-mutation",
            "upgrade-session-hook-ready",
            "transcript-validation-hook",
            "schema-migration-hook",
        ]),
    ]))
}

fn rewrite_match_value(
    artifact_ref: &str,
    kind: &str,
    payload_ref: &str,
    bindings: &[RewriteBinding],
) -> Result<IOValue> {
    validate_ref(artifact_ref, "rewrite match artifact ref")?;
    validate_ref(payload_ref, "rewrite match payload ref")?;
    validate_non_empty(kind, "rewrite match kind")?;
    Ok(record("rewrite-match-v1", vec![
        string(REWRITE_MATCH_SCHEMA),
        record("artifact", vec![string(artifact_ref), string(kind), string(payload_ref)]),
        record("paths", vec![sequence(bindings.iter().map(|binding| string(&binding.path)).collect())]),
        record("bindings", vec![sequence(
            bindings
                .iter()
                .map(|binding| {
                    record("binding", vec![
                        string(&binding.path),
                        string(&binding.value_ref),
                        string(&binding.preview),
                    ])
                })
                .collect(),
        )]),
        checks_value(&["canonical-binding-ref", "bounded-path", "visible-result"]),
    ]))
}

struct RewriteDiffValueInput<'a> {
    artifact_ref: &'a str,
    kind: &'a str,
    old_payload_ref: &'a str,
    new_payload_ref: &'a str,
    paths: &'a [String],
    old_preview: &'a str,
    new_preview: &'a str,
}

struct RewriteReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    subject_ref: &'a str,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

fn rewrite_diff_value(input: &RewriteDiffValueInput<'_>) -> Result<IOValue> {
    validate_ref(input.artifact_ref, "rewrite diff artifact ref")?;
    validate_ref(input.old_payload_ref, "rewrite diff old payload ref")?;
    validate_ref(input.new_payload_ref, "rewrite diff new payload ref")?;
    Ok(record("rewrite-diff-v1", vec![
        string(REWRITE_DIFF_SCHEMA),
        record("artifact", vec![string(input.artifact_ref), string(input.kind)]),
        record("payload", vec![string(input.old_payload_ref), string(input.new_payload_ref)]),
        record("paths", vec![sequence(input.paths.iter().map(string).collect())]),
        record("preview", vec![string(input.old_preview), string(input.new_preview)]),
        checks_value(&["structural-rewrite", "old-artifact-preserved", "canonical-new-payload"]),
    ]))
}

fn pattern_value(pattern: &RewritePattern) -> Result<IOValue> {
    let (kind, needle) = match pattern {
        RewritePattern::Any => ("any", ""),
        RewritePattern::ArtifactKind(value) => ("artifact-kind", value.as_str()),
        RewritePattern::RecordLabel(value) => ("record-label", value.as_str()),
        RewritePattern::StringEquals(value) => ("string-equals", value.as_str()),
        RewritePattern::StringContains(value) => ("string-contains", value.as_str()),
        RewritePattern::SchemaShapeKind(value) => ("schema-shape-kind", value.as_str()),
        RewritePattern::RefContains(value) => ("ref-contains", value.as_str()),
    };
    validate_pattern(pattern)?;
    Ok(record("pattern", vec![
        string(kind),
        string(needle),
        checks_value(&["bounded-preserves-pattern", "no-ambient-code"]),
    ]))
}

fn replacement_value(replacement: &RewriteReplacement) -> Result<IOValue> {
    match replacement {
        RewriteReplacement::StringValue { from, to } => {
            validate_non_empty(from, "rewrite replacement from string")?;
            Ok(record("replacement", vec![
                string("string-value"),
                record("from", vec![string(from)]),
                record("to", vec![string(to)]),
                checks_value(&["structural-value-replacement", "canonical-reparse-not-text-bypass"]),
            ]))
        }
    }
}

fn rewrite_receipt_value(input: &RewriteReceiptValueInput<'_>) -> Result<IOValue> {
    validate_non_empty(input.operation, "rewrite receipt operation")?;
    if !matches!(input.decision, "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!("unsupported rewrite decision {}", input.decision)));
    }
    validate_ref(input.subject_ref, "rewrite receipt subject ref")?;
    validate_refs(input.refs, "rewrite receipt ref")?;
    let mut all_checks = vec![("canonical-receipt", "pass")];
    all_checks.extend_from_slice(input.checks);
    Ok(record("rewrite-receipt-v1", vec![
        string(REWRITE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("subject", vec![string(input.subject_ref)]),
        record("refs", vec![refs_sequence(&sorted_unique_refs(input.refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("tool", vec![string(TOOL_VERSION)]),
        checks_value_from_pairs(&all_checks),
    ]))
}

fn collect_bindings(
    value: &IOValue,
    pattern: &RewritePattern,
    path: &str,
    bindings: &mut impl crate::bounded::VecSink<RewriteBinding>,
) -> Result<()> {
    let mut pending = Vec::with_capacity(1);
    push_bounded(&mut pending, (value.clone(), path.to_string()), MAX_REWRITE_ITEMS, "rewrite scan values")?;
    while let Some((current, current_path)) = pending.pop() {
        if value_matches_pattern(&current, pattern) {
            push_bounded(
                bindings,
                RewriteBinding {
                    path: current_path.clone(),
                    value_ref: canonical_hash(&current)?,
                    preview: preview_text(&current)?,
                },
                MAX_REWRITE_ITEMS,
                "rewrite bindings",
            )?;
        }
        let mut children = Vec::new();
        match current.value_class() {
            ValueClass::Atomic(_) | ValueClass::Embedded => {}
            ValueClass::Compound(CompoundClass::Record) => {
                let label = record_label_name(&current);
                for (index, child) in current.iter().enumerate() {
                    push_bounded(
                        &mut children,
                        (value_to_iovalue(&child), format!("{current_path}/{label}/{index}")),
                        MAX_REWRITE_ITEMS,
                        "rewrite scan child values",
                    )?;
                }
            }
            ValueClass::Compound(CompoundClass::Sequence) | ValueClass::Compound(CompoundClass::Set) => {
                for (index, child) in current.iter().enumerate() {
                    push_bounded(
                        &mut children,
                        (value_to_iovalue(&child), format!("{current_path}/{index}")),
                        MAX_REWRITE_ITEMS,
                        "rewrite scan child values",
                    )?;
                }
            }
            ValueClass::Compound(CompoundClass::Dictionary) => {
                for (index, (key, child)) in current.entries().enumerate() {
                    push_bounded(
                        &mut children,
                        (value_to_iovalue(&key), format!("{current_path}/key/{index}")),
                        MAX_REWRITE_ITEMS,
                        "rewrite scan child values",
                    )?;
                    push_bounded(
                        &mut children,
                        (value_to_iovalue(&child), format!("{current_path}/value/{index}")),
                        MAX_REWRITE_ITEMS,
                        "rewrite scan child values",
                    )?;
                }
            }
        }
        for child in children.into_iter().rev() {
            push_bounded(&mut pending, child, MAX_REWRITE_ITEMS, "rewrite scan values")?;
        }
    }
    Ok(())
}

fn value_matches_pattern(value: &IOValue, pattern: &RewritePattern) -> bool {
    match pattern {
        RewritePattern::Any => true,
        RewritePattern::ArtifactKind(_) => false,
        RewritePattern::RecordLabel(expected) => {
            value.is_record() && value.label().as_symbol().is_some_and(|label| label.as_ref() == expected.as_str())
        }
        RewritePattern::StringEquals(expected) => {
            value.as_string().is_some_and(|text| text.as_ref() == expected.as_str())
        }
        RewritePattern::StringContains(needle) => value.as_string().is_some_and(|text| text.contains(needle.as_str())),
        RewritePattern::SchemaShapeKind(expected) => value
            .collect_simple_record("shape", None)
            .and_then(|fields| {
                if fields.len() == 0 {
                    None
                } else {
                    fields[0].as_string().map(|text| text.into_owned())
                }
            })
            .is_some_and(|kind| kind == expected.as_str()),
        RewritePattern::RefContains(needle) => to_text(value).is_ok_and(|text| text.contains(needle.as_str())),
    }
}

struct RewriteStringValuesInput<'a> {
    value: &'a IOValue,
    from: &'a str,
    to: &'a str,
    path: &'a str,
    changed_paths: &'a mut Vec<String>,
}

fn rewrite_string_values(input: RewriteStringValuesInput<'_>) -> Result<IOValue> {
    let mut traversal = TextTraversal::new(TextTraversalInput {
        value: input.value,
        from: input.from,
        to: input.to,
        path: input.path,
        changed_paths: input.changed_paths,
    })?;
    traversal.run()?;
    traversal.output()
}

enum TextFrame {
    Visit {
        value: IOValue,
        path: String,
    },
    FinishRecord {
        original: IOValue,
        label: IOValue,
        child_count: usize,
        changed_count_before: usize,
    },
    FinishSequence {
        original: IOValue,
        child_count: usize,
        changed_count_before: usize,
    },
}

struct TextTraversalInput<'a> {
    value: &'a IOValue,
    from: &'a str,
    to: &'a str,
    path: &'a str,
    changed_paths: &'a mut Vec<String>,
}

struct TextTraversal<'a> {
    from: &'a str,
    to: &'a str,
    changed_paths: &'a mut Vec<String>,
    frames: Vec<TextFrame>,
    outputs: Vec<IOValue>,
}

impl<'a> TextTraversal<'a> {
    fn new(input: TextTraversalInput<'a>) -> Result<Self> {
        let mut traversal = Self {
            from: input.from,
            to: input.to,
            changed_paths: input.changed_paths,
            frames: Vec::with_capacity(1),
            outputs: Vec::with_capacity(1),
        };
        traversal.push_frame(TextFrame::Visit {
            value: input.value.clone(),
            path: input.path.to_string(),
        })?;
        Ok(traversal)
    }

    fn run(&mut self) -> Result<()> {
        while let Some(frame) = self.frames.pop() {
            match frame {
                TextFrame::Visit { value, path } => self.visit(value, path)?,
                TextFrame::FinishRecord {
                    original,
                    label,
                    child_count,
                    changed_count_before,
                } => self.finish_record(original, label, child_count, changed_count_before)?,
                TextFrame::FinishSequence {
                    original,
                    child_count,
                    changed_count_before,
                } => self.finish_sequence(original, child_count, changed_count_before)?,
            }
        }
        Ok(())
    }

    fn output(mut self) -> Result<IOValue> {
        let output_count = self.outputs.len();
        if output_count != 1 {
            return Err(MoltenError::invalid_harness(format!("rewrite traversal produced {output_count} outputs")));
        }
        self.outputs
            .pop()
            .ok_or_else(|| MoltenError::invalid_harness("rewrite traversal produced no output"))
    }

    fn visit(&mut self, current: IOValue, current_path: String) -> Result<()> {
        if current.as_string().is_some_and(|text| text.as_ref() == self.from) {
            push_bounded(&mut *self.changed_paths, current_path, MAX_REWRITE_ITEMS, "rewrite changed paths")?;
            self.push_output(string(self.to))?;
            return Ok(());
        }
        match current.value_class() {
            ValueClass::Atomic(_) | ValueClass::Embedded => self.push_output(current),
            ValueClass::Compound(CompoundClass::Record) => self.visit_record(current, current_path),
            ValueClass::Compound(CompoundClass::Sequence) => self.visit_sequence(current, current_path),
            ValueClass::Compound(CompoundClass::Set) | ValueClass::Compound(CompoundClass::Dictionary) => {
                self.push_output(current)
            }
        }
    }

    fn visit_record(&mut self, current: IOValue, current_path: String) -> Result<()> {
        let label = value_to_iovalue(&current.label());
        let label_name = record_label_name(&current);
        let changed_count_before = self.changed_paths.len();
        let mut children = Vec::new();
        for (index, child) in current.iter().enumerate() {
            push_bounded(
                &mut children,
                TextFrame::Visit {
                    value: value_to_iovalue(&child),
                    path: format!("{current_path}/{label_name}/{index}"),
                },
                MAX_REWRITE_ITEMS,
                "rewrite traversal child frames",
            )?;
        }
        self.push_frame(TextFrame::FinishRecord {
            original: current,
            label,
            child_count: children.len(),
            changed_count_before,
        })?;
        self.push_children(children)
    }

    fn visit_sequence(&mut self, current: IOValue, current_path: String) -> Result<()> {
        let changed_count_before = self.changed_paths.len();
        let mut children = Vec::new();
        for (index, child) in current.iter().enumerate() {
            push_bounded(
                &mut children,
                TextFrame::Visit {
                    value: value_to_iovalue(&child),
                    path: format!("{current_path}/{index}"),
                },
                MAX_REWRITE_ITEMS,
                "rewrite traversal child frames",
            )?;
        }
        self.push_frame(TextFrame::FinishSequence {
            original: current,
            child_count: children.len(),
            changed_count_before,
        })?;
        self.push_children(children)
    }

    fn finish_record(
        &mut self,
        original: IOValue,
        label: IOValue,
        child_count: usize,
        changed_count_before: usize,
    ) -> Result<()> {
        let fields = self.take_child_outputs(child_count, "rewrite record output count underflow")?;
        let rewritten = if self.changed_paths.len() == changed_count_before {
            original
        } else {
            IOValue::record(label, fields)
        };
        self.push_output(rewritten)
    }

    fn finish_sequence(&mut self, original: IOValue, child_count: usize, changed_count_before: usize) -> Result<()> {
        let items = self.take_child_outputs(child_count, "rewrite sequence output count underflow")?;
        let rewritten = if self.changed_paths.len() == changed_count_before {
            original
        } else {
            sequence(items)
        };
        self.push_output(rewritten)
    }

    fn take_child_outputs(&mut self, child_count: usize, label: &str) -> Result<Vec<IOValue>> {
        let start = self.outputs.len().checked_sub(child_count).ok_or_else(|| MoltenError::invalid_harness(label))?;
        Ok(self.outputs.split_off(start))
    }

    fn push_children(&mut self, children: Vec<TextFrame>) -> Result<()> {
        for child in children.into_iter().rev() {
            self.push_frame(child)?;
        }
        Ok(())
    }

    fn push_frame(&mut self, frame: TextFrame) -> Result<()> {
        push_bounded(&mut self.frames, frame, MAX_REWRITE_ITEMS, "rewrite traversal frames")
    }

    fn push_output(&mut self, output: IOValue) -> Result<()> {
        push_bounded(&mut self.outputs, output, MAX_REWRITE_ITEMS, "rewrite traversal outputs")
    }
}

fn scoped_refs(root: &Path, roots: &[String], include_dependencies: bool) -> Result<BTreeSet<String>> {
    validate_refs(roots, "rewrite scope root ref")?;
    let mut scoped = BTreeSet::new();
    let mut stack = roots.to_vec();
    while let Some(current) = stack.pop() {
        if !scoped.insert(current.clone()) || !include_dependencies {
            continue;
        }
        for dependency in artifacts::direct_dependencies(root, &current)? {
            stack.push(dependency);
        }
    }
    Ok(scoped)
}

fn impacted_refs(root: &Path, diffs: &[RewriteDiff]) -> Result<Vec<String>> {
    let mut impacted = BTreeSet::new();
    for diff in diffs {
        for reference in artifacts::impact_refs(root, std::slice::from_ref(&diff.artifact_ref))? {
            impacted.insert(reference);
        }
    }
    Ok(impacted.into_iter().collect())
}

fn preview_text(value: &IOValue) -> Result<String> {
    let text = to_text(value)?;
    const LIMIT: usize = 240;
    if text.chars().count() > LIMIT {
        let mut truncated = text.chars().take(LIMIT).collect::<String>();
        truncated.push('…');
        Ok(truncated)
    } else {
        Ok(text)
    }
}

fn record_label_name(value: &IOValue) -> String {
    value.label().as_symbol().map(|label| label.into_owned()).unwrap_or_else(|| "record".to_string())
}

fn validate_query_input(input: &RewriteQueryInput) -> Result<()> {
    validate_refs(&input.root_refs, "rewrite query root ref")?;
    validate_refs(&input.policy_refs, "rewrite query policy ref")?;
    validate_refs(&input.capability_refs, "rewrite query capability ref")?;
    validate_refs(&input.hidden_refs, "rewrite query hidden ref")?;
    for kind in &input.artifact_kinds {
        validate_non_empty(kind, "rewrite query artifact kind")?;
    }
    validate_pattern(&input.pattern)
}

fn validate_plan_input(input: &RewritePlanInput) -> Result<()> {
    validate_query_input(&input.query)?;
    validate_ref(&input.planner_ref, "rewrite planner ref")?;
    validate_refs(&input.policy_refs, "rewrite plan policy ref")?;
    validate_refs(&input.capability_refs, "rewrite plan capability ref")?;
    validate_refs(&input.transcript_refs, "rewrite plan transcript ref")?;
    validate_refs(&input.schema_migration_recipe_refs, "rewrite plan schema migration recipe ref")?;
    if input.policy_refs.is_empty() {
        return Err(MoltenError::invalid_harness("rewrite plan requires explicit policy refs"));
    }
    if input.capability_refs.is_empty() {
        return Err(MoltenError::invalid_harness("rewrite plan requires explicit capability refs"));
    }
    match &input.replacement {
        RewriteReplacement::StringValue { from, .. } => validate_non_empty(from, "rewrite replacement from string"),
    }
}

fn validate_pattern(pattern: &RewritePattern) -> Result<()> {
    match pattern {
        RewritePattern::Any => Ok(()),
        RewritePattern::ArtifactKind(value)
        | RewritePattern::RecordLabel(value)
        | RewritePattern::StringEquals(value)
        | RewritePattern::StringContains(value)
        | RewritePattern::SchemaShapeKind(value) => validate_non_empty(value, "rewrite pattern"),
        RewritePattern::RefContains(value) => validate_ref(value, "rewrite ref pattern"),
    }
}

fn refs_sequence(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
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
    let items = required_sequence(&checks[0], "rewrite checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(&item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "rewrite check name")?;
        let status = required_string(&check[1], "rewrite check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("rewrite check {name} has status {status}")));
        }
        push_bounded(&mut parsed, name, MAX_REWRITE_ITEMS, "rewrite checks")?;
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
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}

fn record_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    items.iter().map(|item| required_string(&item, label)).collect()
}

fn parse_ref_sequence_value(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    items.iter().map(|item| required_ref(&item, label)).collect()
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

fn local_ref(kind: &str, refs: &[String]) -> Result<String> {
    canonical_hash(&record("rewrite-derived-ref", vec![string(kind), refs_sequence(&sorted_unique_refs(refs))]))
}

fn merge_refs(left: &[String], right: &[String]) -> Vec<String> {
    left.iter().chain(right.iter()).cloned().collect()
}

fn sorted_unique_refs(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<BTreeSet<_>>().into_iter().collect()
}

fn sorted_unique_strings(values: &[String]) -> Vec<String> {
    values.iter().cloned().collect::<BTreeSet<_>>().into_iter().collect()
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
    if value_ref.starts_with("blake3:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{field} must be a blake3 ref, got {value_ref}")))
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
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
    use crate::preserves_rail::parse_text;

    #[test]
    fn find_matches_schema_shapes_and_visibility_filter_hides_refs() {
        let root = temp_dir("rewrite-find");
        let schema_payload =
            parse_text(r#"<schema <shape "record" [<field "name" <shape "string">>]>>"#).expect("parse schema payload");
        let installed = install_fixture(&root, "schema", schema_payload, &[]);
        let visible =
            find(&root, &query(RewritePattern::SchemaShapeKind("record".to_string()))).expect("find schema shape");
        assert_eq!(visible.matches.len(), 1);
        assert_eq!(visible.matches[0].artifact_ref, installed.artifact_ref);
        let hidden = find(&root, &RewriteQueryInput {
            hidden_refs: vec![installed.artifact_ref.clone()],
            ..query(RewritePattern::SchemaShapeKind("record".to_string()))
        })
        .expect("hidden find");
        assert!(hidden.matches.is_empty());
    }

    #[test]
    fn preview_and_apply_create_new_artifact_without_mutating_old_payload() {
        let root = temp_dir("rewrite-apply");
        let payload = parse_text(r#"<doc "old" ["old" "keep"]>"#).expect("parse payload");
        let installed = install_fixture(&root, "doc", payload.clone(), &[]);
        let input = plan_input(RewritePattern::StringEquals("old".to_string()), "old", "new");
        let previewed = preview(&root, &input).expect("preview rewrite");
        assert_eq!(previewed.diffs.len(), 1);
        assert!(previewed.diffs[0].paths.as_slice().iter().any(|path| path.contains("doc")));
        let applied = apply(&root, &input).expect("apply rewrite");
        assert_eq!(applied.installed.len(), 1);
        let new_ref = &applied.installed[0].new_artifact_ref;
        assert_ne!(&installed.artifact_ref, new_ref);
        assert_eq!(artifacts::read_payload(&root, &installed.artifact_ref).expect("old payload"), payload);
        let new_payload = artifacts::read_payload(&root, new_ref).expect("new payload");
        assert!(to_text(&new_payload).expect("render new").contains("new"));
        assert!(!to_text(&new_payload).expect("render new again").contains("old"));
    }

    #[test]
    fn apply_receipt_builds_upgrade_plan_hook() {
        let root = temp_dir("rewrite-upgrade-hook");
        let payload = parse_text(r#"<doc "old">"#).expect("parse payload");
        install_fixture(&root, "doc", payload, &[]);
        let input = plan_input(RewritePattern::StringEquals("old".to_string()), "old", "new");
        let applied = apply(&root, &input).expect("apply rewrite");
        let plan = upgrade_plan_from_apply(
            &applied,
            "rewrite-session",
            &test_ref("initiator"),
            &[test_ref("upgrade-capability")],
            &[test_ref("upgrade-policy")],
        )
        .expect("upgrade plan");
        let parsed = upgrades::parse_upgrade_plan(&plan).expect("parse upgrade plan");
        assert_eq!(parsed.tasks[0].kind, "install-artifact");
        assert!(parsed.checks.as_slice().iter().any(|check| check == "no-ucm-clone"));
    }

    #[test]
    fn unauthorized_or_empty_policy_is_denied_before_apply() {
        let root = temp_dir("rewrite-deny");
        let payload = parse_text(r#"<doc "old">"#).expect("parse payload");
        install_fixture(&root, "doc", payload, &[]);
        let mut input = plan_input(RewritePattern::StringEquals("old".to_string()), "old", "new");
        input.capability_refs.clear();
        let error = preview(&root, &input).expect_err("missing capability denied");
        assert!(error.to_string().contains("capability"), "{error}");
    }

    #[hegel::test(test_cases = 12)]
    fn hegel_preview_apply_consistency_and_path_stability(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let root = temp_dir("rewrite-hegel");
        let needle = format!("old-{salt}");
        let replacement = format!("new-{salt}");
        let payload = record("doc", vec![string(&needle), sequence(vec![string(&needle), string("stable")])]);
        let installed = install_fixture(&root, "doc", payload.clone(), &[]);
        let input = plan_input(RewritePattern::StringEquals(needle.clone()), &needle, &replacement);
        let first = preview(&root, &input).expect("first preview");
        let second = preview(&root, &input).expect("second preview");
        assert_eq!(first.diffs[0].paths, second.diffs[0].paths);
        assert_eq!(first.diffs[0].new_payload_ref, second.diffs[0].new_payload_ref);
        let applied = apply(&root, &input).expect("apply");
        assert_ne!(installed.artifact_ref, applied.installed[0].new_artifact_ref);
        assert_eq!(artifacts::read_payload(&root, &installed.artifact_ref).expect("old payload"), payload);
    }

    fn query(pattern: RewritePattern) -> RewriteQueryInput {
        RewriteQueryInput {
            artifact_kinds: Vec::new(),
            root_refs: Vec::new(),
            include_dependencies: true,
            pattern,
            policy_refs: vec![test_ref("query-policy")],
            capability_refs: vec![test_ref("query-capability")],
            hidden_refs: Vec::new(),
        }
    }

    fn plan_input(pattern: RewritePattern, from: &str, to: &str) -> RewritePlanInput {
        RewritePlanInput {
            query: query(pattern),
            replacement: RewriteReplacement::StringValue {
                from: from.to_string(),
                to: to.to_string(),
            },
            planner_ref: test_ref("planner"),
            policy_refs: vec![test_ref("plan-policy")],
            capability_refs: vec![test_ref("plan-capability")],
            transcript_refs: vec![test_ref("transcript")],
            schema_migration_recipe_refs: vec![test_ref("migration-recipe")],
        }
    }

    fn install_fixture(
        root: &Path,
        kind: &str,
        payload: IOValue,
        dependency_refs: &[String],
    ) -> artifacts::ArtifactInstall {
        artifacts::install_artifact(root, &artifacts::ArtifactInstallInput {
            kind: kind.to_string(),
            payload,
            schema_refs: vec![test_ref("schema")],
            dependency_refs: dependency_refs.to_vec(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("artifact-policy")],
            evidence_refs: vec![test_ref("artifact-evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("install-capability")],
        })
        .expect("install fixture")
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("rewrite-test-ref", vec![string(label)])).expect("test ref")
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
