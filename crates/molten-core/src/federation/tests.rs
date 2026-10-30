use super::*;

struct Scenario {
    bounds: PullBounds,
    peers: Vec<PeerHint>,
    inventory: InventoryObservation,
    local: Vec<InventoryEntry>,
}
impl Scenario {
    fn new() -> Self {
        let peer_ref = reference('1');
        let signer_ref = reference('2');
        Self {
            bounds: PullBounds {
                inventory_entries: 8,
                inventory_bytes: 4096,
                fetch_bytes: 1024,
                dag_nodes: 8,
                content_chunks: 8,
                concurrent_sessions: 2,
                query_interval: 10,
                freshness: 20,
                session_ticks: 30,
                retries: 1,
                retained_status: 32,
            },
            peers: vec![PeerHint {
                peer_ref: peer_ref.clone(),
                signer_ref: signer_ref.clone(),
                source: LocatorSource::Static,
                observed_at: 1,
                expires_at: 100,
                available: true,
            }],
            inventory: InventoryObservation {
                peer_ref,
                signer_ref,
                generation: 1,
                issued_at: 10,
                expires_at: 30,
                encoded_bytes: 128,
                entries: vec![entry("a", '3')],
            },
            local: Vec::new(),
        }
    }
    fn input(&self) -> PullInput<'_> {
        PullInput {
            bounds: &self.bounds,
            configured: &self.peers,
            inventory: &self.inventory,
            local: &self.local,
            generation: 1,
            now: 20,
            last_query: None,
            resource_history: &[],
            active_sessions: 0,
            receiver_requested: true,
        }
    }
}
fn reference(digit: char) -> String {
    format!("blake3:{}", digit.to_string().repeat(64))
}
fn entry(name: &str, digit: char) -> InventoryEntry {
    InventoryEntry {
        name: name.to_string(),
        head_ref: reference(digit),
        ancestry_ref: reference('a'),
        policy_ref: reference('b'),
        bytes: 64,
        dag_nodes: 1,
        chunks: 1,
    }
}

// r[verify molten.federated_pull_runtime.conflict_boundary]
#[test]
fn divergence_is_reported_without_fetching_or_overwriting() {
    let mut scenario = Scenario::new();
    scenario.local.push(entry("a", '4'));
    let plan = plan_pull(&scenario.input()).unwrap();
    assert_eq!(plan.missing, Vec::<InventoryEntry>::new());
    assert_eq!(plan.conflicts, vec![Conflict {
        name: "a".to_string(),
        local_ref: reference('4'),
        remote: entry("a", '3')
    }]);
    assert_eq!(scenario.local, vec![entry("a", '4')]);
}

// r[verify molten.federated_pull_runtime.receiver_pull]
#[test]
fn only_receiver_requested_missing_content_is_planned() {
    let mut scenario = Scenario::new();
    scenario.inventory.entries.push(entry("b", '5'));
    scenario.local.push(entry("a", '3'));
    assert_eq!(plan_pull(&scenario.input()).unwrap().missing, vec![entry("b", '5')]);
    let mut input = scenario.input();
    input.receiver_requested = false;
    assert_eq!(plan_pull(&input), Err(PullIssue::Unsolicited));
}

// r[verify molten.federated_pull_runtime.discovery_hints]
#[test]
fn observed_locator_cannot_substitute_for_static_peer_admission() {
    let mut scenario = Scenario::new();
    scenario.peers[0].source = LocatorSource::Dht;
    assert_eq!(plan_pull(&scenario.input()), Err(PullIssue::UnknownPeer));
}

// r[verify molten.federated_pull_runtime.bounds_freshness]
#[test]
fn stale_partitioned_oversized_and_exhausted_work_is_denied() {
    let mut scenario = Scenario::new();
    scenario.inventory.expires_at = 19;
    assert_eq!(plan_pull(&scenario.input()), Err(PullIssue::Stale));
    scenario.inventory.expires_at = 30;
    scenario.peers[0].available = false;
    assert_eq!(plan_pull(&scenario.input()), Err(PullIssue::Unavailable));
    scenario.peers[0].available = true;
    scenario.inventory.encoded_bytes = 4097;
    assert_eq!(plan_pull(&scenario.input()), Err(PullIssue::Inventory));
    scenario.inventory.encoded_bytes = 128;
    scenario.bounds.fetch_bytes = 63;
    assert_eq!(plan_pull(&scenario.input()), Err(PullIssue::Resource));
}

