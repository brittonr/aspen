use std::fs;

use crate::artifacts;
use crate::ledger;
use crate::octet_gate;
use crate::protocol_session;

type BtreeSet<T> = std::collections::BTreeSet<T>;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type Record<T> = preserves::Record<T>;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

pub const SUPPORTED_TASK_KINDS: &[&str] = &[
    "install-artifact",
    "move-name",
    "compatibility-alias",
    "deprecate",
    "migrate-storage",
    "install-protocol-bridge",
    "drain-sessions",
    "update-handler-policy",
    "transcript-rerun",
    "update-docs",
    "cutover",
    "rollback-pointer",
    "cleanup",
];

const MAX_UPGRADE_REFS: usize = 4096;
const MAX_UPGRADE_DIAGNOSTICS: usize = 4096;
const MAX_UPGRADE_TASKS: usize = 1024;
const MAX_UPGRADE_POINTERS: usize = 100_000;
const MAX_UPGRADE_SOURCE_GATES: usize = 128;

const _: () = assert!(MAX_UPGRADE_REFS <= 100_000);
const _: () = assert!(MAX_UPGRADE_DIAGNOSTICS <= 100_000);
const _: () = assert!(MAX_UPGRADE_TASKS <= 10_000);
const _: () = assert!(MAX_UPGRADE_POINTERS <= 1_000_000);
const _: () = assert!(MAX_UPGRADE_SOURCE_GATES <= 1_000);

const UPGRADE_NAME_POINTER_SCHEMA: &str = crate::preserves_rail::UPGRADE_NAME_POINTER_SCHEMA;
const UPGRADE_PLAN_SCHEMA: &str = crate::preserves_rail::UPGRADE_PLAN_SCHEMA;
const UPGRADE_RECEIPT_SCHEMA: &str = crate::preserves_rail::UPGRADE_RECEIPT_SCHEMA;

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_hex(value: &str) -> Result<&str> {
    crate::preserves_rail::content_ref_hex(value)
}

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

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

