// r[verify molten.dag_sync.fabric_admission]
#[test]
fn resource_denial_fails_before_any_transfer() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut authority = Authority;
    let mut resources = DeniedResources;
    let mut transport = DagFabricTransportAdapter::open(DagTransportFixtureKind::DeterministicSimulation, None)
        .expect("resource-denied transport");
    let mut content = Content { corrupt: false };
    let mut progress = Progress {
        loaded: None,
        stored: Vec::new(),
        events: events.clone(),
    };
    let mut observations = Observations { events: events.clone() };
    let mut receipts = Receipts { events, count: 0 };
    let denied = run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    });
    assert!(denied.is_err(), "a denied reservation must not publish completion");
    assert_eq!(transport.request_count(), 0, "no fetch may start without an admitted reservation");
    assert!(progress.stored.is_empty(), "no verified progress may persist without an admitted reservation");
}

struct DeniedResources;

impl DagResourcePort for DeniedResources {
    fn reserve(&mut self, plan: &DagSyncPlan) -> Result<DagResourceObservation> {
        Ok(DagResourceObservation {
            reservation_ref: digest('8'),
            plan_ref: plan.plan_ref.clone(),
            admitted: false,
        })
    }
}
