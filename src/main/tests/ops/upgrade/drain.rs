#[test]
fn cli_upgrade_protocol_drain_task_gates_on_ledger_protocol_evidence() {
    let dir = temp_dir("upgrade-cli-protocol-drain");
    let fixture = protocol_drain_fixture(&dir);
    protocol_drain_with_ledger_gate_passes(&fixture);
    protocol_drain_without_ledger_gate_denies(fixture);
}

struct ProtocolDrainFixture {
    dir: PathBuf,
    ledger_root: PathBuf,
    store: PathBuf,
    old_protocol_ref: String,
    new_protocol_ref: String,
    gate_ref: String,
}

fn protocol_drain_fixture(dir: &Path) -> ProtocolDrainFixture {
    let ledger_root = dir.join("ledger");
    let lifecycle = protocol_session::request_response_lifecycle().expect("protocol lifecycle");
    let gate = protocol_gate_for_lifecycle(&lifecycle);
    let gate_ref = ledger::import_artifact(&ledger_root, &gate.value)
        .expect("import protocol gate")
        .artifact_ref;
    ProtocolDrainFixture {
        dir: dir.to_path_buf(),
        ledger_root,
        store: dir.join("upgrades"),
        old_protocol_ref: gate.protocol_ref,
        new_protocol_ref: test_ref("cli-protocol-v2"),
        gate_ref,
    }
}

fn protocol_gate_for_lifecycle(
    lifecycle: &protocol_session::RequestResponseLifecycle,
) -> protocol_session::ProtocolSessionGate {
    protocol_session::gate_protocol_session_lifecycle(protocol_session::ProtocolSessionGateInput {
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
    })
    .expect("protocol gate")
}

fn protocol_drain_with_ledger_gate_passes(fixture: &ProtocolDrainFixture) {
    let plan_value = protocol_drain_plan_value(
        "cli-protocol-drain",
        "drain protocol sessions before name cutover",
        &fixture.old_protocol_ref,
        &fixture.new_protocol_ref,
        fixture.gate_ref.clone(),
    );
    let plan = create_protocol_drain_session(
        fixture,
        plan_value,
        "protocol-drain-plan.preserves",
        "protocol-drain-create.preserves",
        fixture.store.clone(),
    );
    let receipt_out = fixture.dir.join("protocol-drain-task.preserves");
    run_protocol_drain_task(
        fixture.store.clone(),
        fixture.ledger_root.clone(),
        plan.plan_ref,
        &receipt_out,
    );
    let receipt = parse_protocol_drain_receipt(&receipt_out);
    assert_eq!(receipt.decision, "pass");
    assert!(to_text(&receipt.value).expect("receipt text").contains("protocol-session-drain"));
}

fn protocol_drain_without_ledger_gate_denies(fixture: ProtocolDrainFixture) {
    let missing_store = fixture.dir.join("missing-upgrades");
    let plan_value = protocol_drain_plan_value(
        "cli-protocol-drain-missing",
        "missing protocol gate evidence denies",
        &fixture.old_protocol_ref,
        &fixture.new_protocol_ref,
        test_ref("cli-missing-protocol-gate"),
    );
    let plan = create_protocol_drain_session(
        &fixture,
        plan_value,
        "protocol-drain-missing-plan.preserves",
        "protocol-drain-missing-create.preserves",
        missing_store.clone(),
    );
    let receipt_out = fixture.dir.join("protocol-drain-missing-task.preserves");
    run_protocol_drain_task(missing_store, fixture.dir.join("ledger"), plan.plan_ref, &receipt_out);
    let receipt = parse_protocol_drain_receipt(&receipt_out);
    assert_eq!(receipt.decision, "deny");
    assert!(to_text(&receipt.value)
        .expect("missing receipt text")
        .contains("not readable from ledger"));
}

fn protocol_drain_plan_value(
    session_id: &str,
    summary: &str,
    old_protocol_ref: &str,
    new_protocol_ref: &str,
    precondition_ref: String,
) -> preserves::IOValue {
    upgrades::upgrade_plan_value(&upgrades::UpgradePlanInput {
        session_id: session_id.to_string(),
        reason: "protocol drain".to_string(),
        summary: summary.to_string(),
        initiator_ref: test_ref("upgrade-initiator"),
        capability_refs: vec![test_ref("upgrade-capability")],
        affected_refs: vec![old_protocol_ref.to_string(), new_protocol_ref.to_string()],
        impact_refs: vec![old_protocol_ref.to_string()],
        tasks: vec![protocol_drain_task(old_protocol_ref, new_protocol_ref, precondition_ref)],
        compatibility: protocol_drain_compatibility(old_protocol_ref, new_protocol_ref),
        rollback_refs: vec![test_ref("rollback")],
        policy_refs: vec![test_ref("upgrade-policy")],
        evidence_refs: vec![test_ref("upgrade-evidence")],
        source_gate_receipt_values: vec![octet_gate::synthetic_clean_octet_gate_receipt_for_tests()
            .expect("source gate fixture")],
    })
    .expect("protocol drain plan")
}

fn protocol_drain_task(old_protocol_ref: &str, new_protocol_ref: &str, precondition_ref: String) -> upgrades::UpgradeTaskInput {
    upgrades::UpgradeTaskInput {
        task_id: "drain-sessions".to_string(),
        kind: "drain-sessions".to_string(),
        subject: "request-response-protocol".to_string(),
        from_ref: Some(old_protocol_ref.to_string()),
        to_ref: Some(new_protocol_ref.to_string()),
        precondition_refs: vec![precondition_ref],
        postcondition_refs: Vec::new(),
        reversible: false,
    }
}

fn protocol_drain_compatibility(old_protocol_ref: &str, new_protocol_ref: &str) -> upgrades::UpgradeCompatibilityWindow {
    upgrades::UpgradeCompatibilityWindow {
        old_refs: vec![old_protocol_ref.to_string()],
        new_refs: vec![new_protocol_ref.to_string()],
        expires_at: Some(64),
        policy_refs: vec![test_ref("compat-policy")],
    }
}

fn create_protocol_drain_session(
    fixture: &ProtocolDrainFixture,
    plan_value: preserves::IOValue,
    plan_name: &str,
    receipt_name: &str,
    store: PathBuf,
) -> upgrades::UpgradePlan {
    let plan_file = fixture.dir.join(plan_name);
    write_file(&plan_file, &to_text(&plan_value).expect("plan text")).expect("write protocol drain plan");
    let plan = upgrades::parse_upgrade_plan(&plan_value).expect("parse plan");
    run_upgrade_command(UpgradeCommand::Create {
        plan: plan_file,
        store,
        receipt_out: Some(fixture.dir.join(receipt_name)),
    })
    .expect("create protocol drain session");
    plan
}

fn run_protocol_drain_task(store: PathBuf, ledger: PathBuf, plan_ref: String, receipt_out: &Path) {
    run_upgrade_command(UpgradeCommand::RunTask {
        store,
        ledger,
        plan_ref,
        task_id: "drain-sessions".to_string(),
        receipt_out: Some(receipt_out.to_path_buf()),
    })
    .expect("run protocol drain task");
}

fn parse_protocol_drain_receipt(receipt_out: &Path) -> upgrades::UpgradeReceipt {
    upgrades::parse_upgrade_receipt(&read_preserves_file(receipt_out).expect("read receipt")).expect("parse receipt")
}
