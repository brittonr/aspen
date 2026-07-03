type BtreeSet<T> = std::collections::BTreeSet<T>;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type Record<T> = preserves::Record<T>;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

mod fs {
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub(super) fn read_dir(path: impl AsRef<std::path::Path>) -> std::io::Result<std::fs::ReadDir> {
        std::fs::read_dir(path)
    }

    pub(super) fn read_to_string(path: impl AsRef<std::path::Path>) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    #[cfg(test)]
    pub(super) fn remove_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    pub(super) fn write(path: impl AsRef<std::path::Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }
}

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
const UPGRADE_STATE_SNAPSHOT_DIRS: &[&str] = &["plans", "names", "status"];

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
struct ProtocolDrainGateEvidence {
    gate_ref: String,
    decision: String,
    protocol_ref: String,
    session_ids: Vec<String>,
    terminal_state_refs: Vec<String>,
}

struct UpgradeDrainReadinessInput<'a> {
    task_id: &'a str,
    subject: &'a str,
    from_ref: Option<&'a str>,
    to_ref: Option<&'a str>,
    affected_refs: &'a [String],
    compatibility_old_refs: &'a [String],
    compatibility_new_refs: &'a [String],
    evidence_refs: &'a [String],
    gate_evidence: &'a [ProtocolDrainGateEvidence],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpgradeDrainReadinessDecision {
    decision: &'static str,
    diagnostics: Vec<String>,
    checks: Vec<UpgradeCheckPair>,
    terminal_state_refs: Vec<String>,
}

struct UpgradeMutationBoundaryInput<'a> {
    operation: &'a str,
    decision: &'a str,
    before_state_ref: &'a str,
    after_state_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpgradeMutationBoundaryDecision {
    diagnostics: Vec<String>,
    checks: Vec<UpgradeCheckPair>,
}

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
