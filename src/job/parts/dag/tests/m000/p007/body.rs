
    const JOB_DECISION_PASS: &str = "pass";
    const JOB_DECISION_DENY: &str = "deny";
    const GENERATED_JOB_MIN_NODES: u64 = 2;
    const GENERATED_JOB_MAX_NODES: u64 = 5;
    const JOB_OUT_OF_RANGE_DEPENDENCY_INDEX: u64 = 99;

    fn generated_node_id(index: usize) -> String {
        format!("n{index}")
    }

    fn generated_map_node(id: &str) -> IoValue {
        test_node_value(
            id,
            "map",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
        )
        .expect("generated map node")
    }

    fn generated_chain_dag(node_count: usize) -> JobDag {
        let mut nodes = Vec::with_capacity(node_count);
        let mut edges = Vec::with_capacity(node_count.saturating_sub(1));
        for index in 0..node_count {
            nodes.push(generated_map_node(&generated_node_id(index)));
            if index > 0 {
                edges.push(stream_edge_value(&generated_node_id(index.saturating_sub(1)), &generated_node_id(index))
                    .expect("generated edge"));
            }
        }
        let output_roots = vec![generated_node_id(node_count.saturating_sub(1))];
        let value = test_dag_value(nodes, edges, &output_roots).expect("generated dag");
        parse_job_dag_value(&value).expect("generated dag parses")
    }

    fn assert_plan_edges_are_ordered(dag: &JobDag, plan: &TrellisExecutionPlan) {
        let positions = plan
            .order_ids
            .iter()
            .enumerate()
            .map(|(index, node_id)| (node_id.clone(), index))
            .collect::<OrderedMap<_, _>>();
        for edge in &dag.edges {
            let from_position = positions.get(&edge.from_node).expect("from position");
            let to_position = positions.get(&edge.to_node).expect("to position");
            assert!(from_position < to_position);
            let from_index = usize_to_u64(*plan.node_index.get(&edge.from_node).expect("from index"), "from index")
                .expect("from index u64");
            assert!(plan
                .dependency_indices
                .get(&edge.to_node)
                .expect("to deps")
                .contains(&from_index));
        }
    }

    #[hegel::test(test_cases = 12)]
    fn hegel_job_dag_generated_topological_order_is_deterministic(tc: hegel::TestCase) {
        // r[verify molten.job_dag_state_machine_proof.topological_order_determinism]
        let node_count = usize::try_from(
            tc.draw(
                hegel::generators::integers::<u64>()
                    .min_value(GENERATED_JOB_MIN_NODES)
                    .max_value(GENERATED_JOB_MAX_NODES),
            ),
        )
        .expect("node count");
        let dag = generated_chain_dag(node_count);
        let first = trellis_execution_plan(&dag.nodes, &dag.edges).expect("first plan");
        let second = trellis_execution_plan(&dag.nodes, &dag.edges).expect("second plan");
        assert_eq!(first.order_ids, second.order_ids);
        assert_eq!(first.node_index, second.node_index);
        assert_eq!(first.dependency_indices, second.dependency_indices);
        assert_plan_edges_are_ordered(&dag, &first);
    }

    #[test]
    fn job_dag_topology_negative_cases_fail_closed() {
        // r[verify molten.job_dag_state_machine_proof.topological_order_determinism]
        let a = generated_map_node("a");
        let duplicate_a = generated_map_node("a");
        let duplicate = test_dag_value(vec![a.clone(), duplicate_a], Vec::new(), &["a".to_string()])
            .expect("duplicate dag value");
        assert!(parse_job_dag_value(&duplicate).expect_err("duplicate denied").to_string().contains("duplicate"));

        let unknown_edge = stream_edge_value("a", "missing").expect("unknown edge");
        let unknown = test_dag_value(vec![a.clone()], vec![unknown_edge], &["a".to_string()])
            .expect("unknown dag value");
        assert!(parse_job_dag_value(&unknown).expect_err("unknown denied").to_string().contains("unknown"));

        let b = generated_map_node("b");
        let ab = stream_edge_value("a", "b").expect("ab");
        let ba = stream_edge_value("b", "a").expect("ba");
        let cyclic = test_dag_value(vec![a.clone(), b], vec![ab, ba], &["b".to_string()])
            .expect("cycle dag value");
        assert!(parse_job_dag_value(&cyclic).expect_err("cycle denied").to_string().contains("trellis"));

        let node = parse_job_node_value(&a).expect("parse node");
        let mut plan = trellis_execution_plan(std::slice::from_ref(&node), &[]).expect("single-node plan");
        plan.dependency_indices.insert(node.id.clone(), vec![JOB_OUT_OF_RANGE_DEPENDENCY_INDEX]);
        assert!(dependency_ids(&plan, &node.id)
            .expect_err("out-of-range dependency denied")
            .to_string()
            .contains("has no node"));
    }

    #[test]
    fn job_dag_dependency_readiness_gate_denies_unsatisfied_and_missing_inputs() {
        // r[verify molten.job_dag_state_machine_proof.dependency_readiness_gate]
        let dag = generated_chain_dag(GENERATED_JOB_MIN_NODES as usize);
        let plan = trellis_execution_plan(&dag.nodes, &dag.edges).expect("plan");
        let dependent = generated_node_id(usize::try_from(GENERATED_JOB_MIN_NODES - 1).expect("dependent index"));
        let deps = plan.dependency_indices.get(&dependent).expect("dependent deps");
        assert!(!trellis::job_dag::all_deps_satisfied(deps, &[]));
        assert_ne!(trellis::job_dag::unsatisfied_count(deps, &[]), 0);
        let dependency_index = *deps.first().expect("dependency index");
        assert!(trellis::job_dag::all_deps_satisfied(deps, &[dependency_index]));
        assert_eq!(trellis::job_dag::unsatisfied_count(deps, &[dependency_index]), 0);

        let node = find_job_node(&dag.nodes, &dependent).expect("dependent node");
        let missing_outputs = vec![None; dag.nodes.len()];
        assert!(gather_inputs(node, &dag.edges, &missing_outputs, &plan.node_index)
            .expect_err("missing output slot denied")
            .to_string()
            .contains("not available"));

        let root = temp_dir("job-missing-executable-proof");
        let registry = root.join("registry");
        let missing_stage = generated_map_node("missing-stage");
        let dag_value = test_dag_value(vec![missing_stage], Vec::new(), &["missing-stage".to_string()])
            .expect("missing executable dag");
        let installed = install_job_dag(&registry, &dag_value).expect("install missing executable dag");
        assert_eq!(installed.decision, JOB_DECISION_PASS);
        let sync_ref = test_ref("missing-executable-sync");
        let policy_refs = vec![test_ref("missing-executable-policy")];
        let capability_refs = vec![test_ref("missing-executable-capability")];
        let evidence_refs = vec![sync_ref.clone(), test_ref("missing-executable-source-gate")];
        let resource_refs = vec![test_ref("missing-executable-resource")];
        let admission_request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &installed.job_ref,
            sync_ref: &sync_ref,
            stage_ids: &[],
            target_peer: "peer:missing-executable",
            policy_refs: &policy_refs,
            capability_refs: &capability_refs,
            evidence_refs: &evidence_refs,
            resource_refs: &resource_refs,
        })
        .expect("admission request");
        let admission = admission_plan_value(&registry, &admission_request).expect("admission plan");
        assert_eq!(admission.decision, JOB_DECISION_DENY);
        assert!(
            admission
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("lacks artifact-backed executable operation")),
            "missing executable diagnostics: {:?}",
            admission.diagnostics
        );
    }

    #[test]
    fn job_worker_schedule_replay_binds_stage_order_outputs_and_request() {
        // r[verify molten.job_dag_state_machine_proof.worker_schedule_replay]
        let fixture = worker_fixture("job-worker-schedule-replay-proof");
        let worker = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("schedule-replay-storage"),
            cache_root: &fixture.root.join("schedule-replay-cache"),
            chunk_root: &fixture.root.join("schedule-replay-chunks"),
            delivery: &fixture.delivery,
            delivery_log: Some(&fixture.delivery_log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: Some(&fixture.ledger),
        })
        .expect("worker execution");
        assert_eq!(worker.result.decision, JOB_DECISION_PASS, "{:?}", worker.result.diagnostics);
        let request = parse_job_worker_request_value(&fixture.worker_request).expect("worker request");
        let mut refs = fixture.evidence_refs.clone();
        refs.push(worker.receipt_ref.clone());
        refs.push(worker.result.result_ref.clone());
        refs.extend(worker.result.output_refs.iter().cloned());
        for (_, receipt_ref) in &worker.result.stage_receipt_refs {
            refs.push(receipt_ref.clone());
        }
        let empty_diagnostics = Vec::<String>::new();
        let schedule_value = job_worker_schedule_receipt_value(JobWorkerScheduleReceiptValueInput {
            operation: "worker-schedule-local",
            decision: JOB_DECISION_PASS,
            job_ref: &fixture.installed_job.job_ref,
            request_ref: &request.request_ref,
            queue_key: "queue:job-worker-proof",
            lease_key: "lock:job-worker-proof",
            worker_session: "worker-proof",
            coordination_report_ref: &test_ref("schedule-coordination-report"),
            enqueue_receipt_ref: Some(&test_ref("schedule-enqueue")),
            enqueue_duplicate_receipt_ref: Some(&test_ref("schedule-enqueue-duplicate")),
            dequeue_receipt_ref: Some(&test_ref("schedule-dequeue")),
            lease_receipt_ref: Some(&test_ref("schedule-lease")),
            release_receipt_ref: Some(&test_ref("schedule-release")),
            token_ref: Some(&test_ref("schedule-token")),
            worker_receipt_ref: Some(&worker.receipt_ref),
            result_ref: Some(&worker.result.result_ref),
            diagnostics: &empty_diagnostics,
            refs: &refs,
            checks: &[("worker-result-bound", "pass")],
        })
        .expect("schedule receipt value");
        let schedule = parse_job_worker_schedule_receipt_value(&schedule_value).expect("schedule receipt");
        let pass = validate_worker_schedule_replay(JobWorkerScheduleReplayInput {
            schedule: &schedule,
            request: &request,
            result: Some(&worker.result),
            expected_stage_order: &fixture.admission.plan.stage_order,
            expected_output_refs: &worker.result.output_refs,
            expected_diagnostics: &empty_diagnostics,
        })
        .expect("schedule replay pass");
        assert_eq!(pass.decision, JOB_DECISION_PASS);
        assert_eq!(pass.completed_indices.len(), fixture.admission.plan.stage_order.len());

        let mut reordered = fixture.admission.plan.stage_order.clone();
        reordered.reverse();
        let reordered_report = validate_worker_schedule_replay(JobWorkerScheduleReplayInput {
            schedule: &schedule,
            request: &request,
            result: Some(&worker.result),
            expected_stage_order: &reordered,
            expected_output_refs: &worker.result.output_refs,
            expected_diagnostics: &empty_diagnostics,
        })
        .expect("reordered replay report");
        assert_eq!(reordered_report.decision, JOB_DECISION_DENY);
        assert!(reordered_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("stage order")));

        let wrong_outputs = vec![test_ref("wrong-worker-output")];
        let output_report = validate_worker_schedule_replay(JobWorkerScheduleReplayInput {
            schedule: &schedule,
            request: &request,
            result: Some(&worker.result),
            expected_stage_order: &fixture.admission.plan.stage_order,
            expected_output_refs: &wrong_outputs,
            expected_diagnostics: &empty_diagnostics,
        })
        .expect("output replay report");
        assert_eq!(output_report.decision, JOB_DECISION_DENY);
        assert!(output_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("output refs")));

        let mut stale_request = request.clone();
        stale_request.request_ref = test_ref("stale-worker-request");
        let stale_report = validate_worker_schedule_replay(JobWorkerScheduleReplayInput {
            schedule: &schedule,
            request: &stale_request,
            result: Some(&worker.result),
            expected_stage_order: &fixture.admission.plan.stage_order,
            expected_output_refs: &worker.result.output_refs,
            expected_diagnostics: &empty_diagnostics,
        })
        .expect("stale replay report");
        assert_eq!(stale_report.decision, JOB_DECISION_DENY);
        assert!(stale_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("request ref")));
    }
