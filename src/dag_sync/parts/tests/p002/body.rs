
// r[verify molten.dag_sync.final_validation]
#[test]
fn cancellation_corruption_and_authority_denial_fail_before_false_completion() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut authority = Authority;
    let mut resources = Resources;
    let mut cancelled_transport = DagFabricTransportAdapter::open(
        DagTransportFixtureKind::DeterministicSimulation,
        Some(DagTransportFixtureFault::CancelAt { sequence: 0 }),
    )
    .expect("cancel transport");
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
    let cancelled = run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut cancelled_transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    })
    .expect("cancelled sync");
    assert_eq!(cancelled.receipt.decision, DagSyncDecision::Partial);
    assert_eq!(cancelled.receipt.issues, vec![DagSyncIssue::TransferCancelled]);

    let mut corrupt_transport = DagFabricTransportAdapter::open(DagTransportFixtureKind::DeterministicSimulation, None)
        .expect("corrupt transport");
    let mut corrupt_content = Content { corrupt: true };
    let corrupt = run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut corrupt_transport,
        content: &mut corrupt_content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    });
    assert!(corrupt.is_err());

    let mut denied_authority = DeniedAuthority;
    let mut denied_transport = DagFabricTransportAdapter::open(DagTransportFixtureKind::DeterministicSimulation, None)
        .expect("denied transport");
    let denied = run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut denied_authority,
        resources: &mut resources,
        transport: &mut denied_transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    });
    assert!(denied.is_err());
    assert_eq!(denied_transport.request_count(), 0);
}

struct DeniedAuthority;

impl DagAuthorityPort for DeniedAuthority {
    fn observe_authority(&mut self, plan: &DagSyncPlan) -> Result<DagAuthorityObservation> {
        Ok(DagAuthorityObservation {
            authority_ref: digest('7'),
            plan_ref: plan.plan_ref.clone(),
            epoch_ref: plan.epoch_ref.clone(),
            generation: plan.generation,
            admitted: false,
        })
    }
}

// r[verify molten.dag_sync.resume_fencing]
// r[verify molten.dag_sync.final_validation]
#[test]
fn peer_reassignment_requires_a_new_epoch_and_discards_old_progress() {
    let mut peer_request = request();
    peer_request.strategy = DagSyncStrategy::PeerPartitioned;
    peer_request.peers = vec![
        DagPeerId::new("peer-a").expect("peer"),
        DagPeerId::new("peer-b").expect("peer"),
    ];
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
    let (mut progress, mut observations, mut receipts) = shared_event_ports();
    let _partial = run_dag_sync(&graph(), peer_request.clone(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    })
    .expect("partial peer sync");
    progress.loaded = progress.stored.last().cloned();

    peer_request.peers = vec![DagPeerId::new("peer-c").expect("peer")];
    let mut reassigned_transport =
        DagFabricTransportAdapter::open(DagTransportFixtureKind::DeterministicSimulation, None)
            .expect("reassigned transport");
    let rejected = run_dag_sync(&graph(), peer_request.clone(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut reassigned_transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    });
    assert!(rejected.is_err());
    assert_eq!(reassigned_transport.request_count(), 0);

    peer_request.epoch_ref = DagEpochRef::new(digest('9')).expect("new epoch");
    let (mut new_progress, mut new_observations, mut new_receipts) = shared_event_ports();
    let restarted = run_dag_sync(&graph(), peer_request, DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut reassigned_transport,
        content: &mut content,
        progress: &mut new_progress,
        observations: &mut new_observations,
        receipts: &mut new_receipts,
    })
    .expect("new epoch sync");
    assert_eq!(restarted.receipt.decision, DagSyncDecision::Complete);
}

/// Fresh progress, observation, and receipt ports that record into one shared event log.
fn shared_event_ports() -> (Progress, Observations, Receipts) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let progress = Progress {
        loaded: None,
        stored: Vec::new(),
        events: events.clone(),
    };
    let observations = Observations { events: events.clone() };
    (progress, observations, Receipts { events, count: 0 })
}

// r[verify molten.dag_sync.final_validation]
#[test]
fn dag_conformance_uses_fabric_ports_without_backend_imports() {
    let source = concat!(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/dag_sync/parts/conformance/p000/body.rs")),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/dag_sync/parts/conformance/p001/body.rs")),
    );
    assert!(!source.contains("iroh::"));
    assert!(source.contains("crate::fabric_transport"));
}
