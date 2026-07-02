
    #[test]
    fn ledger_catalog_and_mcp_classify_coordination_artifacts() {
        let manifest = coordination_fixture_manifest_value().expect("manifest");
        let mut runtime = new_coordination_runtime(&manifest).expect("runtime");
        let result = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_LOCK, OP_ACQUIRE, "resource:classify", "session", 1, None),
        )
        .expect("lock");
        assert_eq!(crate::ledger::artifact_kind(&manifest), "coordination-service-manifest");
        assert_eq!(crate::ledger::artifact_kind(&result.receipt.value), "coordination-receipt");
        assert_eq!(crate::ledger::artifact_kind(&result.assertions[0].value), "coordination-status-assertion");
        assert_eq!(
            crate::ledger::artifact_kind(&result.token.as_ref().expect("token").value),
            "coordination-fencing-token"
        );
        let report_evidence_refs = result
            .evidence_values
            .iter()
            .map(canonical_hash)
            .collect::<Result<Vec<_>>>()
            .expect("evidence refs");
        let manifest_ref = canonical_hash(&manifest).expect("manifest ref");
        let apply_report = coordination_apply_report_value(ApplyReportValueInput {
            decision: "pass",
            manifest_ref: &manifest_ref,
            final_state_ref: &result.state_snapshot.state_ref,
            receipt_refs: std::slice::from_ref(&result.receipt.receipt_ref),
            assertion_refs: std::slice::from_ref(&result.assertions[0].assertion_ref),
            evidence_refs: &report_evidence_refs,
        })
        .expect("apply report");
        assert_eq!(crate::ledger::artifact_kind(&apply_report), "coordination-apply-report");
        let root = temp_root("coordination-ledger-catalog");
        let registry_root = root.join("registry");
        let ledger_root = root.join("ledger");
        std::fs::create_dir_all(&registry_root).expect("registry root");
        crate::ledger::import_artifact(&ledger_root, &result.receipt.value).expect("import receipt");
        let list = crate::catalog::list(&registry_root, Some(&ledger_root), &crate::catalog::ListInput {
            kind: Some("coordination-receipt".to_string()),
            visibility: crate::catalog::VisibilityInput::default(),
        })
        .expect("catalog list");
        assert_eq!(list.decision, "pass");
        assert_eq!(list.items.len(), 1);
        let view_request =
            crate::catalog_mcp::mcp_request_value("catalog.view", vec![record("reference", vec![string(
                &result.receipt.receipt_ref,
            )])])
            .expect("mcp request");
        let call = crate::catalog_mcp::call(&registry_root, Some(&ledger_root), &view_request).expect("mcp call");
        assert_eq!(call.decision, "pass");
    }

    #[hegel::test(test_cases = 12)]
    fn hegel_fencing_fifo_semaphore_and_no_actor_traffic_invariants(tc: TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(1000));
        let mut runtime = runtime();
        let key = format!("resource:{salt}");
        let acquire =
            apply_coordination_request(&mut runtime, &request(SERVICE_LOCK, OP_ACQUIRE, &key, "owner", salt, None))
                .expect("acquire");
        assert_eq!(acquire.receipt.decision, "pass");
        let token = acquire.token.expect("token").token;
        assert!(token >= 1);
        let queue_key = format!("queue:{salt}");
        apply_coordination_request(
            &mut runtime,
            &request(SERVICE_QUEUE, OP_ENQUEUE, &queue_key, "p", salt + 1, Some(record("item", vec![string("first")]))),
        )
        .expect("enqueue first");
        apply_coordination_request(
            &mut runtime,
            &request(
                SERVICE_QUEUE,
                OP_ENQUEUE,
                &queue_key,
                "p",
                salt + 2,
                Some(record("item", vec![string("second")])),
            ),
        )
        .expect("enqueue second");
        assert_eq!(runtime.state.queues.get(&queue_key).expect("queue")[0], "first");
        let sem_key = format!("sem:{salt}");
        apply_coordination_request(
            &mut runtime,
            &request(SERVICE_SEMAPHORE, OP_ACQUIRE, &sem_key, "a", salt + 3, None),
        )
        .expect("sem a");
        assert!(
            set_len_u64(runtime.state.semaphores.get(&sem_key).expect("sem")).expect("sem count")
                <= runtime.manifest.semaphore_capacity
        );
        let snapshot_before_actor_message = snapshot_from_state(&runtime.state).expect("before").state_ref;
        let snapshot_after_actor_message = snapshot_from_state(&runtime.state).expect("after").state_ref;
        assert_eq!(snapshot_before_actor_message, snapshot_after_actor_message);
    }