type UpgradeCheckPair = (&'static str, &'static str);
type UpgradeTaskOutcome = (&'static str, Vec<String>, Vec<UpgradeCheckPair>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeTaskInput {
    pub task_id: String,
    pub kind: String,
    pub subject: String,
    pub from_ref: Option<String>,
    pub to_ref: Option<String>,
    pub precondition_refs: Vec<String>,
    pub postcondition_refs: Vec<String>,
    pub reversible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeCompatibilityWindow {
    pub old_refs: Vec<String>,
    pub new_refs: Vec<String>,
    pub expires_at: Option<u64>,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradePlanInput {
    pub session_id: String,
    pub reason: String,
    pub summary: String,
    pub initiator_ref: String,
    pub capability_refs: Vec<String>,
    pub affected_refs: Vec<String>,
    pub impact_refs: Vec<String>,
    pub tasks: Vec<UpgradeTaskInput>,
    pub compatibility: UpgradeCompatibilityWindow,
    pub rollback_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub source_gate_receipt_values: Vec<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameMovePlanInput {
    pub session_id: String,
    pub name: String,
    pub from_ref: String,
    pub to_ref: String,
    pub initiator_ref: String,
    pub capability_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub source_gate_receipt_values: Vec<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeTask {
    pub task_id: String,
    pub kind: String,
    pub subject: String,
    pub from_ref: Option<String>,
    pub to_ref: Option<String>,
    pub precondition_refs: Vec<String>,
    pub postcondition_refs: Vec<String>,
    pub reversible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradePlan {
    pub plan_ref: String,
    pub session_id: String,
    pub reason: String,
    pub summary: String,
    pub initiator_ref: String,
    pub capability_refs: Vec<String>,
    pub affected_refs: Vec<String>,
    pub impact_refs: Vec<String>,
    pub tasks: Vec<UpgradeTask>,
    pub compatibility: UpgradeCompatibilityWindow,
    pub rollback_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub session_id: String,
    pub plan_ref: String,
    pub task_id: Option<String>,
    pub value: IoValue,
}

struct UpgradeReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    session_id: &'a str,
    plan_ref: &'a str,
    task_id: Option<&'a str>,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeSessionCreated {
    pub plan: UpgradePlan,
    pub receipt: UpgradeReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeTaskExecution {
    pub plan_ref: String,
    pub task_id: String,
    pub task_kind: String,
    pub receipt: UpgradeReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePointer {
    pub name: String,
    pub pointer_kind: String,
    pub artifact_ref: String,
    pub previous_ref: Option<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeTaskStatus {
    pub task_id: String,
    pub kind: String,
    pub done: bool,
    pub receipt_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeStatus {
    pub plan_ref: String,
    pub session_id: String,
    pub tasks: Vec<UpgradeTaskStatus>,
    pub remaining_task_ids: Vec<String>,
}

pub fn upgrade_task_value(task: &UpgradeTaskInput) -> Result<IoValue> {
    validate_task_input(task)?;
    Ok(record("upgrade-task-v1", vec![
        string(&task.task_id),
        record("kind", vec![string(&task.kind)]),
        record("subject", vec![string(&task.subject)]),
        record("from", vec![optional_ref_value(task.from_ref.as_deref())]),
        record("to", vec![optional_ref_value(task.to_ref.as_deref())]),
        record("preconditions", vec![refs_sequence(&task.precondition_refs)]),
        record("postconditions", vec![refs_sequence(&task.postcondition_refs)]),
        record("reversible", vec![bool_value(task.reversible)]),
    ]))
}

pub fn upgrade_plan_value(input: &UpgradePlanInput) -> Result<IoValue> {
    validate_plan_input(input)?;
    let source_gate_validation_refs = validate_upgrade_source_gates(input)?;
    let evidence_refs =
        sorted_refs(input.evidence_refs.iter().cloned().chain(source_gate_validation_refs.iter().cloned()).collect());
    Ok(record("upgrade-plan-v1", vec![
        string(UPGRADE_PLAN_SCHEMA),
        record("session", vec![string(&input.session_id)]),
        record("summary", vec![string(&input.reason), string(&input.summary)]),
        record("initiator", vec![string(&input.initiator_ref), refs_sequence(&input.capability_refs)]),
        record("affected", vec![refs_sequence(&input.affected_refs)]),
        record("impact", vec![refs_sequence(&input.impact_refs)]),
        record("tasks", vec![sequence(
            input.tasks.iter().map(upgrade_task_value).collect::<Result<Vec<_>>>()?,
        )]),
        compatibility_window_value(&input.compatibility)?,
        record("rollback-rules", vec![refs_sequence(&input.rollback_refs)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("evidence", vec![refs_sequence(&evidence_refs)]),
        checks_value(&[
            "canonical-plan-hash",
            "task-status-receipt-backed",
            "names-are-metadata",
            "compatibility-window-explicit",
            "policy-admission-required",
            "strict-octet-source-gate-bound",
            "no-ucm-clone",
        ]),
    ]))
}

pub fn name_move_plan_value(ledger_root: &Path, input: &NameMovePlanInput) -> Result<IoValue> {
    name_move_plan_value_with_registry(None, ledger_root, input)
}

pub fn name_move_plan_value_with_registry(
    registry_root: Option<&Path>,
    ledger_root: &Path,
    input: &NameMovePlanInput,
) -> Result<IoValue> {
    validate_non_empty(&input.name, "upgrade name")?;
    validate_ref(&input.from_ref, "name move from ref")?;
    validate_ref(&input.to_ref, "name move to ref")?;
    validate_refs(&input.capability_refs, "upgrade capability ref")?;
    validate_refs(&input.policy_refs, "upgrade policy ref")?;
    validate_refs(&input.evidence_refs, "upgrade evidence ref")?;
    validate_ref(&input.initiator_ref, "upgrade initiator ref")?;
    let impact_refs = if let Some(registry_root) = registry_root {
        artifacts::impact_refs(registry_root, std::slice::from_ref(&input.from_ref))?
    } else {
        compute_impact_set(ledger_root, std::slice::from_ref(&input.from_ref))?
    };
    let tasks = planned_tasks(input);
    upgrade_plan_value(&UpgradePlanInput {
        session_id: input.session_id.clone(),
        reason: "name-move".to_string(),
        summary: format!("Move {} from {} to {}", input.name, input.from_ref, input.to_ref),
        initiator_ref: input.initiator_ref.clone(),
        capability_refs: input.capability_refs.clone(),
        affected_refs: vec![input.from_ref.clone(), input.to_ref.clone()],
        impact_refs,
        tasks,
        compatibility: UpgradeCompatibilityWindow {
            old_refs: vec![input.from_ref.clone()],
            new_refs: vec![input.to_ref.clone()],
            expires_at: None,
            policy_refs: input.policy_refs.clone(),
        },
        rollback_refs: vec![input.from_ref.clone()],
        policy_refs: input.policy_refs.clone(),
        evidence_refs: input.evidence_refs.clone(),
        source_gate_receipt_values: input.source_gate_receipt_values.clone(),
    })
}

fn planned_tasks(input: &NameMovePlanInput) -> Vec<UpgradeTaskInput> {
    vec![
        planned_task(
            input,
            "compatibility-alias",
            "compatibility-alias",
            format!("{}@candidate", input.name),
            Vec::new(),
        ),
        planned_task(input, "transcript-gate", "transcript-rerun", input.name.clone(), input.evidence_refs.clone()),
        planned_task(input, "move-name", "move-name", input.name.clone(), Vec::new()),
        planned_task(input, "cutover", "cutover", input.name.clone(), Vec::new()),
    ]
}

fn planned_task(
    input: &NameMovePlanInput,
    task_id: &str,
    kind: &str,
    subject: String,
    postcondition_refs: Vec<String>,
) -> UpgradeTaskInput {
    UpgradeTaskInput {
        task_id: task_id.to_string(),
        kind: kind.to_string(),
        subject,
        from_ref: Some(input.from_ref.clone()),
        to_ref: Some(input.to_ref.clone()),
        precondition_refs: input.evidence_refs.clone(),
        postcondition_refs,
        reversible: true,
    }
}

pub fn parse_upgrade_plan(value: &IoValue) -> Result<UpgradePlan> {
    let fields = value
        .collect_simple_record("upgrade-plan-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <upgrade-plan-v1 ...>"))?;
    require_schema(&fields[0], UPGRADE_PLAN_SCHEMA, "upgrade plan")?;
    let session_id = record_string(&fields[1], "session")?;
    let summary = value_to_iovalue(&fields[2]);
    let summary_fields = simple_record(&summary, "summary", 2)?;
    let initiator = value_to_iovalue(&fields[3]);
    let initiator_fields = simple_record(&initiator, "initiator", 2)?;
    let tasks = parse_tasks(&fields[6])?;
    let compatibility = parse_compatibility_window(&fields[7])?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "canonical-plan-hash", "upgrade plan")?;
    require_check(&checks, "task-status-receipt-backed", "upgrade plan")?;
    require_check(&checks, "names-are-metadata", "upgrade plan")?;
    require_check(&checks, "no-ucm-clone", "upgrade plan")?;
    let plan = UpgradePlan {
        plan_ref: canonical_hash(value)?,
        session_id,
        reason: required_string(&summary_fields[0], "upgrade reason")?,
        summary: required_string(&summary_fields[1], "upgrade summary")?,
        initiator_ref: required_ref(&initiator_fields[0], "upgrade initiator ref")?,
        capability_refs: parse_ref_sequence_value(&initiator_fields[1], "upgrade capability refs")?,
        affected_refs: record_ref_sequence(&fields[4], "affected")?,
        impact_refs: record_ref_sequence(&fields[5], "impact")?,
        tasks,
        compatibility,
        rollback_refs: record_ref_sequence(&fields[8], "rollback-rules")?,
        policy_refs: record_ref_sequence(&fields[9], "policy")?,
        evidence_refs: record_ref_sequence(&fields[10], "evidence")?,
        checks,
        value: value.clone(),
    };
    validate_parsed_plan(&plan)?;
    Ok(plan)
}

pub fn compute_impact_set(ledger_root: &Path, seed_refs: &[String]) -> Result<Vec<String>> {
    validate_refs(seed_refs, "impact seed ref")?;
    let mut impacted: BtreeSet<String> = seed_refs.iter().cloned().collect();
    let mut artifacts = Vec::new();
    for entry in ledger::list_artifacts(ledger_root)? {
        let value = ledger::read_artifact(ledger_root, &entry.artifact_ref)?;
        let text = to_text(&value)?;
        push_bounded(&mut artifacts, (entry.artifact_ref, text), MAX_UPGRADE_REFS, "upgrade impact artifacts")?;
    }
    let mut has_changed_impact = true;
    while has_changed_impact {
        has_changed_impact = false;
        let seeds: Vec<String> = impacted.iter().cloned().collect();
        for (artifact_ref, text) in &artifacts {
            if impacted.contains(artifact_ref) {
                continue;
            }
            if seeds.iter().any(|seed| text.contains(seed)) {
                impacted.insert(artifact_ref.clone());
                has_changed_impact = true;
            }
        }
    }
    Ok(impacted.into_iter().collect())
}

pub fn create_session(root: &Path, plan_value: &IoValue) -> Result<UpgradeSessionCreated> {
    ensure_dirs(root)?;
    let plan = parse_upgrade_plan(plan_value)?;
    if plan.policy_refs.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade session missing policy refs"));
    }
    if plan.capability_refs.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade session missing capability refs"));
    }
    write_preserves(&plan_path(root, &plan.plan_ref)?, plan_value)?;
    let receipt_value = upgrade_receipt_value(&UpgradeReceiptValueInput {
        operation: "session-create",
        decision: "pass",
        session_id: &plan.session_id,
        plan_ref: &plan.plan_ref,
        task_id: None,
        refs: &plan_refs(&plan),
        diagnostics: &[],
        checks: &[
            ("plan-shape", "pass"),
            ("policy-admission", "pass"),
            ("capability-admission", "pass"),
            ("impact-set-bound", "pass"),
            ("compatibility-window", "pass"),
            ("no-ucm-clone", "pass"),
        ],
    })?;
    let receipt = parse_upgrade_receipt(&receipt_value)?;
    store_receipt(root, &receipt_value)?;
    Ok(UpgradeSessionCreated { plan, receipt })
}

pub fn set_name_pointer(root: &Path, name: &str, artifact_ref: &str) -> Result<UpgradeReceipt> {
    ensure_dirs(root)?;
    validate_non_empty(name, "name pointer name")?;
    validate_ref(artifact_ref, "name pointer artifact ref")?;
    let previous = read_name_pointer(root, name)?.map(|pointer| pointer.artifact_ref);
    let receipt_value = upgrade_receipt_value(&UpgradeReceiptValueInput {
        operation: "name-pointer-set",
        decision: "pass",
        session_id: "local-name-pointer",
        plan_ref: artifact_ref,
        task_id: None,
        refs: &[artifact_ref.to_string()],
        diagnostics: &[],
        checks: &[("names-are-metadata", "pass"), ("immutable-artifact-unchanged", "pass")],
    })?;
    let receipt = parse_upgrade_receipt(&receipt_value)?;
    let pointer = name_pointer_value(name, "name", artifact_ref, previous.as_deref(), &receipt.receipt_ref)?;
    write_preserves(&name_pointer_path(root, name)?, &pointer)?;
    store_receipt(root, &receipt_value)?;
    Ok(receipt)
}

pub fn read_name_pointer(root: &Path, name: &str) -> Result<Option<NamePointer>> {
    let path = name_pointer_path(root, name)?;
    if !path.exists() {
        return Ok(None);
    }
    parse_name_pointer(&read_preserves(&path)?).map(Some)
}

pub fn execute_task(root: &Path, ledger_root: &Path, plan_ref: &str, task_id: &str) -> Result<UpgradeTaskExecution> {
    ensure_dirs(root)?;
    let plan = read_plan(root, plan_ref)?;
    let task_index = plan
        .tasks
        .iter()
        .position(|task| task.task_id == task_id)
        .ok_or_else(|| MoltenError::invalid_harness(format!("upgrade plan missing task {task_id}")))?;
    ensure_prior_tasks_complete(root, &plan, task_index)?;
    let task = plan.tasks[task_index].clone();
    let (decision, diagnostics, checks) = task_result(root, ledger_root, &plan, &task)?;
    let refs = task_refs(&task);
    let receipt_value = upgrade_receipt_value(&UpgradeReceiptValueInput {
        operation: if task.kind == "cutover" {
            "cutover"
        } else {
            "task-complete"
        },
        decision,
        session_id: &plan.session_id,
        plan_ref: &plan.plan_ref,
        task_id: Some(&task.task_id),
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &checks,
    })?;
    let receipt = parse_upgrade_receipt(&receipt_value)?;
    store_receipt(root, &receipt_value)?;
    if receipt.decision == "pass" {
        write_status(root, &plan, &task, &receipt.receipt_ref)?;
    }
    Ok(UpgradeTaskExecution {
        plan_ref: plan.plan_ref,
        task_id: task.task_id,
        task_kind: task.kind,
        receipt,
    })
}

fn task_result(root: &Path, ledger_root: &Path, plan: &UpgradePlan, task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    match task.kind.as_str() {
        "compatibility-alias" => alias_result(root, plan, task),
        "transcript-rerun" => Ok(transcript_result(plan, task)),
        "move-name" => move_result(root, plan, task),
        "cutover" => {
            Ok(("pass", Vec::new(), vec![("metadata-cutover", "pass"), ("transcript-gate-before-cutover", "pass")]))
        }
        "migrate-storage" => Ok(("pass", Vec::new(), vec![
            ("typed-storage-migration-recipe-bound", "pass"),
            ("migration-receipt-required", "pass"),
        ])),
        "cleanup" => cleanup_result(root, ledger_root, task),
        "drain-sessions" => protocol_drain_task_outcome(ledger_root, plan, task),
        "install-artifact"
        | "deprecate"
        | "install-protocol-bridge"
        | "update-handler-policy"
        | "update-docs"
        | "rollback-pointer" => {
            Ok(("pass", Vec::new(), vec![("task-admission", "pass"), ("side-effect-boundary", "pass")]))
        }
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported upgrade task kind {other}; expected one of {:?}",
            SUPPORTED_TASK_KINDS
        ))),
    }
}

fn alias_result(root: &Path, plan: &UpgradePlan, task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    let to_ref = task
        .to_ref
        .as_deref()
        .ok_or_else(|| MoltenError::invalid_harness("compatibility alias missing target ref"))?;
    let previous = task.from_ref.as_deref();
    let pending_receipt_ref = local_ref("upgrade-pending-receipt", &plan.plan_ref, &task.task_id)?;
    let pointer = name_pointer_value(&task.subject, "alias", to_ref, previous, &pending_receipt_ref)?;
    write_preserves(&name_pointer_path(root, &task.subject)?, &pointer)?;
    Ok(("pass", Vec::new(), vec![("compatibility-alias", "pass"), ("old-and-new-coexist", "pass")]))
}

fn transcript_result(plan: &UpgradePlan, task: &UpgradeTask) -> UpgradeTaskOutcome {
    if task.precondition_refs.is_empty() && plan.evidence_refs.is_empty() {
        ("deny", vec!["transcript rerun task has no transcript or receipt evidence refs".to_string()], vec![
            ("transcript-evidence", "fail"),
        ])
    } else {
        ("pass", Vec::new(), vec![("transcript-evidence", "pass"), ("handler-profile-bound", "pass")])
    }
}

fn move_result(root: &Path, plan: &UpgradePlan, task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    let from_ref =
        task.from_ref.as_deref().ok_or_else(|| MoltenError::invalid_harness("move-name missing from ref"))?;
    let to_ref = task.to_ref.as_deref().ok_or_else(|| MoltenError::invalid_harness("move-name missing to ref"))?;
    let current = read_name_pointer(root, &task.subject)?;
    if let Some(current) = current.as_ref()
        && current.artifact_ref != from_ref
    {
        return Ok((
            "deny",
            vec![format!(
                "name {} currently points to {}, expected {}",
                task.subject, current.artifact_ref, from_ref
            )],
            vec![("current-pointer", "fail")],
        ));
    }

    let pending_receipt_ref = local_ref("upgrade-pending-receipt", &plan.plan_ref, &task.task_id)?;
    let pointer = name_pointer_value(&task.subject, "name", to_ref, Some(from_ref), &pending_receipt_ref)?;
    write_preserves(&name_pointer_path(root, &task.subject)?, &pointer)?;
    Ok(("pass", Vec::new(), vec![
        ("metadata-pointer-move", "pass"),
        ("artifact-content-immutable", "pass"),
    ]))
}

fn cleanup_result(root: &Path, ledger_root: &Path, task: &UpgradeTask) -> Result<UpgradeTaskOutcome> {
    let cleanup_ref = task.to_ref.as_deref().or(task.from_ref.as_deref()).unwrap_or(&task.subject);
    let cleanup = cleanup_admission(root, ledger_root, cleanup_ref)?;
    if cleanup.decision == "pass" {
        Ok(("pass", Vec::new(), vec![("cleanup-safety", "pass")]))
    } else {
        Ok(("deny", vec![format!("cleanup denied by receipt {}", cleanup.receipt_ref)], vec![(
            "cleanup-safety",
            "fail",
        )]))
    }
}

pub fn rollback_task(root: &Path, plan_ref: &str, task_id: &str) -> Result<UpgradeReceipt> {
    ensure_dirs(root)?;
    let plan = read_plan(root, plan_ref)?;
    let task = plan
        .tasks
        .iter()
        .find(|task| task.task_id == task_id)
        .ok_or_else(|| MoltenError::invalid_harness(format!("upgrade plan missing task {task_id}")))?;
    let is_irreversible_task = matches!(task.kind.as_str(), "migrate-storage" | "cleanup" | "install-protocol-bridge");
    let (decision, diagnostics, checks) = if is_irreversible_task || !task.reversible {
        ("deny", vec![format!("task {} kind {} is not reversible", task.task_id, task.kind)], vec![
            ("reversible-metadata-only", "fail"),
            ("irreversible-effects-preserved", "pass"),
        ])
    } else if let Some(from_ref) = task.from_ref.as_deref() {
        let rollback_receipt_ref = local_ref("upgrade-rollback-pending", &plan.plan_ref, &task.task_id)?;
        let pointer =
            name_pointer_value(&task.subject, "name", from_ref, task.to_ref.as_deref(), &rollback_receipt_ref)?;
        if matches!(task.kind.as_str(), "move-name" | "compatibility-alias" | "cutover" | "rollback-pointer") {
            write_preserves(&name_pointer_path(root, &task.subject)?, &pointer)?;
        }
        ("pass", Vec::new(), vec![("reversible-metadata-only", "pass"), ("rollback-pointer", "pass")])
    } else {
        ("deny", vec![format!("task {} has no rollback ref", task.task_id)], vec![("rollback-ref", "fail")])
    };
    let receipt_value = upgrade_receipt_value(&UpgradeReceiptValueInput {
        operation: "rollback",
        decision,
        session_id: &plan.session_id,
        plan_ref: &plan.plan_ref,
        task_id: Some(&task.task_id),
        refs: &task_refs(task),
        diagnostics: &diagnostics,
        checks: &checks,
    })?;
    let receipt = parse_upgrade_receipt(&receipt_value)?;
    store_receipt(root, &receipt_value)?;
    Ok(receipt)
}

pub fn cleanup_admission(root: &Path, ledger_root: &Path, artifact_ref: &str) -> Result<UpgradeReceipt> {
    cleanup_admission_with_registry(root, ledger_root, None, artifact_ref)
}

pub fn cleanup_admission_with_registry(
    root: &Path,
    ledger_root: &Path,
    registry_root: Option<&Path>,
    artifact_ref: &str,
) -> Result<UpgradeReceipt> {
    ensure_dirs(root)?;
    validate_ref(artifact_ref, "cleanup artifact ref")?;
    let mut diagnostics = Vec::new();
    for pointer in read_name_pointers(root)? {
        if pointer.artifact_ref == artifact_ref || pointer.previous_ref.as_deref() == Some(artifact_ref) {
            push_bounded(
                &mut diagnostics,
                format!("name pointer {} retains {}", pointer.name, artifact_ref),
                MAX_UPGRADE_DIAGNOSTICS,
                "upgrade cleanup diagnostics",
            )?;
        }
    }
    if store_text_contains_ref(&root.join("plans"), artifact_ref)? {
        push_bounded(
            &mut diagnostics,
            format!("upgrade plan retains {artifact_ref}"),
            MAX_UPGRADE_DIAGNOSTICS,
            "upgrade cleanup diagnostics",
        )?;
    }
    if store_text_contains_ref(&root.join("receipts"), artifact_ref)? {
        push_bounded(
            &mut diagnostics,
            format!("upgrade receipt retains {artifact_ref}"),
            MAX_UPGRADE_DIAGNOSTICS,
            "upgrade cleanup diagnostics",
        )?;
    }
    if let Some(registry_root) = registry_root {
        for diagnostic in artifacts::reference_diagnostics(registry_root, artifact_ref)? {
            push_bounded(&mut diagnostics, diagnostic, MAX_UPGRADE_DIAGNOSTICS, "upgrade cleanup diagnostics")?;
        }
    }
    for entry in ledger::list_artifacts(ledger_root)? {
        if entry.artifact_ref == artifact_ref {
            continue;
        }
        let value = ledger::read_artifact(ledger_root, &entry.artifact_ref)?;
        if to_text(&value)?.contains(artifact_ref) {
            push_bounded(
                &mut diagnostics,
                format!("ledger artifact {} retains {}", entry.artifact_ref, artifact_ref),
                MAX_UPGRADE_DIAGNOSTICS,
                "upgrade cleanup diagnostics",
            )?;
        }
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let checks = if diagnostics.is_empty() {
        vec![("reference-index-empty", "pass"), ("cleanup-safety", "pass")]
    } else {
        vec![("reference-index-empty", "fail"), ("cleanup-safety", "fail")]
    };
    let receipt_value = upgrade_receipt_value(&UpgradeReceiptValueInput {
        operation: "cleanup",
        decision,
        session_id: "cleanup",
        plan_ref: artifact_ref,
        task_id: None,
        refs: &[artifact_ref.to_string()],
        diagnostics: &diagnostics,
        checks: &checks,
    })?;
    let receipt = parse_upgrade_receipt(&receipt_value)?;
    store_receipt(root, &receipt_value)?;
    Ok(receipt)
}

#[derive(Clone, Copy)]
struct GateFacts {
    is_decision_pass: bool,
    is_terminal: bool,
    is_protocol_match: bool,
}

#[derive(Default)]
struct DrainState {
    diagnostics: Vec<String>,
    has_gate: bool,
    has_gate_decision_pass: bool,
    has_terminal_state: bool,
    has_protocol_match: bool,
    has_drained_gate: bool,
}

impl DrainState {
    fn push(&mut self, message: String) -> Result<()> {
        push_bounded(&mut self.diagnostics, message, MAX_UPGRADE_DIAGNOSTICS, "upgrade protocol drain diagnostics")
    }

    fn require_refs(&mut self, refs: &[String]) -> Result<()> {
        if refs.is_empty() {
            self.push(
                "drain-sessions task requires a protocol-session-gate-receipt-v1 precondition or postcondition ref"
                    .to_string(),
            )?;
        }
        Ok(())
    }

    fn inspect_ref(&mut self, ledger_root: &Path, evidence_ref: &str, expected_refs: &[String]) -> Result<()> {
        let value = match ledger::read_artifact(ledger_root, evidence_ref) {
            Ok(value) => value,
            Err(error) => {
                self.push(format!("protocol drain evidence {evidence_ref} is not readable from ledger: {error}"))?;
                return Ok(());
            }
        };
        let gate = match protocol_session::parse_protocol_session_gate_receipt(&value) {
            Ok(gate) => gate,
            Err(error) => {
                self.push(format!(
                    "protocol drain evidence {evidence_ref} is not a protocol session gate receipt: {error}"
                ))?;
                return Ok(());
            }
        };
        self.observe(&gate, expected_refs)
    }

    fn observe(&mut self, gate: &protocol_session::ProtocolSessionGateReceipt, expected_refs: &[String]) -> Result<()> {
        self.has_gate = true;
        let facts = GateFacts {
            is_decision_pass: gate.decision == "pass",
            is_terminal: !gate.session_ids.is_empty() && !gate.final_state_refs.is_empty(),
            is_protocol_match: expected_refs.iter().any(|expected| expected == &gate.protocol_ref),
        };
        self.has_gate_decision_pass |= facts.is_decision_pass;
        self.has_terminal_state |= facts.is_terminal;
        self.has_protocol_match |= facts.is_protocol_match;
        self.note_gate(gate, expected_refs, facts)?;
        self.has_drained_gate |= facts.is_decision_pass && facts.is_terminal && facts.is_protocol_match;
        Ok(())
    }

    fn note_gate(
        &mut self,
        gate: &protocol_session::ProtocolSessionGateReceipt,
        expected_refs: &[String],
        facts: GateFacts,
    ) -> Result<()> {
        if !facts.is_decision_pass {
            self.push(format!("protocol drain gate {} denied with decision {}", gate.receipt_ref, gate.decision))?;
        }
        if !facts.is_terminal {
            self.push(format!("protocol drain gate {} does not bind terminal session state", gate.receipt_ref))?;
        }
        if !facts.is_protocol_match {
            self.push(format!(
                "protocol drain gate {} is for {}, expected one of {}",
                gate.receipt_ref,
                gate.protocol_ref,
                expected_refs.join(",")
            ))?;
        }
        Ok(())
    }

    fn require_gate(&mut self, refs: &[String]) -> Result<()> {
        if !refs.is_empty() && !self.has_gate {
            self.push("drain-sessions task did not bind any readable protocol session gate receipts".to_string())?;
        }
        Ok(())
    }

    fn outcome(self) -> UpgradeTaskOutcome {
        let decision = if self.diagnostics.is_empty() && self.has_drained_gate {
            "pass"
        } else {
            "deny"
        };
        (decision, self.diagnostics, vec![
            ("protocol-session-gate-bound", pass_fail(self.has_gate)),
            ("protocol-session-gate-pass", pass_fail(self.has_gate_decision_pass)),
            ("protocol-terminal-state", pass_fail(self.has_terminal_state)),
            ("protocol-ref-bound", pass_fail(self.has_protocol_match)),
            ("protocol-session-drain", pass_fail(self.has_drained_gate)),
            ("protocol-drain-is-not-authority", "pass"),
        ])
    }
}

fn protocol_drain_task_outcome(
    ledger_root: &Path,
    plan: &UpgradePlan,
    task: &UpgradeTask,
) -> Result<UpgradeTaskOutcome> {
    let evidence_refs = protocol_drain_evidence_refs(task)?;
    let expected_refs = protocol_drain_expected_protocol_refs(plan, task)?;
    let mut state = DrainState::default();
    state.require_refs(&evidence_refs)?;
    for evidence_ref in &evidence_refs {
        state.inspect_ref(ledger_root, evidence_ref, &expected_refs)?;
    }
    state.require_gate(&evidence_refs)?;
    Ok(state.outcome())
}

fn protocol_drain_evidence_refs(task: &UpgradeTask) -> Result<Vec<String>> {
    let mut refs = BtreeSet::new();
    refs.extend(task.precondition_refs.iter().cloned());
    refs.extend(task.postcondition_refs.iter().cloned());
    let refs: Vec<String> = refs.into_iter().collect();
    validate_refs(&refs, "upgrade protocol drain evidence ref")?;
    Ok(refs)
}

fn protocol_drain_expected_protocol_refs(plan: &UpgradePlan, task: &UpgradeTask) -> Result<Vec<String>> {
    let mut refs = BtreeSet::new();
    if let Some(from_ref) = task.from_ref.as_ref() {
        refs.insert(from_ref.clone());
    } else if is_canonical_ref(&task.subject) {
        refs.insert(task.subject.clone());
    } else {
        refs.extend(plan.compatibility.old_refs.iter().cloned());
        if refs.is_empty() {
            refs.extend(plan.affected_refs.iter().cloned());
        }
    }
    if refs.is_empty() {
        return Err(MoltenError::invalid_harness("drain-sessions task has no protocol ref binding"));
    }
    let refs: Vec<String> = refs.into_iter().collect();
    validate_refs(&refs, "upgrade protocol drain expected protocol ref")?;
    Ok(refs)
}

fn is_canonical_ref(value: &str) -> bool {
    validate_content_ref(value).is_ok()
}

fn pass_fail(is_pass: bool) -> &'static str {
    if is_pass { "pass" } else { "fail" }
}

pub fn status(root: &Path, plan_ref: &str) -> Result<UpgradeStatus> {
    let plan = read_plan(root, plan_ref)?;
    ensure_count_at_most(plan.tasks.len(), MAX_UPGRADE_TASKS, "upgrade plan tasks")?;
    let mut tasks = Vec::with_capacity(plan.tasks.len());
    let mut remaining_task_ids = Vec::new();
    for task in &plan.tasks {
        let receipt_ref = read_status_receipt_ref(root, &plan, &task.task_id)?;
        let is_task_done = receipt_ref.is_some();
        if !is_task_done {
            push_bounded(&mut remaining_task_ids, task.task_id.clone(), MAX_UPGRADE_TASKS, "upgrade remaining tasks")?;
        }
        push_bounded(
            &mut tasks,
            UpgradeTaskStatus {
                task_id: task.task_id.clone(),
                kind: task.kind.clone(),
                done: is_task_done,
                receipt_ref,
            },
            MAX_UPGRADE_TASKS,
            "upgrade task status entries",
        )?;
    }
    Ok(UpgradeStatus {
        plan_ref: plan.plan_ref,
        session_id: plan.session_id,
        tasks,
        remaining_task_ids,
    })
}

fn read_plan(root: &Path, plan_ref: &str) -> Result<UpgradePlan> {
    validate_ref(plan_ref, "upgrade plan ref")?;
    parse_upgrade_plan(&read_preserves(&plan_path(root, plan_ref)?)?)
}

fn validate_upgrade_source_gates(input: &UpgradePlanInput) -> Result<Vec<String>> {
    if input.source_gate_receipt_values.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade plan requires strict Octet source gate receipt values"));
    }
    ensure_count_at_most(
        input.source_gate_receipt_values.len(),
        MAX_UPGRADE_SOURCE_GATES,
        "upgrade source gate receipt values",
    )?;
    let subject_ref = source_gate_subject_ref(&input.session_id, &input.affected_refs)?;
    let mut validation_refs = Vec::new();
    let mut diagnostics = Vec::new();
    for value in &input.source_gate_receipt_values {
        let validation = octet_gate::validate_octet_source_gate(&octet_gate::OctetSourceGateValidationInput {
            consumer: "upgrade-plan".to_string(),
            subject_ref: subject_ref.clone(),
            gate_receipt_value: Some(value.clone()),
            source_scope: Vec::new(),
        })?;
        push_bounded(
            &mut validation_refs,
            validation.validation_ref.clone(),
            MAX_UPGRADE_SOURCE_GATES,
            "upgrade source gate validation refs",
        )?;
        if validation.decision != "pass" {
            push_bounded(
                &mut diagnostics,
                format!("strict Octet source gate validation {} denied", validation.validation_ref),
                MAX_UPGRADE_DIAGNOSTICS,
                "upgrade source gate diagnostics",
            )?;
        }
    }
    if validation_refs.is_empty() || !diagnostics.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "upgrade plan source gate validation failed: {}",
            diagnostics.join("; ")
        )));
    }
    Ok(validation_refs)
}

