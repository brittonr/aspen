
// r[verify molten.dag_sync.content_adapter_boundary]
#[test]
fn deferral_and_corruption_never_publish_false_completion() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut authority = Authority;
    let mut resources = Resources;
    let mut transport = Transport {
        calls: 0,
        defer_after: Some(DEFER_AFTER_FIRST_RESPONSE),
    };
    let mut content = Content { corrupt: false };
    let mut progress = Progress {
        loaded: None,
        stored: Vec::new(),
        events: events.clone(),
    };
    let mut observations = Observations { events: events.clone() };
    let mut receipts = Receipts {
        events: events.clone(),
        count: 0,
    };
    let partial = run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    })
    .expect("partial DAG sync");
    assert_eq!(partial.receipt.decision, DagSyncDecision::Partial);
    assert_eq!(partial.receipt.issues, vec![DagSyncIssue::TransferDeferred]);
    assert!(!partial.receipt.missing.is_empty());

    let events = Rc::new(RefCell::new(Vec::new()));
    let mut authority = Authority;
    let mut resources = Resources;
    let mut transport = Transport {
        calls: 0,
        defer_after: None,
    };
    let mut content = Content { corrupt: true };
    let mut progress = Progress {
        loaded: None,
        stored: Vec::new(),
        events: events.clone(),
    };
    let mut observations = Observations { events: events.clone() };
    let mut receipts = Receipts { events, count: 0 };
    let corrupt = run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    });
    assert!(corrupt.is_err());
    assert!(progress.stored.is_empty());
    assert_eq!(receipts.count, 0);
}

// r[verify molten.dag_sync.model]
// r[verify molten.dag_sync.domain_boundary]
#[test]
fn job_and_artifact_projections_preserve_domain_boundaries() {
    let job = fixture_job_dag();
    let job_graph = project_job_dag(&job).expect("job projection");
    assert_eq!(job_graph.roots.len(), 1);
    assert_eq!(job_graph.nodes.len(), EXPECTED_PROJECTED_NODES);
    assert_eq!(job_graph.roots.first().expect("root").domain, "molten-job-dag");

    let root_ref = digest('a');
    let dependency_ref = digest('b');
    let closure = crate::objects::ArtifactClosure {
        roots: vec![root_ref.clone()],
        closure_refs: vec![root_ref.clone(), dependency_ref.clone()],
        missing_refs: Vec::new(),
        closure_hash: digest('c'),
        receipt_value: crate::preserves_rail::record("fixture-closure", Vec::new()),
    };
    let edges = vec![crate::objects::ArtifactDependencyEdge {
        edge_ref: digest('d'),
        source_ref: root_ref,
        target_ref: dependency_ref,
        target_kind: "artifact".to_string(),
        relation: "requires".to_string(),
        required: true,
        scope: "runtime".to_string(),
        evidence_refs: Vec::new(),
        value: crate::preserves_rail::record("fixture-edge", Vec::new()),
    }];
    let artifact_graph = project_artifact_closure(&closure, &edges).expect("artifact projection");
    assert_eq!(artifact_graph.roots.len(), 1);
    assert_eq!(artifact_graph.nodes.len(), EXPECTED_PROJECTED_NODES);
    assert_eq!(artifact_graph.roots.first().expect("root").domain, "molten-artifact-closure");
}

