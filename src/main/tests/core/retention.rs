    #[test]
    fn cli_retention_commands_work() {
        let dir = temp_dir("retention-cli");
        let root = dir.join("store");
        let policy_ref = cli_synthetic_ref("retention-policy").expect("policy ref");
        let evidence_ref = cli_synthetic_ref("retention-evidence").expect("evidence ref");
        let authority_ref = cli_synthetic_ref("retention-authority").expect("authority ref");
        let owner_ref = cli_synthetic_ref("retention-owner").expect("owner ref");
        let object_ref = cli_synthetic_ref("retention-object").expect("object ref");
        let class_out = dir.join("class.preserves");
        run_retention_command(RetentionCommand::Class(cli_retention::command::base::Class {
            class_name: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            minimum_age_seconds: 0,
            maximum_age_seconds: Some(3600),
            deletion_authority_ref: authority_ref.clone(),
            policy_refs: vec![policy_ref.clone()],
            has_secret_redaction_hook: true,
            has_remote_gc_plan: true,
            has_compaction: false,
            out: Some(class_out.clone()),
        }))
        .expect("retention class");
        run_retention_command(RetentionCommand::Show(cli_retention::command::ops::Show {
            artifact: class_out.clone(),
        }))
        .expect("show retention class");
        let admission_out = dir.join("authority-admission.preserves");
        run_retention_command(RetentionCommand::Admit(cli_retention::command::base::Admit {
            root: root.clone(),
            kind: retention::ADMISSION_KIND_AUTHORITY.to_string(),
            decision: "pass".to_string(),
            requester_ref: owner_ref.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            action: retention::ACTION_DELETE.to_string(),
            bound_refs: vec![authority_ref.clone()],
            retained_refs: Vec::new(),
            remote_refs: Vec::new(),
            is_reference_index_complete: true,
            is_stale: false,
            revoked_refs: Vec::new(),
            diagnostics: Vec::new(),
            out: Some(admission_out.clone()),
        }))
        .expect("retention admission");
        run_retention_command(RetentionCommand::Show(cli_retention::command::ops::Show {
            artifact: admission_out,
        }))
        .expect("show retention admission");
        let clearance_out = dir.join("remote-clearance.preserves");
        let remote_ref = cli_synthetic_ref("retention-remote").expect("remote ref");
        let peer_ref = cli_synthetic_ref("retention-peer").expect("peer ref");
        run_retention_command(RetentionCommand::Clearance(cli_retention::command::base::Record {
            root: root.clone(),
            decision: "pass".to_string(),
            requester_ref: owner_ref.clone(),
            peer_ref: peer_ref.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            action: retention::ACTION_DELETE.to_string(),
            remote_ref: remote_ref.clone(),
            policy_ref: policy_ref.clone(),
            authority_ref: authority_ref.clone(),
            evidence_refs: vec![evidence_ref.clone()],
            retained_refs: Vec::new(),
            is_stale: false,
            revoked_refs: Vec::new(),
            diagnostics: Vec::new(),
            out: Some(clearance_out.clone()),
        }))
        .expect("retention remote clearance");
        run_retention_command(RetentionCommand::Show(cli_retention::command::ops::Show {
            artifact: clearance_out,
        }))
        .expect("show retention remote clearance");
        let request_out = dir.join("remote-clearance-request.preserves");
        run_retention_command(RetentionCommand::ClearanceRequest(cli_retention::command::base::Request {
            root: root.clone(),
            requester_ref: owner_ref.clone(),
            peer_ref: peer_ref.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            action: retention::ACTION_DELETE.to_string(),
            remote_ref: remote_ref.clone(),
            policy_ref: policy_ref.clone(),
            authority_ref: authority_ref.clone(),
            evidence_refs: vec![evidence_ref.clone()],
            out: Some(request_out.clone()),
        }))
        .expect("retention remote clearance request");
        run_retention_command(RetentionCommand::Show(cli_retention::command::ops::Show {
            artifact: request_out.clone(),
        }))
        .expect("show retention remote clearance request");
        let response_out = dir.join("remote-clearance-response.preserves");
        run_retention_command(RetentionCommand::ClearanceRespond(cli_retention::command::base::Respond {
            root: root.clone(),
            request: request_out.clone(),
            evidence_refs: vec![cli_synthetic_ref("retention-peer-evidence").expect("peer evidence ref")],
            retained_refs: Vec::new(),
            is_stale: false,
            revoked_refs: Vec::new(),
            diagnostics: Vec::new(),
            out: Some(response_out.clone()),
        }))
        .expect("retention remote clearance response");
        run_retention_command(RetentionCommand::Show(cli_retention::command::ops::Show {
            artifact: response_out.clone(),
        }))
        .expect("show retention remote clearance response");
        let import_out = dir.join("remote-clearance-import.preserves");
        run_retention_command(RetentionCommand::ClearanceImport(cli_retention::command::base::Import {
            root: root.clone(),
            request: request_out.clone(),
            response: response_out.clone(),
            expected_peer_ref: Some(peer_ref.clone()),
            expected_remote_ref: Some(remote_ref.clone()),
            out: Some(import_out.clone()),
        }))
        .expect("retention remote clearance import");
        run_retention_command(RetentionCommand::Show(cli_retention::command::ops::Show {
            artifact: import_out.clone(),
        }))
        .expect("show retention remote clearance import");
        let import = retention::parse_retention_remote_gc_clearance_import(
            &read_preserves_file(&import_out).expect("read clearance import"),
        )
        .expect("parse clearance import");
        assert_eq!(import.decision, "pass");
        assert!(import.clearance_ref.is_some());
        let retained_response_out = dir.join("remote-clearance-retained-response.preserves");
        run_retention_command(RetentionCommand::ClearanceRespond(cli_retention::command::base::Respond {
            root: root.clone(),
            request: request_out.clone(),
            evidence_refs: Vec::new(),
            retained_refs: vec![cli_synthetic_ref("retention-remote-retained").expect("remote retained ref")],
            is_stale: false,
            revoked_refs: Vec::new(),
            diagnostics: Vec::new(),
            out: Some(retained_response_out.clone()),
        }))
        .expect("retention retained remote clearance response");
        let retained_import_out = dir.join("remote-clearance-retained-import.preserves");
        run_retention_command(RetentionCommand::ClearanceImport(cli_retention::command::base::Import {
            root: root.clone(),
            request: request_out,
            response: retained_response_out,
            expected_peer_ref: Some(peer_ref),
            expected_remote_ref: Some(remote_ref),
            out: Some(retained_import_out.clone()),
        }))
        .expect("retention retained remote clearance import");
        let retained_import = retention::parse_retention_remote_gc_clearance_import(
            &read_preserves_file(&retained_import_out).expect("read retained clearance import"),
        )
        .expect("parse retained clearance import");
        assert_eq!(retained_import.decision, "deny");
        assert!(retained_import.clearance_ref.is_none());
        assert!(retained_import.diagnostics.iter().any(|diagnostic| diagnostic.contains("retained")));
        let pin_out = dir.join("pin.preserves");
        let pin_receipt_out = dir.join("pin-receipt.preserves");
        run_retention_command(RetentionCommand::Pin(cli_retention::command::base::Pin {
            root: root.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            source: retention::SOURCE_SECRET_REDACTION.to_string(),
            reason: "reveal audit pending".to_string(),
            owner_ref: owner_ref.clone(),
            expiry_ref: None,
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![evidence_ref.clone()],
            has_authority: true,
            pin_out: Some(pin_out.clone()),
            receipt_out: Some(pin_receipt_out.clone()),
        }))
        .expect("pin retention object");
        let pin = retention::parse_retention_pin(&read_preserves_file(&pin_out).expect("read pin")).expect("parse pin");
        let denied_receipt = dir.join("delete-denied.preserves");
        run_retention_command(RetentionCommand::Check(cli_retention::command::ops::Check {
            root: root.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            action: retention::ACTION_DELETE.to_string(),
            requester_ref: owner_ref.clone(),
            is_reference_index_complete: true,
            retained_refs: Vec::new(),
            remote_refs: Vec::new(),
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![evidence_ref.clone()],
            has_delete_authority: true,
            has_remote_gc_clearance: true,
            receipt_out: Some(denied_receipt.clone()),
        }))
        .expect("deny pinned delete");
        let denied =
            retention::parse_retention_receipt(&read_preserves_file(&denied_receipt).expect("read denied receipt"))
                .expect("parse denied receipt");
        assert_eq!(denied.decision, "deny");
        let unpin_receipt = dir.join("unpin-receipt.preserves");
        run_retention_command(RetentionCommand::Unpin(cli_retention::command::base::Unpin {
            root: root.clone(),
            pin_ref: pin.pin_ref,
            requester_ref: owner_ref.clone(),
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![evidence_ref.clone()],
            has_authority: true,
            receipt_out: Some(unpin_receipt),
        }))
        .expect("unpin retention object");
        let tombstone_receipt = dir.join("tombstone-receipt.preserves");
        run_retention_command(RetentionCommand::Check(cli_retention::command::ops::Check {
            root: root.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            action: retention::ACTION_TOMBSTONE.to_string(),
            requester_ref: owner_ref,
            is_reference_index_complete: true,
            retained_refs: Vec::new(),
            remote_refs: Vec::new(),
            policy_refs: vec![policy_ref],
            evidence_refs: vec![evidence_ref],
            has_delete_authority: true,
            has_remote_gc_clearance: true,
            receipt_out: Some(tombstone_receipt.clone()),
        }))
        .expect("tombstone retention object");
        let tombstone = retention::parse_retention_receipt(
            &read_preserves_file(&tombstone_receipt).expect("read tombstone receipt"),
        )
        .expect("parse tombstone receipt");
        assert_eq!(tombstone.decision, "pass");
        assert!(tombstone.tombstone_ref.is_some());
        let audit_object_ref = cli_synthetic_ref("retention-audit-object").expect("audit object ref");
        let audit_object = RetentionCliObject {
            root: &root,
            label: "retention-audit",
            object_ref: &audit_object_ref,
            object_kind: "encrypted-ref",
            retention_class: retention::CLASS_PRIVATE_SECRET_REF,
            action: retention::ACTION_DELETE,
        };
        let audit_retention = retention_cli_args_for_object(audit_object);
        let audit_apply_ref = retention_apply_ref(audit_object, "ledger-gc", &audit_retention);
        let audit_execution = retention::store_retention_gc_execution_gate(retention::RetentionGcExecutionGateInput {
            root: &root,
            subsystem: "ledger-gc",
            action: retention::ACTION_DELETE,
            object_ref: &audit_object_ref,
            object_kind: "encrypted-ref",
            retention_class: retention::CLASS_PRIVATE_SECRET_REF,
            apply_ref: Some(&audit_apply_ref),
        })
        .expect("store audit execution gate");
        assert_eq!(audit_execution.decision, "pass");
        let audit_out = dir.join("gc-audit.preserves");
        run_retention_command(RetentionCommand::GcAudit(cli_retention::command::ops::GcAudit {
            root: root.clone(),
            execution_ref: audit_execution.execution_ref,
            out: Some(audit_out.clone()),
        }))
        .expect("retention gc audit");
        let audit =
            retention::parse_retention_gc_audit(&read_preserves_file(&audit_out).expect("read retention gc audit"))
                .expect("parse retention gc audit");
        assert_eq!(audit.decision, "pass");
        assert_eq!(audit.apply_ref.as_deref(), Some(audit_apply_ref.as_str()));
        assert!(audit.retention_receipt_ref.is_some());
        assert!(audit.tombstone_ref.is_some());
        run_retention_command(RetentionCommand::Show(cli_retention::command::ops::Show { artifact: audit_out }))
            .expect("show retention gc audit");
        let fixture_out = dir.join("fixture");
        run_retention_command(RetentionCommand::RunFixture(cli_retention::command::ops::RunFixture {
            out: fixture_out.clone(),
        }))
        .expect("retention fixture");
        assert!(fixture_out.join("tombstone.preserves").exists());
    }