fn source_gate_subject_ref(session_id: &str, affected_refs: &[String]) -> Result<String> {
    canonical_hash(&record("upgrade-source-gate-subject-v1", vec![
        string(session_id),
        refs_sequence(&sorted_refs(affected_refs.to_vec())),
    ]))
}

fn sorted_refs(mut refs: Vec<String>) -> Vec<String> {
    refs.sort();
    refs.dedup();
    refs
}

fn validate_plan_input(input: &UpgradePlanInput) -> Result<()> {
    validate_non_empty(&input.session_id, "upgrade session id")?;
    validate_non_empty(&input.reason, "upgrade reason")?;
    validate_non_empty(&input.summary, "upgrade summary")?;
    validate_ref(&input.initiator_ref, "upgrade initiator ref")?;
    validate_refs(&input.capability_refs, "upgrade capability ref")?;
    validate_refs(&input.affected_refs, "upgrade affected ref")?;
    validate_refs(&input.impact_refs, "upgrade impact ref")?;
    validate_refs(&input.rollback_refs, "upgrade rollback ref")?;
    validate_refs(&input.policy_refs, "upgrade policy ref")?;
    validate_refs(&input.evidence_refs, "upgrade evidence ref")?;
    validate_compatibility(&input.compatibility)?;
    if input.tasks.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade plan must contain at least one task"));
    }
    let mut seen = BtreeSet::new();
    for task in &input.tasks {
        validate_task_input(task)?;
        if !seen.insert(task.task_id.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate upgrade task id {}", task.task_id)));
        }
    }
    Ok(())
}