/// A two-node source-to-sink job DAG with one stream edge.
fn fixture_job_dag() -> crate::workload::JobDag {
    let first_node = crate::workload::JobNode {
        id: "source".to_string(),
        kind: "fixture".to_string(),
        stage_artifact_ref: Some(digest('d')),
        input_ports: Vec::new(),
        output_ports: vec!["out".to_string()],
        config: crate::preserves_rail::record("fixture-config", Vec::new()),
        effect_manifest_refs: Vec::new(),
        policy_refs: Vec::new(),
        evidence_refs: Vec::new(),
        checks: Vec::new(),
    };
    let second_node = crate::workload::JobNode {
        id: "sink".to_string(),
        kind: "fixture".to_string(),
        stage_artifact_ref: None,
        input_ports: vec!["in".to_string()],
        output_ports: Vec::new(),
        config: crate::preserves_rail::record("fixture-config", Vec::new()),
        effect_manifest_refs: Vec::new(),
        policy_refs: Vec::new(),
        evidence_refs: Vec::new(),
        checks: Vec::new(),
    };
    crate::workload::JobDag {
        job_ref: digest('e'),
        version: "v1".to_string(),
        nodes: vec![first_node, second_node],
        edges: vec![crate::workload::JobEdge {
            from_node: "source".to_string(),
            from_port: "out".to_string(),
            to_node: "sink".to_string(),
            to_port: "in".to_string(),
            schema_ref: None,
            partitioning: "single".to_string(),
            materialization: "stream".to_string(),
        }],
        output_roots: vec!["sink".to_string()],
        schema_refs: vec![digest('f')],
        effect_manifest_refs: Vec::new(),
        policy_refs: Vec::new(),
        evidence_refs: Vec::new(),
        value: crate::preserves_rail::record("fixture-job", Vec::new()),
    }
}

fn conformance_outcome(kind: DagTransportFixtureKind) -> DagSyncOutcome {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut authority = Authority;
    let mut resources = Resources;
    let mut transport = DagFabricTransportAdapter::open(kind, None).expect("transport adapter");
    let mut content = Content { corrupt: false };
    let mut progress = Progress {
        loaded: None,
        stored: Vec::new(),
        events: events.clone(),
    };
    let mut observations = Observations { events: events.clone() };
    let mut receipts = Receipts { events, count: 0 };
    run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    })
    .expect("DAG conformance outcome")
}

// r[verify molten.dag_sync.final_validation]
#[test]
fn same_core_simulation_and_live_iroh_loopback_agree_on_canonical_outcome() {
    let simulated = conformance_outcome(DagTransportFixtureKind::DeterministicSimulation);
    let live = conformance_outcome(DagTransportFixtureKind::IrohLiveLoopback);
    assert_eq!(simulated.plan, live.plan);
    assert_eq!(simulated.progress, live.progress);
    assert_eq!(simulated.receipt, live.receipt);
    assert_eq!(simulated.canonical_receipt.bytes, live.canonical_receipt.bytes);
}

// r[verify molten.dag_sync.resume_fencing]
// r[verify molten.dag_sync.final_validation]
#[test]
fn partitioned_progress_resumes_after_restart_without_repeating_verified_refs() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut authority = Authority;
    let mut resources = Resources;
    let mut transport = DagFabricTransportAdapter::open(
        DagTransportFixtureKind::DeterministicSimulation,
        Some(DagTransportFixtureFault::PartitionAt {
            sequence: DEFER_AFTER_FIRST_RESPONSE,
        }),
    )
    .expect("partitioned transport");
    let mut content = Content { corrupt: false };
    let mut progress = Progress {
        loaded: None,
        stored: Vec::new(),
        events: events.clone(),
    };
    let mut observations = Observations { events: events.clone() };
    let mut receipts = Receipts {
        events: events.clone(),
        count: 0,
    };
    let first = run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    })
    .expect("partial sync");
    assert_eq!(first.receipt.decision, DagSyncDecision::Partial);
    assert_eq!(first.receipt.issues, vec![DagSyncIssue::TransferDeferred]);
    progress.loaded = progress.stored.last().cloned();

    let mut resumed_transport = DagFabricTransportAdapter::open(DagTransportFixtureKind::DeterministicSimulation, None)
        .expect("resumed transport");
    let resumed = run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut resumed_transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    })
    .expect("resumed sync");
    assert_eq!(resumed.receipt.decision, DagSyncDecision::Complete);
    assert_eq!(resumed.plan.requests.len(), 1);
    assert_eq!(resumed.progress.verified.len(), first.plan.missing.len());
    assert_eq!(receipts.count, 2);
}
