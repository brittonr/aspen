    #[test]
    fn cli_upgrade_protocol_drain_task_gates_on_ledger_protocol_evidence() {
        let dir = temp_dir("upgrade-cli-protocol-drain");
        let ledger_root = dir.join("ledger");
        let store = dir.join("upgrades");
        let lifecycle = protocol_session::request_response_lifecycle().expect("protocol lifecycle");
        let gate = protocol_session::gate_protocol_session_lifecycle(protocol_session::ProtocolSessionGateInput {
            install_receipt: lifecycle.install.value.clone(),
            initial_states: lifecycle.initial_states.iter().map(|state| state.value.clone()).collect(),
            operation_receipts: lifecycle.operations.iter().map(|operation| operation.receipt.value.clone()).collect(),
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
        .expect("protocol gate");
        let gate_ref = ledger::import_artifact(&ledger_root, &gate.value).expect("import protocol gate").artifact_ref;
        let old_protocol_ref = gate.protocol_ref.clone();
        let new_protocol_ref = test_ref("cli-protocol-v2");
        let plan_value = upgrades::upgrade_plan_value(&upgrades::UpgradePlanInput {
            session_id: "cli-protocol-drain".to_string(),
            reason: "protocol drain".to_string(),
            summary: "drain protocol sessions before name cutover".to_string(),
            initiator_ref: test_ref("upgrade-initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![old_protocol_ref.clone(), new_protocol_ref.clone()],
            impact_refs: vec![old_protocol_ref.clone()],
            tasks: vec![upgrades::UpgradeTaskInput {
                task_id: "drain-sessions".to_string(),
                kind: "drain-sessions".to_string(),
                subject: "request-response-protocol".to_string(),
                from_ref: Some(old_protocol_ref.clone()),
                to_ref: Some(new_protocol_ref.clone()),
                precondition_refs: vec![gate_ref],
                postcondition_refs: Vec::new(),
                reversible: false,
            }],
            compatibility: upgrades::UpgradeCompatibilityWindow {
                old_refs: vec![old_protocol_ref.clone()],
                new_refs: vec![new_protocol_ref.clone()],
                expires_at: Some(64),
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: vec![test_ref("rollback")],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("upgrade-evidence")],
            source_gate_receipt_values: vec![
                octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("source gate fixture"),
            ],
        })
        .expect("protocol drain plan");
        let plan_file = dir.join("protocol-drain-plan.preserves");
        write_file(&plan_file, &to_text(&plan_value).expect("plan text")).expect("write protocol drain plan");
        let plan = upgrades::parse_upgrade_plan(&plan_value).expect("parse plan");
        run_upgrade_command(UpgradeCommand::Create {
            plan: plan_file,
            store: store.clone(),
            receipt_out: Some(dir.join("protocol-drain-create.preserves")),
        })
        .expect("create protocol drain session");
        let receipt_out = dir.join("protocol-drain-task.preserves");
        run_upgrade_command(UpgradeCommand::RunTask {
            store,
            ledger: ledger_root,
            plan_ref: plan.plan_ref,
            task_id: "drain-sessions".to_string(),
            receipt_out: Some(receipt_out.clone()),
        })
        .expect("run protocol drain task");
        let receipt = upgrades::parse_upgrade_receipt(&read_preserves_file(&receipt_out).expect("read receipt"))
            .expect("parse receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(to_text(&receipt.value).expect("receipt text").contains("protocol-session-drain"));

        let missing_store = dir.join("missing-upgrades");
        let missing_gate_ref = test_ref("cli-missing-protocol-gate");
        let missing_plan = upgrades::upgrade_plan_value(&upgrades::UpgradePlanInput {
            session_id: "cli-protocol-drain-missing".to_string(),
            reason: "protocol drain".to_string(),
            summary: "missing protocol gate evidence denies".to_string(),
            initiator_ref: test_ref("upgrade-initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![old_protocol_ref.clone(), new_protocol_ref.clone()],
            impact_refs: vec![old_protocol_ref.clone()],
            tasks: vec![upgrades::UpgradeTaskInput {
                task_id: "drain-sessions".to_string(),
                kind: "drain-sessions".to_string(),
                subject: "request-response-protocol".to_string(),
                from_ref: Some(old_protocol_ref),
                to_ref: Some(new_protocol_ref),
                precondition_refs: vec![missing_gate_ref],
                postcondition_refs: Vec::new(),
                reversible: false,
            }],
            compatibility: upgrades::UpgradeCompatibilityWindow {
                old_refs: vec![test_ref("compat-old-protocol")],
                new_refs: vec![test_ref("compat-new-protocol")],
                expires_at: Some(64),
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: vec![test_ref("rollback")],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("upgrade-evidence")],
            source_gate_receipt_values: vec![
                octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("source gate fixture"),
            ],
        })
        .expect("missing protocol drain plan");
        let missing_plan_file = dir.join("protocol-drain-missing-plan.preserves");
        write_file(&missing_plan_file, &to_text(&missing_plan).expect("missing plan text"))
            .expect("write missing plan");
        let missing_plan = upgrades::parse_upgrade_plan(&missing_plan).expect("parse missing plan");
        run_upgrade_command(UpgradeCommand::Create {
            plan: missing_plan_file,
            store: missing_store.clone(),
            receipt_out: Some(dir.join("protocol-drain-missing-create.preserves")),
        })
        .expect("create missing protocol drain session");
        let missing_receipt_out = dir.join("protocol-drain-missing-task.preserves");
        run_upgrade_command(UpgradeCommand::RunTask {
            store: missing_store,
            ledger: dir.join("ledger"),
            plan_ref: missing_plan.plan_ref,
            task_id: "drain-sessions".to_string(),
            receipt_out: Some(missing_receipt_out.clone()),
        })
        .expect("run missing protocol drain task");
        let missing_receipt =
            upgrades::parse_upgrade_receipt(&read_preserves_file(&missing_receipt_out).expect("read missing receipt"))
                .expect("parse missing receipt");
        assert_eq!(missing_receipt.decision, "deny");
        assert!(to_text(&missing_receipt.value).expect("missing receipt text").contains("not readable from ledger"));
    }

    #[test]
    fn cli_chain_publish_fetch_commands_work() {
        let dir = temp_dir("chain-cli");
        let ledger = dir.join("ledger-source");
        let destination = dir.join("ledger-destination");
        let iroh_store = dir.join("chain-iroh");
        let chain = molten::evidence_chain::ChainScope::new("cli-chain", "artifact", "epoch");
        let payload_value =
            molten::preserves_rail::record("cli-chain-payload", vec![molten::preserves_rail::string("ok")]);
        let payload_ref = ledger::import_artifact(&ledger, &payload_value).expect("import chain payload").artifact_ref;
        let input = molten::evidence_chain::ChainLinkInput::genesis(
            chain.clone(),
            molten::evidence_chain::ChainPayload::new("cli-chain-payload", payload_ref, "molten.test.payload.v1"),
            Vec::new(),
            molten::evidence_chain::ChainProducer::new("node:cli", test_ref("producer-key")),
            test_ref("genesis-input"),
        );
        let link_value = molten::evidence_chain::chain_link_value(&input);
        let link = molten::evidence_chain::parse_chain_link(&link_value).expect("parse chain link");
        molten::evidence_chain::append_chain_link(&ledger, &link_value).expect("append chain link");

        run_chain_command(ChainCommand::Publish {
            ledger: ledger.clone(),
            iroh_store: iroh_store.clone(),
            scope: chain.scope.clone(),
            id: chain.id.clone(),
            epoch: chain.epoch.clone(),
            anchor: None,
            head: Some(link.link_ref.clone()),
            node: "node:cli".to_string(),
            fork_policy: "reject-unexpected-forks".to_string(),
            receipt_out: Some(dir.join("chain-publish.preserves")),
        })
        .expect("publish chain segment");
        let bundle_ref = only_blob_ref(&iroh_store);
        run_chain_command(ChainCommand::Fetch {
            ticket: format!("iroh-local-chain:{bundle_ref}"),
            ledger: destination.clone(),
            iroh_store,
            expected_bundle_ref: Some(bundle_ref),
            peer: "peer:cli".to_string(),
            fork_policy: "reject-unexpected-forks".to_string(),
            receipt_out: Some(dir.join("chain-fetch.preserves")),
        })
        .expect("fetch chain segment");
        let entries = ledger::list_artifacts(&destination).expect("list destination ledger");
        assert!(entries.iter().any(|entry| entry.artifact_kind == "chain-link"));
        assert!(entries.iter().any(|entry| entry.artifact_kind == "iroh-chain-exchange-receipt"));
    }