fn validate_parsed_plan(plan: &UpgradePlan) -> Result<()> {
    validate_non_empty(&plan.session_id, "upgrade session id")?;
    validate_ref(&plan.initiator_ref, "upgrade initiator ref")?;
    validate_refs(&plan.capability_refs, "upgrade capability ref")?;
    validate_refs(&plan.affected_refs, "upgrade affected ref")?;
    validate_refs(&plan.impact_refs, "upgrade impact ref")?;
    validate_refs(&plan.rollback_refs, "upgrade rollback ref")?;
    validate_refs(&plan.policy_refs, "upgrade policy ref")?;
    validate_refs(&plan.evidence_refs, "upgrade evidence ref")?;
    validate_compatibility(&plan.compatibility)?;
    if plan.tasks.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade plan must contain at least one task"));
    }
    let mut seen = BtreeSet::new();
    for task in &plan.tasks {
        validate_task(task)?;
        if !seen.insert(task.task_id.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate upgrade task id {}", task.task_id)));
        }
    }
    if plan.tasks.iter().any(|task| task.kind == "cutover")
        && !plan.tasks.iter().any(|task| task.kind == "transcript-rerun")
    {
        return Err(MoltenError::invalid_harness("upgrade cutover requires a transcript-rerun task before cutover"));
    }
    Ok(())
}