// r[verify molten.federated_pull_runtime.bounds_freshness]
#[test]
fn restart_preserves_rate_and_generation_fences() {
    let scenario = Scenario::new();
    let mut input = scenario.input();
    input.last_query = Some(15);
    assert_eq!(plan_pull(&input), Err(PullIssue::RateLimited));
    input.last_query = Some(10);
    assert_eq!(plan_pull(&input).unwrap().missing, vec![entry("a", '3')]);
    input.generation = 2;
    assert_eq!(plan_pull(&input), Err(PullIssue::Generation));
}

fn planned_session(scenario: &Scenario) -> Session {
    Session {
        session_ref: reference('c'),
        generation: 1,
        plan: plan_pull(&scenario.input()).unwrap(),
        phase: SessionPhase::Planned,
        verified: Vec::new(),
        admitted: Vec::new(),
        committed: Vec::new(),
        evidence_refs: vec![reference('d')],
        attempts: 0,
    }
}

#[test]
fn distributed_federation_partition_retry_restart_and_terminal_fences() {
    let scenario = Scenario::new();
    let planned = planned_session(&scenario);
    let fetching = advance_session(&planned, &SessionEvent::BeginFetch, 1, 20, &scenario.bounds).unwrap();
    let restored = fetching.clone();
    assert_eq!(restored.attempts, 1);
    assert_eq!(
        advance_session(&restored, &SessionEvent::BeginFetch, 2, 21, &scenario.bounds),
        Err(SessionIssue::Fence)
    );
    let retried = advance_session(&restored, &SessionEvent::BeginFetch, 1, 21, &scenario.bounds).unwrap();
    assert_eq!(retried.attempts, 2);
    assert_eq!(
        advance_session(&retried, &SessionEvent::BeginFetch, 1, 22, &scenario.bounds),
        Err(SessionIssue::Retry)
    );
    assert_eq!(
        advance_session(&retried, &SessionEvent::BeginFetch, 1, 51, &scenario.bounds),
        Err(SessionIssue::Deadline)
    );
    let denied = advance_session(
        &retried,
        &SessionEvent::Denied {
            evidence_ref: reference('e'),
        },
        1,
        51,
        &scenario.bounds,
    )
    .unwrap();
    assert_eq!(denied.phase, SessionPhase::Denied);
    assert_eq!(
        advance_session(&denied, &SessionEvent::BeginFetch, 1, 51, &scenario.bounds),
        Err(SessionIssue::Deadline)
    );
    assert!(denied.committed.is_empty());
}

#[test]
fn distributed_federation_corrupt_or_unsolicited_content_cannot_commit() {
    let scenario = Scenario::new();
    let session = planned_session(&scenario);
    let unverified = SessionEvent::Committed {
        head_ref: reference('3'),
        evidence_ref: reference('e'),
    };
    assert_eq!(advance_session(&session, &unverified, 1, 20, &scenario.bounds), Err(SessionIssue::Unadmitted));
    let pushed = SessionEvent::Verified {
        head_ref: reference('9'),
        evidence_ref: reference('e'),
    };
    assert_eq!(advance_session(&session, &pushed, 1, 20, &scenario.bounds), Err(SessionIssue::Unplanned));
    let corrupt = SessionEvent::Verified {
        head_ref: reference('3'),
        evidence_ref: "corrupt".to_string(),
    };
    assert_eq!(advance_session(&session, &corrupt, 1, 20, &scenario.bounds), Err(SessionIssue::Evidence));
    assert!(session.verified.is_empty());
    assert!(session.committed.is_empty());
}

#[test]
fn distributed_federation_denial_survives_full_evidence_budget() {
    let mut scenario = Scenario::new();
    let session = planned_session(&scenario);
    scenario.bounds.retained_status = 1;
    let denied = advance_session(
        &session,
        &SessionEvent::Denied {
            evidence_ref: reference('e'),
        },
        1,
        51,
        &scenario.bounds,
    )
    .unwrap();
    assert_eq!(denied.evidence_refs, vec![reference('e')]);
    assert_eq!(denied.phase, SessionPhase::Denied);
    assert_eq!(
        advance_session(
            &denied,
            &SessionEvent::Denied {
                evidence_ref: reference('f')
            },
            1,
            51,
            &scenario.bounds
        ),
        Err(SessionIssue::Terminal)
    );
}

