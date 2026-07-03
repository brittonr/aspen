
    #[hegel::test(test_cases = 16)]
    fn hegel_upgrade_plan_hash_task_order_and_impact_invariants(tc: TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let root = temp_dir("upgrade-hegel");
        let ledger_root = root.join("ledger");
        let base =
            crate::ledger::import_artifact(&ledger_root, &record("artifact", vec![string(format!("base-{salt}"))]))
                .expect("base")
                .artifact_ref;
        let dependent = crate::ledger::import_artifact(
            &ledger_root,
            &record("dependent", vec![string(&base), string(format!("dep-{salt}"))]),
        )
        .expect("dependent")
        .artifact_ref;
        let other =
            crate::ledger::import_artifact(&ledger_root, &record("other", vec![string(format!("other-{salt}"))]))
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

    fn protocol_drain_gate() -> crate::protocol_session::ProtocolSessionGate {
        protocol_drain_gate_with_diagnostics(Vec::new())
    }

    fn protocol_drain_gate_with_diagnostics(
        extra_diagnostics: Vec<String>,
    ) -> crate::protocol_session::ProtocolSessionGate {
        let lifecycle = crate::protocol_session::request_response_lifecycle().expect("protocol lifecycle");
        crate::protocol_session::gate_protocol_session_lifecycle_with_diagnostics(
            crate::protocol_session::ProtocolSessionGateInput {
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

    const UPGRADE_TEST_COMPATIBILITY_EXPIRY: u64 = 32;

    fn protocol_drain_gate_with_terminal_state_refs(
        gate: &crate::protocol_session::ProtocolSessionGate,
        terminal_state_refs: Vec<String>,
    ) -> IoValue {
        let receipt = crate::protocol_session::parse_protocol_session_gate_receipt(&gate.value)
            .expect("parse protocol gate receipt");
        let gate_status = if receipt.decision == "pass" { "pass" } else { "fail" };
        record("protocol-session-gate-receipt-v1", vec![
            string(crate::preserves_rail::PROTOCOL_SESSION_GATE_RECEIPT_SCHEMA),
            record("decision", vec![string(&receipt.decision)]),
            record("install", vec![string(&receipt.install_ref)]),
            record("protocol", vec![string(&receipt.protocol_ref)]),
            record("sessions", vec![strings_sequence(&receipt.session_ids)]),
            record("initial-states", vec![refs_sequence(&receipt.initial_state_refs)]),
            record("operations", vec![refs_sequence(&receipt.operation_refs)]),
            record("messages", vec![refs_sequence(&receipt.message_refs)]),
            record("final-states", vec![refs_sequence(&terminal_state_refs)]),
            record("diagnostics", vec![strings_sequence(&receipt.diagnostics)]),
            record("checks", vec![sequence(vec![
                record("check", vec![string("install-replay"), string(gate_status)]),
                record("check", vec![string("projected-operation-replay"), string(gate_status)]),
                record("check", vec![string("terminal-session-state"), string(gate_status)]),
                record("check", vec![string("transport-neutral-message"), string(gate_status)]),
                record("check", vec![string("protocol-session-gate-is-not-authority"), string("pass")]),
            ])]),
        ])
    }

    fn protocol_drain_plan_value(gate_ref: &str, old_protocol_ref: &str, new_protocol_ref: &str) -> Result<IoValue> {
        upgrade_plan_value(&protocol_drain_plan_input(gate_ref, old_protocol_ref, new_protocol_ref))
    }

    fn protocol_drain_cutover_plan_value(
        gate_ref: &str,
        old_protocol_ref: &str,
        new_protocol_ref: &str,
    ) -> Result<IoValue> {
        let mut input = protocol_drain_plan_input(gate_ref, old_protocol_ref, new_protocol_ref);
        input.tasks = vec![
            protocol_transcript_task_input(gate_ref),
            protocol_drain_task_input(gate_ref, old_protocol_ref, new_protocol_ref),
            protocol_cutover_task_input(old_protocol_ref, new_protocol_ref),
        ];
        upgrade_plan_value(&input)
    }

    fn protocol_drain_plan_input(gate_ref: &str, old_protocol_ref: &str, new_protocol_ref: &str) -> UpgradePlanInput {
        UpgradePlanInput {
            session_id: "session-protocol-drain".to_string(),
            reason: "protocol drain".to_string(),
            summary: "drain protocol sessions before cutover".to_string(),
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![old_protocol_ref.to_string(), new_protocol_ref.to_string()],
            impact_refs: vec![old_protocol_ref.to_string()],
            tasks: vec![protocol_drain_task_input(gate_ref, old_protocol_ref, new_protocol_ref)],
            compatibility: UpgradeCompatibilityWindow {
                old_refs: vec![old_protocol_ref.to_string()],
                new_refs: vec![new_protocol_ref.to_string()],
                expires_at: Some(UPGRADE_TEST_COMPATIBILITY_EXPIRY),
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: vec![old_protocol_ref.to_string()],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![gate_ref.to_string()],
            source_gate_receipt_values: source_gate_values(),
        }
    }

    fn protocol_drain_task_input(gate_ref: &str, old_protocol_ref: &str, new_protocol_ref: &str) -> UpgradeTaskInput {
        UpgradeTaskInput {
            task_id: "drain-sessions".to_string(),
            kind: "drain-sessions".to_string(),
            subject: "request-response-protocol".to_string(),
            from_ref: Some(old_protocol_ref.to_string()),
            to_ref: Some(new_protocol_ref.to_string()),
            precondition_refs: vec![gate_ref.to_string()],
            postcondition_refs: Vec::new(),
            reversible: false,
        }
    }

    fn protocol_transcript_task_input(gate_ref: &str) -> UpgradeTaskInput {
        UpgradeTaskInput {
            task_id: "transcript-gate".to_string(),
            kind: "transcript-rerun".to_string(),
            subject: "request-response-protocol".to_string(),
            from_ref: None,
            to_ref: None,
            precondition_refs: vec![gate_ref.to_string()],
            postcondition_refs: Vec::new(),
            reversible: true,
        }
    }

    fn protocol_cutover_task_input(old_protocol_ref: &str, new_protocol_ref: &str) -> UpgradeTaskInput {
        UpgradeTaskInput {
            task_id: "cutover".to_string(),
            kind: "cutover".to_string(),
            subject: "request-response-protocol".to_string(),
            from_ref: Some(old_protocol_ref.to_string()),
            to_ref: Some(new_protocol_ref.to_string()),
            precondition_refs: Vec::new(),
            postcondition_refs: Vec::new(),
            reversible: true,
        }
    }

    fn strings_sequence(values: &[String]) -> IoValue {
        sequence(values.iter().map(string).collect())
    }

    fn artifact_input(kind: &str, label: &str, dependency_refs: &[String]) -> crate::artifacts::ArtifactInstallInput {
        crate::artifacts::ArtifactInstallInput {
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
        vec![crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("source gate fixture")]
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