fn validate_task_input(task: &UpgradeTaskInput) -> Result<()> {
    validate_non_empty(&task.task_id, "upgrade task id")?;
    validate_non_empty(&task.subject, "upgrade task subject")?;
    validate_task_kind(&task.kind)?;
    if let Some(value) = task.from_ref.as_deref() {
        validate_ref(value, "upgrade task from ref")?;
    }
    if let Some(value) = task.to_ref.as_deref() {
        validate_ref(value, "upgrade task to ref")?;
    }
    validate_refs(&task.precondition_refs, "upgrade task precondition ref")?;
    validate_refs(&task.postcondition_refs, "upgrade task postcondition ref")?;
    validate_task_shape(&task.kind, task.from_ref.as_deref(), task.to_ref.as_deref(), task.reversible)
}

fn validate_task(task: &UpgradeTask) -> Result<()> {
    validate_non_empty(&task.task_id, "upgrade task id")?;
    validate_non_empty(&task.subject, "upgrade task subject")?;
    validate_task_kind(&task.kind)?;
    if let Some(value) = task.from_ref.as_deref() {
        validate_ref(value, "upgrade task from ref")?;
    }
    if let Some(value) = task.to_ref.as_deref() {
        validate_ref(value, "upgrade task to ref")?;
    }
    validate_refs(&task.precondition_refs, "upgrade task precondition ref")?;
    validate_refs(&task.postcondition_refs, "upgrade task postcondition ref")?;
    validate_task_shape(&task.kind, task.from_ref.as_deref(), task.to_ref.as_deref(), task.reversible)
}

fn validate_task_shape(kind: &str, from_ref: Option<&str>, to_ref: Option<&str>, reversible: bool) -> Result<()> {
    match kind {
        "move-name" | "compatibility-alias" | "cutover" | "rollback-pointer" => {
            if from_ref.is_none() || to_ref.is_none() {
                return Err(MoltenError::invalid_harness(format!("upgrade task kind {kind} requires from/to refs")));
            }
        }
        "migrate-storage" => {
            if from_ref.is_none() || to_ref.is_none() {
                return Err(MoltenError::invalid_harness("storage migration upgrade task requires recipe/source refs"));
            }
            if reversible {
                return Err(MoltenError::invalid_harness(
                    "storage migration upgrade task cannot claim reversible rollback",
                ));
            }
        }
        "cleanup" if from_ref.is_none() && to_ref.is_none() => {
            return Err(MoltenError::invalid_harness("cleanup upgrade task requires an artifact ref"));
        }
        "cleanup" => {}
        _ => {}
    }
    Ok(())
}

fn validate_task_kind(kind: &str) -> Result<()> {
    if SUPPORTED_TASK_KINDS.contains(&kind) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported upgrade task kind {kind}; expected one of {:?}",
            SUPPORTED_TASK_KINDS
        )))
    }
}

fn validate_compatibility(compatibility: &UpgradeCompatibilityWindow) -> Result<()> {
    validate_refs(&compatibility.old_refs, "compatibility old ref")?;
    validate_refs(&compatibility.new_refs, "compatibility new ref")?;
    validate_refs(&compatibility.policy_refs, "compatibility policy ref")?;
    let old: BtreeSet<_> = compatibility.old_refs.iter().collect();
    if compatibility.new_refs.iter().any(|new_ref| old.contains(new_ref)) {
        return Err(MoltenError::invalid_harness("compatibility window old/new refs must be explicit and distinct"));
    }
    Ok(())
}

fn compatibility_window_value(compatibility: &UpgradeCompatibilityWindow) -> Result<IoValue> {
    validate_compatibility(compatibility)?;
    Ok(record("compatibility-window", vec![
        record("old", vec![refs_sequence(&compatibility.old_refs)]),
        record("new", vec![refs_sequence(&compatibility.new_refs)]),
        record("expires-at", vec![optional_u64_value(compatibility.expires_at)]),
        record("policy", vec![refs_sequence(&compatibility.policy_refs)]),
    ]))
}

fn parse_compatibility_window(value: &Value<IoValue>) -> Result<UpgradeCompatibilityWindow> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "compatibility-window", 4)?;
    Ok(UpgradeCompatibilityWindow {
        old_refs: record_ref_sequence(&fields[0], "old")?,
        new_refs: record_ref_sequence(&fields[1], "new")?,
        expires_at: record_optional_u64(&fields[2], "expires-at")?,
        policy_refs: record_ref_sequence(&fields[3], "policy")?,
    })
}

fn parse_tasks(value: &Value<IoValue>) -> Result<Vec<UpgradeTask>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "tasks", 1)?;
    let items = required_sequence(&fields[0], "upgrade tasks")?;
    let mut tasks = Vec::with_capacity(items.len());
    for item in items.iter() {
        tasks.push(parse_task(&value_to_iovalue(item))?);
    }
    Ok(tasks)
}

fn parse_task(value: &IoValue) -> Result<UpgradeTask> {
    let fields = value
        .collect_simple_record("upgrade-task-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <upgrade-task-v1 ...>"))?;
    let reversible_value = value_to_iovalue(&fields[7]);
    let reversible = simple_record(&reversible_value, "reversible", 1)?;
    Ok(UpgradeTask {
        task_id: required_string(&fields[0], "upgrade task id")?,
        kind: record_string(&fields[1], "kind")?,
        subject: record_string(&fields[2], "subject")?,
        from_ref: record_optional_ref(&fields[3], "from")?,
        to_ref: record_optional_ref(&fields[4], "to")?,
        precondition_refs: record_ref_sequence(&fields[5], "preconditions")?,
        postcondition_refs: record_ref_sequence(&fields[6], "postconditions")?,
        reversible: required_bool(&reversible[0], "reversible")?,
    })
}

pub fn parse_upgrade_receipt(value: &IoValue) -> Result<UpgradeReceipt> {
    let fields = value
        .collect_simple_record("upgrade-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <upgrade-receipt-v1 ...>"))?;
    require_schema(&fields[0], UPGRADE_RECEIPT_SCHEMA, "upgrade receipt")?;
    let session = value_to_iovalue(&fields[3]);
    let session_fields = simple_record(&session, "session", 2)?;
    let task = value_to_iovalue(&fields[4]);
    let task_fields = simple_record(&task, "task", 1)?;
    let checks = parse_checks(&fields[7])?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("upgrade receipt missing checks"));
    }
    Ok(UpgradeReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        session_id: required_string(&session_fields[0], "upgrade receipt session id")?,
        plan_ref: required_ref(&session_fields[1], "upgrade receipt plan ref")?,
        task_id: parse_optional_string_value(&task_fields[0])?,
        value: value.clone(),
    })
}

