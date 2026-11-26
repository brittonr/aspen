
/// Admits the first `admitted_observations` authority observations, then reports revocation.
struct RevokingAuthority {
    observations: usize,
    admitted_observations: usize,
}

impl DagAuthorityPort for RevokingAuthority {
    fn observe_authority(&mut self, plan: &DagSyncPlan) -> Result<DagAuthorityObservation> {
        self.observations += 1;
        Ok(DagAuthorityObservation {
            authority_ref: digest('7'),
            plan_ref: plan.plan_ref.clone(),
            epoch_ref: plan.epoch_ref.clone(),
            generation: plan.generation,
            admitted: self.observations <= self.admitted_observations,
        })
    }
}

/// Holds its reservation for the first `held_confirmations` confirmations, then reports it released.
struct ReleasingResources {
    confirmations: usize,
    held_confirmations: usize,
}

impl DagResourcePort for ReleasingResources {
    fn reserve(&mut self, plan: &DagSyncPlan) -> Result<DagResourceObservation> {
        Ok(DagResourceObservation {
            reservation_ref: digest('8'),
            plan_ref: plan.plan_ref.clone(),
            admitted: true,
        })
    }

    fn confirm(&mut self, plan: &DagSyncPlan, reservation_ref: &str) -> Result<DagResourceObservation> {
        self.confirmations += 1;
        Ok(DagResourceObservation {
            reservation_ref: reservation_ref.to_string(),
            plan_ref: plan.plan_ref.clone(),
            admitted: self.confirmations <= self.held_confirmations,
        })
    }
}

fn run_with<A: DagAuthorityPort, R: DagResourcePort>(
    authority: &mut A,
    resources: &mut R,
    transport: &mut Transport,
) -> Result<DagSyncOutcome> {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut content = Content { corrupt: false };
    let mut progress = Progress {
        loaded: None,
        stored: Vec::new(),
        events: events.clone(),
    };
    let mut observations = Observations { events: events.clone() };
    let mut receipts = Receipts { events, count: 0 };
    run_dag_sync(&graph(), request(), DagSyncPorts {
        authority,
        resources,
        transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    })
}

// r[verify molten.runtime_spine.admission_and_capability_hardening]
#[test]
fn authority_revoked_mid_transfer_stops_the_next_fetch() {
    let mut transport = Transport {
        calls: 0,
        defer_after: None,
    };
    let control = run_with(
        &mut RevokingAuthority {
            observations: 0,
            admitted_observations: usize::MAX,
        },
        &mut Resources,
        &mut transport,
    )
    .expect("positive control");
    let requests = control.plan.requests.len();
    assert!(requests >= 2, "fixture must transfer more than one object, got {requests}");
    assert_eq!(control.receipt.decision, DagSyncDecision::Complete);

    // The plan-time observation and the first fetch are admitted; revocation lands before the second.
    let mut transport = Transport {
        calls: 0,
        defer_after: None,
    };
    let revoked = run_with(
        &mut RevokingAuthority {
            observations: 0,
            admitted_observations: 2,
        },
        &mut Resources,
        &mut transport,
    );
    assert!(revoked.is_err(), "revocation observed mid-transfer must fail closed");
    assert_eq!(transport.calls, 1, "no object may be fetched after revocation");
}

// r[verify molten.runtime_spine.admission_and_capability_hardening]
#[test]
fn released_reservation_mid_transfer_stops_the_next_fetch() {
    let mut transport = Transport {
        calls: 0,
        defer_after: None,
    };
    let released = run_with(
        &mut Authority,
        &mut ReleasingResources {
            confirmations: 0,
            held_confirmations: 1,
        },
        &mut transport,
    );
    assert!(released.is_err(), "a released reservation must fail closed");
    assert_eq!(transport.calls, 1, "no object may be fetched after the reservation is released");
}
