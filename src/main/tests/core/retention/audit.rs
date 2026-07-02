fn audit_retention_gc(dir: &Path, root: &Path) {
    let audit_object_ref = cli_synthetic_ref("retention-audit-object").expect("audit object ref");
    let audit_object = RetentionCliObject {
        root,
        label: "retention-audit",
        object_ref: &audit_object_ref,
        object_kind: "encrypted-ref",
        retention_class: molten::retention::CLASS_PRIVATE_SECRET_REF,
        action: molten::retention::ACTION_DELETE,
    };
    let retention_args = retention_cli_args_for_object(audit_object);
    let audit_apply_ref = retention_apply_ref(audit_object, "ledger-gc", &retention_args);
    let audit_execution = store_audit_execution(root, &audit_object_ref, &audit_apply_ref);
    let audit_out = dir.join("gc-audit.preserves");
    run_retention_command(RetentionCommand::GcAudit(cli_retention::command::ops::GcAudit {
        root: root.to_path_buf(),
        execution_ref: audit_execution.execution_ref,
        out: Some(audit_out.clone()),
    }))
    .expect("retention gc audit");
    assert_audit_receipt(&audit_out, &audit_apply_ref);
    show_retention_artifact(audit_out, "show retention gc audit");
}

fn store_audit_execution(
    root: &Path,
    object_ref: &str,
    apply_ref: &str,
) -> molten::retention::GcExecutionGate {
    let audit_execution = molten::retention::store_gc_execution_gate(molten::retention::GcExecutionGateInput {
        root,
        subsystem: "ledger-gc",
        action: molten::retention::ACTION_DELETE,
        object_ref,
        object_kind: "encrypted-ref",
        retention_class: molten::retention::CLASS_PRIVATE_SECRET_REF,
        apply_ref: Some(apply_ref),
    })
    .expect("store audit execution gate");
    assert_eq!(audit_execution.decision, "pass");
    audit_execution
}

fn assert_audit_receipt(audit_out: &Path, audit_apply_ref: &str) {
    let audit = molten::retention::parse_gc_audit(&read_preserves_file(audit_out).expect("read retention gc audit"))
        .expect("parse retention gc audit");
    assert_eq!(audit.decision, "pass");
    assert_eq!(audit.apply_ref.as_deref(), Some(audit_apply_ref));
    assert!(audit.retention_receipt_ref.is_some());
    assert!(audit.tombstone_ref.is_some());
}