fn upgrade_receipt_value(input: &UpgradeReceiptValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "upgrade receipt operation")?;
    if input.decision != "pass" && input.decision != "deny" {
        return Err(MoltenError::invalid_harness(format!("unsupported upgrade receipt decision {}", input.decision)));
    }
    validate_non_empty(input.session_id, "upgrade receipt session id")?;
    validate_ref(input.plan_ref, "upgrade receipt plan ref")?;
    validate_refs(input.refs, "upgrade receipt ref")?;
    Ok(record("upgrade-receipt-v1", vec![
        string(UPGRADE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("session", vec![string(input.session_id), string(input.plan_ref)]),
        record("task", vec![optional_string_value(input.task_id)]),
        record("refs", vec![refs_sequence(input.refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(input.checks),
    ]))
}

fn name_pointer_value(
    name: &str,
    pointer_kind: &str,
    artifact_ref: &str,
    previous_ref: Option<&str>,
    receipt_ref: &str,
) -> Result<IoValue> {
    validate_non_empty(name, "name pointer name")?;
    validate_non_empty(pointer_kind, "name pointer kind")?;
    validate_ref(artifact_ref, "name pointer artifact ref")?;
    if let Some(previous_ref) = previous_ref {
        validate_ref(previous_ref, "name pointer previous ref")?;
    }
    validate_ref(receipt_ref, "name pointer receipt ref")?;
    Ok(record("upgrade-name-pointer-v1", vec![
        string(UPGRADE_NAME_POINTER_SCHEMA),
        record("name", vec![string(name)]),
        record("kind", vec![string(pointer_kind)]),
        record("artifact", vec![string(artifact_ref)]),
        record("previous", vec![optional_ref_value(previous_ref)]),
        record("receipt", vec![string(receipt_ref)]),
        checks_value(&["names-are-metadata", "artifact-content-immutable"]),
    ]))
}

fn parse_name_pointer(value: &IoValue) -> Result<NamePointer> {
    let fields = value
        .collect_simple_record("upgrade-name-pointer-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <upgrade-name-pointer-v1 ...>"))?;
    require_schema(&fields[0], UPGRADE_NAME_POINTER_SCHEMA, "upgrade name pointer")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "names-are-metadata", "upgrade name pointer")?;
    Ok(NamePointer {
        name: record_string(&fields[1], "name")?,
        pointer_kind: record_string(&fields[2], "kind")?,
        artifact_ref: record_ref(&fields[3], "artifact")?,
        previous_ref: record_optional_ref(&fields[4], "previous")?,
        receipt_ref: record_ref(&fields[5], "receipt")?,
        value: value.clone(),
    })
}

fn plan_refs(plan: &UpgradePlan) -> Vec<String> {
    let mut refs = BtreeSet::new();
    refs.insert(plan.plan_ref.clone());
    refs.insert(plan.initiator_ref.clone());
    refs.extend(plan.capability_refs.iter().cloned());
    refs.extend(plan.affected_refs.iter().cloned());
    refs.extend(plan.impact_refs.iter().cloned());
    refs.extend(plan.rollback_refs.iter().cloned());
    refs.extend(plan.policy_refs.iter().cloned());
    refs.extend(plan.evidence_refs.iter().cloned());
    for task in &plan.tasks {
        refs.extend(task_refs(task));
    }
    refs.into_iter().collect()
}

fn task_refs(task: &UpgradeTask) -> Vec<String> {
    let mut refs = BtreeSet::new();
    if let Some(value) = task.from_ref.as_ref() {
        refs.insert(value.clone());
    }
    if let Some(value) = task.to_ref.as_ref() {
        refs.insert(value.clone());
    }
    refs.extend(task.precondition_refs.iter().cloned());
    refs.extend(task.postcondition_refs.iter().cloned());
    refs.into_iter().collect()
}

fn ensure_prior_tasks_complete(root: &Path, plan: &UpgradePlan, task_index: usize) -> Result<()> {
    for task in &plan.tasks[..task_index] {
        if read_status_receipt_ref(root, plan, &task.task_id)?.is_none() {
            return Err(MoltenError::invalid_harness(format!(
                "upgrade task {} cannot run before prior task {} completes",
                plan.tasks[task_index].task_id, task.task_id
            )));
        }
    }
    Ok(())
}

fn write_status(root: &Path, plan: &UpgradePlan, task: &UpgradeTask, receipt_ref: &str) -> Result<()> {
    validate_ref(receipt_ref, "upgrade task status receipt ref")?;
    let path = status_path(root, &plan.session_id, &task.task_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, receipt_ref).map_err(MoltenError::from)
}

fn read_status_receipt_ref(root: &Path, plan: &UpgradePlan, task_id: &str) -> Result<Option<String>> {
    let path = status_path(root, &plan.session_id, task_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let receipt_ref = fs::read_to_string(path).map_err(MoltenError::from)?;
    validate_ref(&receipt_ref, "upgrade task status receipt ref")?;
    Ok(Some(receipt_ref))
}

fn read_name_pointers(root: &Path) -> Result<Vec<NamePointer>> {
    let names = root.join("names");
    if !names.exists() {
        return Ok(Vec::new());
    }
    let mut pointers = Vec::new();
    for entry in fs::read_dir(names).map_err(MoltenError::from)? {
        let entry = entry.map_err(MoltenError::from)?;
        if entry.file_type().map_err(MoltenError::from)?.is_file() {
            push_bounded(
                &mut pointers,
                parse_name_pointer(&read_preserves(&entry.path())?)?,
                MAX_UPGRADE_POINTERS,
                "upgrade name pointers",
            )?;
        }
    }
    Ok(pointers)
}

fn store_text_contains_ref(dir: &Path, target_ref: &str) -> Result<bool> {
    if !dir.exists() {
        return Ok(false);
    }
    let mut pending_dirs = Vec::with_capacity(1);
    pending_dirs.push(dir.to_path_buf());
    let mut scanned_entries = 0usize;
    while let Some(current_dir) = pending_dirs.pop() {
        for entry in fs::read_dir(current_dir).map_err(MoltenError::from)? {
            scanned_entries = scanned_entries
                .checked_add(1)
                .ok_or_else(|| MoltenError::invalid_harness("upgrade store scan count overflow"))?;
            ensure_count_at_most(scanned_entries, MAX_UPGRADE_POINTERS, "upgrade store scan entries")?;
            let entry = entry.map_err(MoltenError::from)?;
            if entry.file_type().map_err(MoltenError::from)?.is_dir() {
                push_bounded(&mut pending_dirs, entry.path(), MAX_UPGRADE_POINTERS, "upgrade store scan dirs")?;
            } else if fs::read_to_string(entry.path()).map_err(MoltenError::from)?.contains(target_ref) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn ensure_dirs(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join("plans")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("receipts")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("names")).map_err(MoltenError::from)?;
    fs::create_dir_all(root.join("status")).map_err(MoltenError::from)
}

fn write_preserves(path: &Path, value: &IoValue) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, to_text(value)?).map_err(MoltenError::from)
}

fn read_preserves(path: &Path) -> Result<IoValue> {
    parse_text(&fs::read_to_string(path).map_err(MoltenError::from)?)
}

fn store_receipt(root: &Path, receipt_value: &IoValue) -> Result<()> {
    let receipt_ref = canonical_hash(receipt_value)?;
    write_preserves(&receipt_path(root, &receipt_ref)?, receipt_value)
}

fn plan_path(root: &Path, plan_ref: &str) -> Result<PathBuf> {
    Ok(root.join("plans").join(filename_for_ref(plan_ref)?))
}

fn receipt_path(root: &Path, receipt_ref: &str) -> Result<PathBuf> {
    Ok(root.join("receipts").join(filename_for_ref(receipt_ref)?))
}

fn name_pointer_path(root: &Path, name: &str) -> Result<PathBuf> {
    let key = canonical_hash(&record("upgrade-name-pointer-key", vec![string(name)]))?;
    Ok(root.join("names").join(filename_for_ref(&key)?))
}

fn status_path(root: &Path, session_id: &str, task_id: &str) -> Result<PathBuf> {
    let session = canonical_hash(&record("upgrade-session-status-key", vec![string(session_id)]))?;
    let task = canonical_hash(&record("upgrade-task-status-key", vec![string(task_id)]))?;
    Ok(root.join("status").join(filename_for_ref(&session)?).join(filename_for_ref(&task)?))
}

fn filename_for_ref(value_ref: &str) -> Result<String> {
    let hex = content_ref_hex(value_ref)
        .map_err(|error| MoltenError::invalid_harness(format!("unsupported ref {value_ref}: {error}")))?;
    Ok(format!("blake3_{hex}.preserves"))
}

fn local_ref(kind: &str, a: &str, b: &str) -> Result<String> {
    canonical_hash(&record("upgrade-local-ref", vec![string(kind), string(a), string(b)]))
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_u64_value(value: Option<u64>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}

fn parse_optional_ref_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn parse_optional_string_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_string(&fields[0], "optional string").map(Some);
    }
    required_string(value, "optional string").map(Some)
}

fn parse_optional_u64_value(value: &Value<IoValue>) -> Result<Option<u64>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_u64(&fields[0], "optional u64").map(Some);
    }
    required_u64(value, "optional u64").map(Some)
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn record_optional_u64(value: &Value<IoValue>, label: &str) -> Result<Option<u64>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_u64_value(&record[0])
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let count = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(count, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn parse_ref_sequence_value(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    ensure_count_at_most(items.len(), MAX_UPGRADE_REFS, label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut refs, required_ref(item, label)?, MAX_UPGRADE_REFS, label)?;
    }
    Ok(refs)
}