#[test]
fn distributed_federation_malicious_signer_stale_inventory_and_resource_exhaustion() {
    let mut scenario = Scenario::new();
    scenario.inventory.signer_ref = reference('f');
    assert_eq!(plan_pull(&scenario.input()), Err(PullIssue::UnknownPeer));
    scenario.inventory.signer_ref = reference('2');
    scenario.inventory.issued_at = 0;
    let mut input = scenario.input();
    input.now = 21;
    assert_eq!(plan_pull(&input), Err(PullIssue::Stale));
    scenario.inventory.issued_at = 10;
    let mut input = scenario.input();
    input.active_sessions = 2;
    assert_eq!(plan_pull(&input), Err(PullIssue::Sessions));
}

#[test]
fn distributed_federation_preflights_all_import_evidence_before_fetch() {
    let mut scenario = Scenario::new();
    scenario.bounds.retained_status = 9;
    assert_eq!(plan_pull(&scenario.input()), Err(PullIssue::Resource));
    scenario.bounds.retained_status = 10;
    assert_eq!(plan_pull(&scenario.input()).unwrap().missing, vec![entry("a", '3')]);
    scenario.inventory.entries.push(entry("b", '4'));
    assert_eq!(plan_pull(&scenario.input()), Err(PullIssue::Resource));
    scenario.bounds.retained_status = 19;
    assert_eq!(plan_pull(&scenario.input()).unwrap().missing, vec![entry("a", '3'), entry("b", '4')]);
}

#[test]
fn distributed_federation_cross_peer_cannot_bypass_resource_rate_after_restart() {
    let scenario = Scenario::new();
    let plan = plan_pull(&scenario.input()).unwrap();
    let history = reserve_resources(&[], &plan, &scenario.bounds, 20).unwrap();
    let mut other = Scenario::new();
    other.inventory.peer_ref = reference('8');
    other.peers[0].peer_ref = reference('8');
    let restored = history.clone();
    let mut input = other.input();
    input.resource_history = &restored;
    input.last_query = None;
    assert_eq!(plan_pull(&input), Err(PullIssue::ResourceRateLimited));
    assert_eq!(restored[0].attempts, 1);
    assert_eq!(restored[0].bytes, 64);
}

#[test]
fn distributed_federation_resource_retry_and_cumulative_bytes_survive_new_session() {
    let mut scenario = Scenario::new();
    scenario.inventory.expires_at = 100;
    scenario.bounds.freshness = 100;
    let plan = plan_pull(&scenario.input()).unwrap();
    let first = reserve_resources(&[], &plan, &scenario.bounds, 20).unwrap();
    let second = reserve_resources(&first, &plan, &scenario.bounds, 30).unwrap();
    assert_eq!(second[0].attempts, 2);
    assert_eq!(second[0].bytes, 128);
    let mut input = scenario.input();
    input.now = 40;
    input.resource_history = &second;
    assert_eq!(plan_pull(&input), Err(PullIssue::ResourceRetry));
    scenario.bounds.fetch_bytes = 100;
    let mut input = scenario.input();
    input.now = 30;
    input.resource_history = &first;
    assert_eq!(plan_pull(&input), Err(PullIssue::Resource));
    let reset = reserve_resources(&second, &plan, &scenario.bounds, 50).unwrap();
    assert_eq!(reset[0].window_started, 50);
    assert_eq!(reset[0].attempts, 1);
    assert_eq!(reset[0].bytes, 64);
}

#[test]
fn distributed_federation_resource_history_is_bounded_and_rejects_future_or_duplicate_state() {
    let scenario = Scenario::new();
    let plan = plan_pull(&scenario.input()).unwrap();
    let first = reserve_resources(&[], &plan, &scenario.bounds, 20).unwrap();
    let duplicate = vec![first[0].clone(), first[0].clone()];
    assert_eq!(reserve_resources(&duplicate, &plan, &scenario.bounds, 20), Err(PullIssue::ResourceHistory));
    assert_eq!(reserve_resources(&first, &plan, &scenario.bounds, 19), Err(PullIssue::ResourceHistory));
    let full: Vec<_> = (0..MAX_FEDERATION_ENTRIES)
        .map(|index| ResourceHistory {
            head_ref: format!("blake3:{index:064x}"),
            window_started: 20,
            last_query: 20,
            attempts: 1,
            bytes: 1,
            dag_nodes: 1,
            chunks: 1,
        })
        .collect();
    assert_eq!(reserve_resources(&full, &plan, &scenario.bounds, 20), Err(PullIssue::ResourceHistory));
    let reset = reserve_resources(&full, &plan, &scenario.bounds, 50).unwrap();
    assert_eq!(reset.len(), 1);
    assert_eq!(reset[0].head_ref, reference('3'));
}
