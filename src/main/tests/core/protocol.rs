    #[test]
    fn cli_protocol_commands_work() {
        let dir = temp_dir("protocol-cli");
        let out = dir.join("request-response");
        run_protocol_command(ProtocolCommand::RunRequestResponse { out: out.clone() })
            .expect("run protocol request-response lifecycle");
        let receipt = out.join("install-receipt.preserves");
        assert!(receipt.exists());
        run_protocol_command(ProtocolCommand::Show {
            receipt: receipt.clone(),
        })
        .expect("show protocol install");
        let gate_receipt = dir.join("protocol-gate.preserves");
        run_protocol_command(ProtocolCommand::GateLifecycle {
            dir: out.clone(),
            receipt_out: Some(gate_receipt.clone()),
        })
        .expect("gate protocol lifecycle");
        let gate = protocol_session::parse_protocol_session_gate_receipt(
            &read_preserves_file(&gate_receipt).expect("read protocol gate receipt"),
        )
        .expect("parse protocol gate receipt");
        assert_eq!(gate.decision, "pass");
        let install_out = dir.join("install-only");
        run_protocol_command(ProtocolCommand::Install {
            manifest: out.join("manifest.preserves"),
            out: install_out.clone(),
        })
        .expect("install protocol manifest");
        assert!(read_preserves_file(&install_out.join("endpoints").join("endpoint-0.preserves")).is_ok());
    }

    #[test]
    fn cli_raft_commands_work() {
        let dir = temp_dir("raft-cli");
        let out = dir.join("fixture");
        run_raft_command(RaftCommand::RunFixture { out: out.clone() }).expect("run raft fixture");
        assert!(out.join("manifest.preserves").exists());
        assert!(out.join("state.preserves").exists());
        assert!(out.join("read-receipt.preserves").exists());
        assert!(out.join("snapshot.preserves").exists());
        run_raft_command(RaftCommand::Show {
            artifact: out.join("state.preserves"),
        })
        .expect("show raft state");
    }

    #[test]
    fn cli_delivery_idempotency_commands_work() {
        let dir = temp_dir("delivery-cli");
        let root = dir.join("store");
        let scope_out = dir.join("scope.preserves");
        let policy_ref = cli_synthetic_ref("delivery-policy").expect("policy ref");
        let evidence_ref = cli_synthetic_ref("delivery-evidence").expect("evidence ref");
        let payload_ref = cli_synthetic_ref("delivery-payload").expect("payload ref");
        let result_ref = cli_synthetic_ref("delivery-result").expect("result ref");
        run_delivery_command(DeliveryCommand::Scope {
            scope_profile: delivery_idempotency::SCOPE_REMOTE_TOPIC.to_string(),
            scope_name: "peer:b:services".to_string(),
            retention_refs: vec![policy_ref.clone()],
            out: Some(scope_out.clone()),
        })
        .expect("write delivery scope");
        assert!(scope_out.exists());
        let operation_out = dir.join("operation.preserves");
        run_delivery_command(DeliveryCommand::OperationId {
            scope_profile: delivery_idempotency::SCOPE_REMOTE_TOPIC.to_string(),
            scope_name: Some("peer:b:services".to_string()),
            scope_ref: None,
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 1,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_ref.clone(),
            policy_refs: vec![policy_ref.clone()],
            out: Some(operation_out.clone()),
        })
        .expect("write operation id");
        run_delivery_command(DeliveryCommand::Show {
            artifact: operation_out.clone(),
        })
        .expect("show operation id");
        let first_receipt = dir.join("first.preserves");
        run_delivery_command(DeliveryCommand::Check {
            root: root.clone(),
            scope_profile: delivery_idempotency::SCOPE_REMOTE_TOPIC.to_string(),
            scope_name: Some("peer:b:services".to_string()),
            scope_ref: None,
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 1,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_ref.clone(),
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![evidence_ref.clone()],
            semantic_result_ref: Some(result_ref.clone()),
            gap_policy: "deny".to_string(),
            receipt_out: Some(first_receipt.clone()),
        })
        .expect("first delivery check");
        let first = delivery_idempotency::parse_idempotency_receipt(
            &read_preserves_file(&first_receipt).expect("read first receipt"),
        )
        .expect("parse first receipt");
        assert_eq!(first.decision, "first");
        run_delivery_command(DeliveryCommand::ReceiptShow {
            receipt_ref: first.receipt_ref.clone(),
            root: root.clone(),
        })
        .expect("show stored receipt");
        let duplicate_receipt = dir.join("duplicate.preserves");
        run_delivery_command(DeliveryCommand::Check {
            root: root.clone(),
            scope_profile: delivery_idempotency::SCOPE_REMOTE_TOPIC.to_string(),
            scope_name: Some("peer:b:services".to_string()),
            scope_ref: None,
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 1,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_ref.clone(),
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![evidence_ref.clone()],
            semantic_result_ref: Some(result_ref),
            gap_policy: "deny".to_string(),
            receipt_out: Some(duplicate_receipt.clone()),
        })
        .expect("duplicate delivery check");
        let duplicate = delivery_idempotency::parse_idempotency_receipt(
            &read_preserves_file(&duplicate_receipt).expect("read duplicate receipt"),
        )
        .expect("parse duplicate receipt");
        assert_eq!(duplicate.decision, "duplicate");
        assert_eq!(duplicate.prior_receipt_ref.as_deref(), Some(first.receipt_ref.as_str()));
    }
