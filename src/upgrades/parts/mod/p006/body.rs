
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
            "structured-session-surfaces",
            "external-workflows-not-replaced",
            "no-ucm-clone",
        ]),
    ]))
}