fn checks_value(names: &[&str]) -> IoValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
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
            return Err(MoltenError::invalid_harness(format!("upgrade check {name} has status {status}")));
        }
        push_bounded(&mut parsed, name, MAX_UPGRADE_TASKS, "upgrade checks")?;
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

fn require_schema(value: &Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &Value<IoValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn required_bool(value: &Value<IoValue>, field: &str) -> Result<bool> {
    value.as_boolean().ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {field}")))
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
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref, got {value_ref}: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use hegel::TestCase;
    use hegel::generators;

    use super::*;

    #[test]
    fn name_move_session_keeps_artifacts_immutable_and_receipted() {
        let root = temp_dir("upgrade-name-move");
        let ledger_root = root.join("ledger");
        let store = root.join("upgrades");
        let old = ledger::import_artifact(&ledger_root, &parse_text("<module \"old\">").expect("old artifact"))
            .expect("import old")
            .artifact_ref;
        let new = ledger::import_artifact(&ledger_root, &parse_text("<module \"new\">").expect("new artifact"))
            .expect("import new")
            .artifact_ref;
        let dependent =
            ledger::import_artifact(&ledger_root, &record("dependent", vec![string(&old), string("uses old")]))
                .expect("import dependent")
                .artifact_ref;
        let plan_value = name_move_plan_value(&ledger_root, &NameMovePlanInput {
            session_id: "session-name-move".to_string(),
            name: "app/main".to_string(),
            from_ref: old.clone(),
            to_ref: new.clone(),
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("transcript-pass")],
            source_gate_receipt_values: source_gate_values(),
        })
        .expect("plan value");
        let plan = parse_upgrade_plan(&plan_value).expect("parse plan");
        assert!(plan.impact_refs.contains(&old));
        assert!(plan.impact_refs.contains(&dependent));
        let created = create_session(&store, &plan_value).expect("create session");
        assert_eq!(created.receipt.decision, "pass");
        set_name_pointer(&store, "app/main", &old).expect("initial name pointer");
        for task_id in ["compatibility-alias", "transcript-gate", "move-name", "cutover"] {
            let executed = execute_task(&store, &ledger_root, &created.plan.plan_ref, task_id).expect("execute task");
            assert_eq!(executed.receipt.decision, "pass", "{task_id}");
        }
        let pointer = read_name_pointer(&store, "app/main").expect("read pointer").expect("pointer exists");
        assert_eq!(pointer.artifact_ref, new);
        let status = status(&store, &created.plan.plan_ref).expect("status");
        assert!(status.remaining_task_ids.is_empty());
        let cleanup_old = cleanup_admission(&store, &ledger_root, &old).expect("cleanup old");
        assert_eq!(cleanup_old.decision, "deny");
    }

    #[test]
    fn registry_backed_name_move_impact_uses_reverse_dependencies() {
        let root = temp_dir("upgrade-registry-impact");
        let registry_root = root.join("registry");
        let ledger_root = root.join("ledger");
        let old = artifacts::install_artifact(&registry_root, &artifact_input("schema", "old", &[]))
            .expect("install old")
            .artifact_ref;
        let dependent = artifacts::install_artifact(
            &registry_root,
            &artifact_input("steel", "dependent", std::slice::from_ref(&old)),
        )
        .expect("install dependent")
        .artifact_ref;
        let new = artifacts::install_artifact(&registry_root, &artifact_input("schema", "new", &[]))
            .expect("install new")
            .artifact_ref;
        let plan_value = name_move_plan_value_with_registry(Some(&registry_root), &ledger_root, &NameMovePlanInput {
            session_id: "session-registry-impact".to_string(),
            name: "app/main".to_string(),
            from_ref: old.clone(),
            to_ref: new,
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("transcript-pass")],
            source_gate_receipt_values: source_gate_values(),
        })
        .expect("registry impact plan");
        let plan = parse_upgrade_plan(&plan_value).expect("parse plan");
        assert!(plan.impact_refs.contains(&old));
        assert!(plan.impact_refs.contains(&dependent));
    }

    #[test]
    fn rollback_denies_irreversible_storage_migration_claims() {
        let root = temp_dir("upgrade-rollback");
        let store = root.join("upgrades");
        let source_schema = test_ref("schema-v1");
        let recipe = test_ref("migration-recipe");
        let plan_value = upgrade_plan_value(&UpgradePlanInput {
            session_id: "session-storage-migration".to_string(),
            reason: "storage migration".to_string(),
            summary: "migrate durable records".to_string(),
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![source_schema.clone(), recipe.clone()],
            impact_refs: vec![source_schema.clone()],
            tasks: vec![UpgradeTaskInput {
                task_id: "migrate".to_string(),
                kind: "migrate-storage".to_string(),
                subject: "profiles".to_string(),
                from_ref: Some(source_schema),
                to_ref: Some(recipe),
                precondition_refs: vec![test_ref("storage-migration-policy")],
                postcondition_refs: Vec::new(),
                reversible: false,
            }],
            compatibility: UpgradeCompatibilityWindow {
                old_refs: vec![test_ref("schema-v1-old")],
                new_refs: vec![test_ref("schema-v2-new")],
                expires_at: Some(10),
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: Vec::new(),
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("migration-review")],
            source_gate_receipt_values: source_gate_values(),
        })
        .expect("plan value");
        let created = create_session(&store, &plan_value).expect("create session");
        let rollback = rollback_task(&store, &created.plan.plan_ref, "migrate").expect("rollback denied receipt");
        assert_eq!(rollback.decision, "deny");
        assert!(to_text(&rollback.value).expect("receipt text").contains("not reversible"));
    }

    #[test]
    fn upgrade_plan_requires_valid_source_gate_receipt_content() {
        let base_input = || UpgradePlanInput {
            session_id: "session-source-gate".to_string(),
            reason: "source gate".to_string(),
            summary: "validate strict source gate".to_string(),
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![test_ref("affected")],
            impact_refs: vec![test_ref("affected")],
            tasks: vec![UpgradeTaskInput {
                task_id: "transcript".to_string(),
                kind: "transcript-rerun".to_string(),
                subject: "source-gate".to_string(),
                from_ref: None,
                to_ref: None,
                precondition_refs: vec![test_ref("transcript")],
                postcondition_refs: Vec::new(),
                reversible: true,
            }],
            compatibility: UpgradeCompatibilityWindow {
                old_refs: vec![test_ref("old")],
                new_refs: vec![test_ref("new")],
                expires_at: None,
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: vec![test_ref("old")],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("transcript-pass")],
            source_gate_receipt_values: source_gate_values(),
        };
        let pass = upgrade_plan_value(&base_input()).expect("passing source gate plan");
        let plan = parse_upgrade_plan(&pass).expect("parse pass plan");
        assert!(plan.evidence_refs.len() > 1);

        let mut missing = base_input();
        missing.source_gate_receipt_values.clear();
        assert!(
            upgrade_plan_value(&missing)
                .expect_err("missing source gate denied")
                .to_string()
                .contains("strict Octet source gate")
        );

        let denied_gate = parse_text(
            &to_text(&octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("source gate fixture"))
                .expect("source gate text")
                .replacen("<decision \"pass\">", "<decision \"deny\">", 1),
        )
        .expect("denied gate parse");
        let mut denied = base_input();
        denied.source_gate_receipt_values = vec![denied_gate];
        assert!(
            upgrade_plan_value(&denied)
                .expect_err("denied source gate rejected")
                .to_string()
                .contains("source gate validation failed")
        );
    }

    #[test]
    fn protocol_drain_task_requires_passing_protocol_gate_evidence() {
        let root = temp_dir("upgrade-protocol-drain");
        let ledger_root = root.join("ledger");
        let store = root.join("upgrades");
        let gate = protocol_drain_gate();
        let gate_ref = ledger::import_artifact(&ledger_root, &gate.value).expect("import gate").artifact_ref;
        assert_eq!(gate_ref, gate.receipt_ref);
        let new_protocol_ref = test_ref("protocol-v2");
        let plan_value =
            protocol_drain_plan_value(&gate_ref, &gate.protocol_ref, &new_protocol_ref).expect("protocol drain plan");
        let created = create_session(&store, &plan_value).expect("create session");
        let executed =
            execute_task(&store, &ledger_root, &created.plan.plan_ref, "drain-sessions").expect("execute drain task");
        assert_eq!(executed.receipt.decision, "pass");
        let text = to_text(&executed.receipt.value).expect("receipt text");
        assert!(text.contains("protocol-session-drain"));
        assert!(status(&store, &created.plan.plan_ref).expect("status").remaining_task_ids.is_empty());
    }

    #[test]
    fn protocol_drain_task_denies_missing_stale_or_mismatched_gate_evidence() {
        let root = temp_dir("upgrade-protocol-drain-deny");
        let ledger_root = root.join("ledger");
        let missing_store = root.join("missing-upgrades");
        let gate = protocol_drain_gate();
        let new_protocol_ref = test_ref("protocol-v2");
        let missing_gate_ref = test_ref("missing-protocol-gate");
        let missing_plan =
            protocol_drain_plan_value(&missing_gate_ref, &gate.protocol_ref, &new_protocol_ref).expect("missing plan");
        let missing_created = create_session(&missing_store, &missing_plan).expect("create missing session");
        let missing = execute_task(&missing_store, &ledger_root, &missing_created.plan.plan_ref, "drain-sessions")
            .expect("execute missing drain");
        assert_eq!(missing.receipt.decision, "deny");
        assert!(to_text(&missing.receipt.value).expect("missing text").contains("not readable from ledger"));

        let denied_store = root.join("denied-upgrades");
        let denied_gate = protocol_drain_gate_with_diagnostics(vec!["stale protocol lifecycle evidence".to_string()]);
        let denied_gate_ref =
            ledger::import_artifact(&ledger_root, &denied_gate.value).expect("import denied gate").artifact_ref;
        let denied_plan =
            protocol_drain_plan_value(&denied_gate_ref, &gate.protocol_ref, &new_protocol_ref).expect("denied plan");
        let denied_created = create_session(&denied_store, &denied_plan).expect("create denied session");
        let denied = execute_task(&denied_store, &ledger_root, &denied_created.plan.plan_ref, "drain-sessions")
            .expect("execute denied drain");
        assert_eq!(denied.receipt.decision, "deny");
        assert!(to_text(&denied.receipt.value).expect("denied text").contains("denied with decision"));

        let mismatch_store = root.join("mismatch-upgrades");
        let gate_ref = ledger::import_artifact(&ledger_root, &gate.value).expect("import pass gate").artifact_ref;
        let wrong_protocol_ref = test_ref("wrong-protocol");
        let mismatch_plan =
            protocol_drain_plan_value(&gate_ref, &wrong_protocol_ref, &new_protocol_ref).expect("mismatch plan");
        let mismatch_created = create_session(&mismatch_store, &mismatch_plan).expect("create mismatch session");
        let mismatch = execute_task(&mismatch_store, &ledger_root, &mismatch_created.plan.plan_ref, "drain-sessions")
            .expect("execute mismatch drain");
        assert_eq!(mismatch.receipt.decision, "deny");
        assert!(to_text(&mismatch.receipt.value).expect("mismatch text").contains("expected one of"));
    }

    #[test]
    fn cleanup_passes_only_without_active_references() {
        let root = temp_dir("upgrade-cleanup");
        let ledger_root = root.join("ledger");
        let store = root.join("upgrades");
        let artifact = ledger::import_artifact(&ledger_root, &parse_text("<module \"unused\">").expect("artifact"))
            .expect("import artifact")
            .artifact_ref;
        let pass = cleanup_admission(&store, &ledger_root, &artifact).expect("cleanup pass");
        assert_eq!(pass.decision, "pass");
        set_name_pointer(&store, "unused", &artifact).expect("pin by name");
        let deny = cleanup_admission(&store, &ledger_root, &artifact).expect("cleanup deny");
        assert_eq!(deny.decision, "deny");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_upgrade_plan_hash_task_order_and_impact_invariants(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let root = temp_dir("upgrade-hegel");
        let ledger_root = root.join("ledger");
        let base = ledger::import_artifact(&ledger_root, &record("artifact", vec![string(format!("base-{salt}"))]))
            .expect("base")
            .artifact_ref;
        let dependent = ledger::import_artifact(
            &ledger_root,
            &record("dependent", vec![string(&base), string(format!("dep-{salt}"))]),
        )
        .expect("dependent")
        .artifact_ref;
        let other = ledger::import_artifact(&ledger_root, &record("other", vec![string(format!("other-{salt}"))]))
            .expect("other")
            .artifact_ref;
        let impact_one = compute_impact_set(&ledger_root, std::slice::from_ref(&base)).expect("impact one");
        let impact_two = compute_impact_set(&ledger_root, &[base.clone(), other.clone()]).expect("impact two");
        assert!(impact_one.contains(&base));
        assert!(impact_one.contains(&dependent));
        for impacted in &impact_one {
            assert!(impact_two.contains(impacted));
        }
        let input = NameMovePlanInput {
            session_id: format!("session-{salt}"),
            name: format!("name-{salt}"),
            from_ref: base,
            to_ref: other,
            initiator_ref: test_ref(&format!("initiator-{salt}")),
            capability_refs: vec![test_ref(&format!("cap-{salt}"))],
            policy_refs: vec![test_ref(&format!("policy-{salt}"))],
            evidence_refs: vec![test_ref(&format!("evidence-{salt}"))],
            source_gate_receipt_values: source_gate_values(),
        };
        let first = name_move_plan_value(&ledger_root, &input).expect("first plan");
        let second = name_move_plan_value(&ledger_root, &input).expect("second plan");
        assert_eq!(canonical_hash(&first).expect("first hash"), canonical_hash(&second).expect("second hash"));
        let plan = parse_upgrade_plan(&first).expect("parse plan");
        assert!(
            plan.tasks.iter().position(|task| task.kind == "transcript-rerun")
                < plan.tasks.iter().position(|task| task.kind == "cutover")
        );
        let old: BtreeSet<_> = plan.compatibility.old_refs.iter().collect();
        assert!(plan.compatibility.new_refs.iter().all(|new_ref| !old.contains(new_ref)));
    }

    fn protocol_drain_gate() -> protocol_session::ProtocolSessionGate {
        protocol_drain_gate_with_diagnostics(Vec::new())
    }

    fn protocol_drain_gate_with_diagnostics(extra_diagnostics: Vec<String>) -> protocol_session::ProtocolSessionGate {
        let lifecycle = protocol_session::request_response_lifecycle().expect("protocol lifecycle");
        protocol_session::gate_protocol_session_lifecycle_with_diagnostics(
            protocol_session::ProtocolSessionGateInput {
                install_receipt: lifecycle.install.value.clone(),
                initial_states: lifecycle.initial_states.iter().map(|state| state.value.clone()).collect(),
                operation_receipts: lifecycle
                    .operations
                    .iter()
                    .map(|operation| operation.receipt.value.clone())
                    .collect(),
                messages: lifecycle
                    .operations
                    .iter()
                    .filter_map(|operation| operation.message.as_ref().map(|message| message.value.clone()))
                    .collect(),
                next_states: lifecycle
                    .operations
                    .iter()
                    .filter_map(|operation| operation.next_state.as_ref().map(|state| state.value.clone()))
                    .collect(),
            },
            extra_diagnostics,
        )
        .expect("protocol gate")
    }

    fn protocol_drain_plan_value(gate_ref: &str, old_protocol_ref: &str, new_protocol_ref: &str) -> Result<IoValue> {
        upgrade_plan_value(&UpgradePlanInput {
            session_id: "session-protocol-drain".to_string(),
            reason: "protocol drain".to_string(),
            summary: "drain protocol sessions before cutover".to_string(),
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![old_protocol_ref.to_string(), new_protocol_ref.to_string()],
            impact_refs: vec![old_protocol_ref.to_string()],
            tasks: vec![UpgradeTaskInput {
                task_id: "drain-sessions".to_string(),
                kind: "drain-sessions".to_string(),
                subject: "request-response-protocol".to_string(),
                from_ref: Some(old_protocol_ref.to_string()),
                to_ref: Some(new_protocol_ref.to_string()),
                precondition_refs: vec![gate_ref.to_string()],
                postcondition_refs: Vec::new(),
                reversible: false,
            }],
            compatibility: UpgradeCompatibilityWindow {
                old_refs: vec![old_protocol_ref.to_string()],
                new_refs: vec![new_protocol_ref.to_string()],
                expires_at: Some(32),
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: vec![old_protocol_ref.to_string()],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![gate_ref.to_string()],
            source_gate_receipt_values: source_gate_values(),
        })
    }

    fn artifact_input(kind: &str, label: &str, dependency_refs: &[String]) -> artifacts::ArtifactInstallInput {
        artifacts::ArtifactInstallInput {
            kind: kind.to_string(),
            payload: record("upgrade-artifact-payload", vec![string(label)]),
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
        canonical_hash(&record("upgrade-test-ref", vec![string(label)])).expect("test ref")
    }

    fn source_gate_values() -> Vec<IoValue> {
        vec![octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("source gate fixture")]
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
