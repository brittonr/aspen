use super::*;

fn pool(id: &str, kind: PoolKind, total: u64, free: Option<u64>, includes: bool) -> Pool {
    Pool {
        id: id.into(),
        kind,
        authority_id: format!("authority-{id}"),
        total,
        allocatable: total,
        observed_used: None,
        observed_free: free,
        is_observation_debited: includes,
    }
}

fn offer(host: &str, unified: bool) -> Offer {
    let memory = if unified { "shared-memory" } else { "device-memory" };
    Offer {
        schema: OFFER_SCHEMA.into(),
        host_id: host.into(),
        host_incarnation: "host-1".into(),
        device_incarnation: "device-1".into(),
        provider_profile_ref: "provider-v1".into(),
        source_profile_ref: "source-v1".into(),
        revision: 1,
        topology_generation: 1,
        observed_at: 10,
        observation_charge_cursor: None,
        expires_at: 100,
        health: Health::Healthy,
        label_authority: LabelAuthority::Authoritative,
        compatibility: Compatibility {
            backend: Some("backend-a".into()),
            runtime: Some(if unified { "unified" } else { "cuda" }.into()),
            architecture: Some("arch-a".into()),
            topology: Some("single".into()),
            qualification_refs: vec!["qualified-profile".into()],
        },
        pools: vec![
            pool(
                memory,
                if unified {
                    PoolKind::UnifiedMemoryBytes
                } else {
                    PoolKind::DedicatedMemoryBytes
                },
                100,
                Some(100),
                false,
            ),
            pool("chip", PoolKind::Chips, 1, Some(1), false),
        ],
        views: vec![View {
            id: "device".into(),
            device_id: "opaque-device".into(),
            partition_id: None,
            granularity: Granularity::MmioBundle,
            backing_pools: vec![memory.into(), "chip".into()],
            is_grouping_enforced: true,
        }],
    }
}

fn input(host: &str, operation: &str, memory: &str, amount: u64) -> ReservationInput {
    ReservationInput {
        host_id: host.into(),
        host_incarnation: "host-1".into(),
        device_incarnation: "device-1".into(),
        offer_revision: 1,
        topology_generation: 1,
        now: 20,
        view_id: "device".into(),
        granularity: Granularity::MmioBundle,
        compatibility: RequiredCompatibility {
            backend: Some("backend-a".into()),
            runtime: Some(if host == "host-b" { "unified" } else { "cuda" }.into()),
            architecture: Some("arch-a".into()),
            topology: Some("single".into()),
        },
        minimum_label_authority: LabelAuthority::Authoritative,
        consumer_id: format!("consumer-{operation}"),
        operation_id: operation.into(),
        attempt_id: format!("attempt-{operation}"),
        owner_generation: 1,
        grant_ref: format!("grant-{operation}"),
        grant: GrantFacts {
            grant_ref: format!("grant-{operation}"),
            consumer_id: format!("consumer-{operation}"),
            is_valid: true,
            is_revoked: false,
            expires_at: 80,
        },
        claims: vec![
            Claim {
                pool_id: memory.into(),
                quantity: amount,
            },
            Claim {
                pool_id: "chip".into(),
                quantity: 1,
            },
        ],
        serving_allocation_id: None,
        kind: AllocationKind::Execution,
    }
}

fn two_hosts() -> CapacityState {
    let state = observe(&CapacityState::default(), offer("host-a", false)).unwrap();
    observe(&state, offer("host-b", true)).unwrap()
}

fn reserved(state: &CapacityState, input: &ReservationInput) -> CapacityState {
    let result = reserve(state, input);
    assert!(matches!(result.decision, ReservationOutcome::Reserved { .. }), "{:?}", result.decision);
    result.state
}

fn owner(id: &str) -> OwnerInput {
    OwnerInput {
        allocation_id: id.into(),
        consumer_id: format!("consumer-{id}"),
        owner_generation: 1,
    }
}

fn evidence(id: &str) -> BackendReleaseEvidence {
    BackendReleaseEvidence {
        allocation_id: id.into(),
        owner_generation: 1,
        backend_ref: "stop-ack".into(),
        is_released: true,
        enforcement: EnforcementStrength::BackendAcknowledged,
    }
}

// r[verify molten.compute_capacity.offers]
#[test]
fn compute_capacity_two_hosts_offer_eligibility_and_reasons() {
    let state = two_hosts();
    let request = input("host-a", "a", "device-memory", 90);
    assert_eq!(eligibility(&state, &request), Eligibility::Eligible);
    let mut mismatch = request.clone();
    mismatch.compatibility.runtime = Some("unified".into());
    assert_eq!(eligibility(&state, &mismatch), Eligibility::Rejected(CapacityIssue::Incompatible));
    let mut stale = request.clone();
    stale.now = 100;
    assert_eq!(eligibility(&state, &stale), Eligibility::Rejected(CapacityIssue::Stale));
    let mut unsupported = request.clone();
    unsupported.granularity = Granularity::Mesh;
    assert_eq!(eligibility(&state, &unsupported), Eligibility::Rejected(CapacityIssue::Unsupported));
    let mut unavailable = state.clone();
    unavailable.offers.get_mut("host-a").unwrap().health = Health::Unavailable;
    assert_eq!(eligibility(&unavailable, &request), Eligibility::Rejected(CapacityIssue::Unavailable));
    let mut too_large = request;
    too_large.claims[0].quantity = 101;
    assert_eq!(eligibility(&state, &too_large), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
    assert_eq!(eligibility(&state, &input("host-b", "b", "shared-memory", 90)), Eligibility::Eligible);
}

// r[verify molten.compute_capacity.backing_pools]
// r[verify molten.compute_capacity.reservation_authority]
#[test]
fn compute_capacity_atomic_shared_pool_alias_partition_and_idempotency() {
    let mut state = two_hosts();
    let offer = state.offers.get_mut("host-b").unwrap();
    offer.pools.retain(|p| p.id == "shared-memory");
    offer.views = ["cpu", "accelerator", "alias", "partition"]
        .into_iter()
        .map(|id| View {
            id: id.into(),
            device_id: "same-device".into(),
            partition_id: Some(id.into()),
            granularity: if id == "partition" {
                Granularity::Partition
            } else {
                Granularity::Alias
            },
            backing_pools: vec!["shared-memory".into()],
            is_grouping_enforced: true,
        })
        .collect();
    let mut first = input("host-b", "first", "shared-memory", 60);
    first.claims.pop();
    first.view_id = "cpu".into();
    first.granularity = Granularity::Alias;
    state = reserved(&state, &first);
    let duplicate = reserve(&state, &first);
    assert!(matches!(duplicate.decision, ReservationOutcome::Existing { .. }));
    assert_eq!(duplicate.state, state);
    let mut second = input("host-b", "second", "shared-memory", 60);
    second.claims.pop();
    second.view_id = "accelerator".into();
    second.granularity = Granularity::Alias;
    assert_eq!(eligibility(&state, &second), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
    for view in ["alias", "partition"] {
        second.view_id = view.into();
        second.granularity = if view == "partition" {
            Granularity::Partition
        } else {
            Granularity::Alias
        };
        assert_eq!(eligibility(&state, &second), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
    }
    assert_eq!(state.allocations.len(), 1);
    let mut conflicting = first.clone();
    conflicting.consumer_id = "other".into();
    assert!(matches!(
        reserve(&state, &conflicting).decision,
        ReservationOutcome::Rejected(CapacityIssue::WrongOwner)
    ));
}

// r[verify molten.compute_capacity.backing_pools]
#[test]
fn compute_capacity_observed_free_basis_and_missing_observations() {
    let mut state = two_hosts();
    let offer = state.offers.get_mut("host-b").unwrap();
    offer.pools.retain(|p| p.id == "shared-memory");
    offer.views[0].backing_pools = vec!["shared-memory".into()];
    let mut first = input("host-b", "first", "shared-memory", 40);
    first.claims.pop();
    state = reserved(&state, &first);
    let mut next_offer = state.offers["host-b"].clone();
    next_offer.revision = 2;
    next_offer.observed_at = 21;
    next_offer.pools[0].observed_free = Some(60);
    next_offer.pools[0].is_observation_debited = true;
    next_offer.observation_charge_cursor = Some(state.charge_cursor);
    state = observe(&state, next_offer).unwrap();
    let mut second = input("host-b", "second", "shared-memory", 60);
    second.claims.pop();
    second.offer_revision = 2;
    second.now = 22;
    assert_eq!(eligibility(&state, &second), Eligibility::Eligible);
    state = reserved(&state, &second);
    let mut third = input("host-b", "third", "shared-memory", 1);
    third.claims.pop();
    third.offer_revision = 2;
    third.now = 22;
    assert_eq!(eligibility(&state, &third), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
    let mut missing = state.offers["host-b"].clone();
    missing.revision = 3;
    missing.observed_at = 23;
    missing.pools[0].observed_free = None;
    missing.pools[0].observed_used = None;
    state = observe(&state, missing).unwrap();
    third.offer_revision = 3;
    third.now = 24;
    assert_eq!(eligibility(&state, &third), Eligibility::Rejected(CapacityIssue::InsufficientEvidence));
}

// r[verify molten.compute_capacity.reservation_authority]
#[test]
fn compute_capacity_cross_host_serving_is_charged_once() {
    let mut host_a = offer("host-a", false);
    host_a.pools.push(pool("host-memory", PoolKind::UnifiedMemoryBytes, 100, Some(100), false));
    host_a.views.push(View {
        id: "cpu-execution".into(),
        device_id: "host-a-cpu".into(),
        partition_id: None,
        granularity: Granularity::Alias,
        backing_pools: vec!["host-memory".into()],
        is_grouping_enforced: true,
    });
    let state = observe(&CapacityState::default(), host_a).unwrap();
    let mut state = observe(&state, offer("host-b", true)).unwrap();
    let mut serving = input("host-b", "serving", "shared-memory", 80);
    serving.kind = AllocationKind::ServingDeployment;
    state = reserved(&state, &serving);
    // Independent agent executions on host A reference deployment B.
    for operation in ["worker1", "worker2"] {
        let mut worker = input("host-a", operation, "host-memory", 40);
        worker.serving_allocation_id = Some("serving".into());
        worker.view_id = "cpu-execution".into();
        worker.granularity = Granularity::Alias;
        worker.claims.pop();
        state = reserved(&state, &worker);
    }
    assert_eq!(charged(&state, "host-b", "shared-memory"), Some(80));
    assert_eq!(state.allocations.len(), 3);
    assert_eq!(
        release(&state, &ReleaseInput {
            owner: owner("serving"),
            backend_evidence: Some(evidence("serving"))
        })
        .issue,
        Some(CapacityIssue::UncertainRelease)
    );
}

// r[verify molten.compute_capacity.reservation_authority]
#[test]
fn compute_capacity_rechecks_authority_before_effects() {
    let state = two_hosts();
    let request = input("host-a", "op", "device-memory", 10);
    let mut cases = Vec::new();
    let mut stale_host = request.clone();
    stale_host.host_incarnation = "previous".into();
    cases.push((stale_host, CapacityIssue::Stale));
    let mut stale_device = request.clone();
    stale_device.device_incarnation = "previous".into();
    cases.push((stale_device, CapacityIssue::Stale));
    let mut stale_revision = request.clone();
    stale_revision.offer_revision = 0;
    cases.push((stale_revision, CapacityIssue::Stale));
    let mut expired = request.clone();
    expired.now = 100;
    cases.push((expired, CapacityIssue::Stale));
    let mut incompatible = request.clone();
    incompatible.compatibility.backend = Some("other".into());
    cases.push((incompatible, CapacityIssue::Incompatible));
    let mut revoked = request.clone();
    revoked.grant.is_revoked = true;
    cases.push((revoked, CapacityIssue::RevokedGrant));
    let mut invalid = request.clone();
    invalid.grant.is_valid = false;
    cases.push((invalid, CapacityIssue::InvalidGrant));
    for (input, issue) in cases {
        let result = reserve(&state, &input);
        assert_eq!(result.decision, ReservationOutcome::Rejected(issue));
        assert_eq!(result.state, state);
    }
    let mut weak = state.clone();
    weak.offers.get_mut("host-a").unwrap().label_authority = LabelAuthority::Advisory;
    assert_eq!(reserve(&weak, &request).decision, ReservationOutcome::Rejected(CapacityIssue::WeakAuthority));
    let mut unknown = state.clone();
    unknown.offers.get_mut("host-a").unwrap().compatibility.runtime = None;
    assert_eq!(
        reserve(&unknown, &request).decision,
        ReservationOutcome::Rejected(CapacityIssue::InsufficientEvidence)
    );
    assert!(matches!(reserve(&state, &request).decision, ReservationOutcome::Reserved { effects, .. }
        if effects == vec![EffectIntent::PersistReservation("op".into()), EffectIntent::Activate("op".into())]));
}

// r[verify molten.compute_capacity.uncertainty_release]
// r[verify molten.compute_capacity.readback]
#[test]
fn compute_capacity_uncertainty_and_release_evidence() {
    for event in [
        LifecycleEvent::Drain,
        LifecycleEvent::Cancel,
        LifecycleEvent::Timeout,
        LifecycleEvent::LostStopAcknowledgement,
        LifecycleEvent::BackendDisappeared,
    ] {
        let state = reserved(&two_hosts(), &input("host-a", "op", "device-memory", 80));
        let uncertain = lifecycle(&state, &owner("op"), event);
        assert_eq!(uncertain.state.allocations["op"].status, ReservationStatus::Quarantined);
        assert_eq!(charged(&uncertain.state, "host-a", "device-memory"), Some(80));
        assert_eq!(
            release(&uncertain.state, &ReleaseInput {
                owner: owner("op"),
                backend_evidence: None
            })
            .issue,
            Some(CapacityIssue::UncertainRelease)
        );
        let readback = inspect(&uncertain.state, "host-a", 20).unwrap();
        assert!(readback.is_fresh);
        assert_eq!(readback.qualification_refs, vec!["qualified-profile"]);
        assert!(readback.allocations[0].is_logically_fenced);
        assert_eq!(readback.allocations[0].enforcement, EnforcementStrength::LogicalFenceOnly);
        let mut wrong = owner("op");
        wrong.owner_generation = 0;
        assert_eq!(
            release(&uncertain.state, &ReleaseInput {
                owner: wrong,
                backend_evidence: Some(evidence("op"))
            })
            .issue,
            Some(CapacityIssue::WrongOwner)
        );
        let mut wrong_consumer = owner("op");
        wrong_consumer.consumer_id = "another".into();
        assert_eq!(
            release(&uncertain.state, &ReleaseInput {
                owner: wrong_consumer,
                backend_evidence: Some(evidence("op"))
            })
            .issue,
            Some(CapacityIssue::WrongOwner)
        );
        assert_eq!(
            release(&uncertain.state, &ReleaseInput {
                owner: owner("op"),
                backend_evidence: Some(BackendReleaseEvidence {
                    owner_generation: 0,
                    ..evidence("op")
                })
            })
            .issue,
            Some(CapacityIssue::UncertainRelease)
        );
        let released = release(&uncertain.state, &ReleaseInput {
            owner: owner("op"),
            backend_evidence: Some(evidence("op")),
        });
        assert_eq!(released.issue, None);
        assert!(!released.state.allocations.contains_key("op"));
        assert_eq!(released.state.retired_allocations.back().unwrap().status, ReservationStatus::Released);
        assert_eq!(charged(&released.state, "host-a", "device-memory"), Some(0));
    }
}

// r[verify molten.compute_capacity.backing_pools]
// r[verify molten.compute_capacity.offers]
// r[verify molten.compute_capacity.fixtures]
#[test]
fn compute_capacity_synthetic_tenstorrent_overlap_and_unsupported_grouping() {
    let mut tt = offer("tt-host", false);
    tt.compatibility.backend = Some("tt-metal".into());
    tt.compatibility.runtime = Some("tt-runtime".into());
    tt.compatibility.topology = Some("galaxy-rev-c".into());
    tt.pools = ["chip-mmio", "chip-remote-a", "chip-remote-b"]
        .into_iter()
        .map(|id| pool(id, PoolKind::Chips, 1, Some(1), false))
        .collect();
    tt.views = vec![
        View {
            id: "chip".into(),
            device_id: "mmio-device".into(),
            partition_id: None,
            granularity: Granularity::Chip,
            backing_pools: vec!["chip-mmio".into()],
            is_grouping_enforced: true,
        },
        View {
            id: "tray".into(),
            device_id: "mmio-device".into(),
            partition_id: None,
            granularity: Granularity::Tray,
            backing_pools: vec!["chip-mmio".into(), "chip-remote-a".into()],
            is_grouping_enforced: true,
        },
        View {
            id: "mesh-1".into(),
            device_id: "mmio-device".into(),
            partition_id: None,
            granularity: Granularity::Mesh,
            backing_pools: vec!["chip-mmio".into(), "chip-remote-a".into()],
            is_grouping_enforced: true,
        },
        View {
            id: "mesh-2".into(),
            device_id: "mmio-device".into(),
            partition_id: None,
            granularity: Granularity::Mesh,
            backing_pools: vec!["chip-remote-a".into(), "chip-remote-b".into()],
            is_grouping_enforced: true,
        },
    ];
    let state = observe(&CapacityState::default(), tt).unwrap();
    let request = |view: &str, operation: &str, claims: &[&str]| {
        let mut request = input("tt-host", operation, "chip-mmio", 1);
        request.compatibility.backend = Some("tt-metal".into());
        request.compatibility.runtime = Some("tt-runtime".into());
        request.compatibility.topology = Some("galaxy-rev-c".into());
        request.view_id = view.into();
        request.granularity = if view == "chip" {
            Granularity::Chip
        } else if view == "tray" {
            Granularity::Tray
        } else {
            Granularity::Mesh
        };
        request.claims = claims
            .iter()
            .map(|id| Claim {
                pool_id: (*id).into(),
                quantity: 1,
            })
            .collect();
        request
    };
    let chip = request("chip", "chip-op", &["chip-mmio"]);
    let tray = request("tray", "tray-op", &["chip-mmio", "chip-remote-a"]);
    let mesh1 = request("mesh-1", "mesh1-op", &["chip-mmio", "chip-remote-a"]);
    let mesh2 = request("mesh-2", "mesh2-op", &["chip-remote-a", "chip-remote-b"]);
    let after_chip = reserved(&state, &chip);
    assert_eq!(eligibility(&after_chip, &tray), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
    assert_eq!(eligibility(&after_chip, &mesh1), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
    let after_mesh2 = reserved(&state, &mesh2);
    assert_eq!(eligibility(&after_mesh2, &tray), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
    assert_eq!(eligibility(&after_mesh2, &mesh1), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
    let mut unsupported = state.clone();
    unsupported
        .offers
        .get_mut("tt-host")
        .unwrap()
        .views
        .retain(|view| view.granularity == Granularity::Chip);
    assert_eq!(eligibility(&unsupported, &mesh1), Eligibility::Rejected(CapacityIssue::Unsupported));
    let mut unenforced = state.clone();
    unenforced.offers.get_mut("tt-host").unwrap().views[2].is_grouping_enforced = false;
    assert_eq!(eligibility(&unenforced, &mesh1), Eligibility::Rejected(CapacityIssue::Unsupported));
    assert_eq!(
        eligibility(&state, &request("mesh-1", "partial", &["chip-mmio"])),
        Eligibility::Rejected(CapacityIssue::Unsupported)
    );
}

// r[verify molten.compute_capacity.backing_pools]
#[test]
fn compute_capacity_observed_free_debits_only_new_allocations() {
    let mut state = two_hosts();
    let host = state.offers.get_mut("host-b").unwrap();
    host.pools.retain(|pool| pool.id == "shared-memory");
    host.views[0].backing_pools = vec!["shared-memory".into()];
    host.pools[0].observed_free = Some(70); // 30 bytes consumed outside the allocator.
    host.pools[0].is_observation_debited = true;
    let mut first = input("host-b", "first", "shared-memory", 50);
    first.claims.pop();
    state = reserved(&state, &first);
    let mut second = input("host-b", "second", "shared-memory", 21);
    second.claims.pop();
    assert_eq!(eligibility(&state, &second), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
    second.claims[0].quantity = 20;
    state = reserved(&state, &second);
    assert_eq!(charged(&state, "host-b", "shared-memory"), Some(70));
}

// r[verify molten.compute_capacity.offers]
// r[verify molten.compute_capacity.fixtures]
#[test]
fn compute_capacity_rejects_unbounded_and_ambiguous_offers() {
    let mut unbounded = offer("host", false);
    unbounded.views[0].device_id = "x".repeat(MAX_TEXT + 1);
    assert_eq!(validate_offer(&unbounded), Err(CapacityIssue::InvalidInput));
    let mut duplicate = offer("host", false);
    duplicate.pools.push(duplicate.pools[0].clone());
    assert_eq!(validate_offer(&duplicate), Err(CapacityIssue::InvalidInput));
    let mut impossible = offer("host", false);
    impossible.pools[0].observed_used = Some(80);
    impossible.pools[0].observed_free = Some(30);
    assert_eq!(validate_offer(&impossible), Err(CapacityIssue::InvalidInput));
    let state = observe(&CapacityState::default(), offer("host", false)).unwrap();
    let mut stale = offer("host", false);
    stale.revision = 1;
    assert_eq!(observe(&state, stale), Err(CapacityIssue::Stale));
}

// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.uncertainty_release]
#[test]
fn compute_capacity_replacement_cannot_replay_or_release_old_claim() {
    let state = reserved(&two_hosts(), &input("host-a", "op", "device-memory", 30));
    let mut changed = input("host-a", "op", "device-memory", 31);
    assert_eq!(reserve(&state, &changed).decision, ReservationOutcome::Rejected(CapacityIssue::WrongOwner));
    changed.claims[0].quantity = 30;
    changed.owner_generation = 2;
    assert_eq!(reserve(&state, &changed).decision, ReservationOutcome::Rejected(CapacityIssue::WrongOwner));
    let changed_owner = OwnerInput {
        allocation_id: "op".into(),
        consumer_id: "consumer-op".into(),
        owner_generation: 2,
    };
    let outcome = release(&state, &ReleaseInput {
        owner: changed_owner,
        backend_evidence: Some(evidence("op")),
    });
    assert_eq!(outcome.issue, Some(CapacityIssue::WrongOwner));
    assert_eq!(outcome.state, state);
    let mut next_offer = state.offers["host-a"].clone();
    next_offer.revision = 2;
    next_offer.host_incarnation = "host-2".into();
    next_offer.device_incarnation = "device-2".into();
    next_offer.observed_at = 21;
    let replaced = observe(&state, next_offer).unwrap();
    let mut successor = input("host-a", "successor", "device-memory", 70);
    successor.host_incarnation = "host-2".into();
    successor.device_incarnation = "device-2".into();
    successor.offer_revision = 2;
    successor.now = 22;
    successor.owner_generation = 2;
    assert_eq!(eligibility(&replaced, &successor), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
}

// r[verify molten.compute_capacity.reservation_authority]
#[test]
fn compute_capacity_activation_requires_backend_evidence() {
    let state = reserved(&two_hosts(), &input("host-a", "op", "device-memory", 20));
    assert_eq!(state.allocations["op"].status, ReservationStatus::Intent);
    let proof = ActivationEvidence {
        allocation_id: "op".into(),
        owner_generation: 1,
        backend_ref: "backend-ack".into(),
        is_active: true,
        enforcement: EnforcementStrength::BackendAcknowledged,
    };
    let mut missing = proof.clone();
    missing.is_active = false;
    assert_eq!(
        acknowledge_activation(&state, &owner("op"), &missing).issue,
        Some(CapacityIssue::InsufficientEvidence)
    );
    let mut wrong = proof.clone();
    wrong.owner_generation = 2;
    assert_eq!(acknowledge_activation(&state, &owner("op"), &wrong).state, state);
    let mut wrong_owner = owner("op");
    wrong_owner.consumer_id = "different".into();
    assert_eq!(acknowledge_activation(&state, &wrong_owner, &proof).issue, Some(CapacityIssue::WrongOwner));
    let committed = acknowledge_activation(&state, &owner("op"), &proof);
    assert_eq!(committed.issue, None);
    assert_eq!(committed.state.allocations["op"].status, ReservationStatus::Active);
    assert_eq!(committed.state.allocations["op"].enforcement, EnforcementStrength::BackendAcknowledged);
    assert_eq!(committed.state.allocations["op"].activation_backend_ref.as_deref(), Some("backend-ack"));
    let replay = acknowledge_activation(&committed.state, &owner("op"), &proof);
    assert_eq!(replay.issue, None);
    assert!(replay.effects.is_empty());
    assert_eq!(replay.state, committed.state);
    for changed in [
        ActivationEvidence {
            backend_ref: "different-ack".into(),
            ..proof.clone()
        },
        ActivationEvidence {
            enforcement: EnforcementStrength::BackendFenced,
            ..proof.clone()
        },
        ActivationEvidence {
            is_active: false,
            ..proof.clone()
        },
        ActivationEvidence {
            owner_generation: 2,
            ..proof.clone()
        },
    ] {
        let denied = acknowledge_activation(&committed.state, &owner("op"), &changed);
        assert_eq!(denied.issue, Some(CapacityIssue::InsufficientEvidence));
        assert_eq!(denied.state, committed.state);
    }
    assert_eq!(charged(&committed.state, "host-a", "device-memory"), Some(20));
}

// r[verify molten.compute_capacity.backing_pools]
#[test]
fn compute_capacity_exhaustive_small_pool_pairs_never_oversubscribe() {
    for first_amount in 1..=10 {
        for second_amount in 1..=10 {
            let mut snapshot = offer("host-b", true);
            snapshot.pools.retain(|pool| pool.id == "shared-memory");
            snapshot.pools[0].total = 10;
            snapshot.pools[0].allocatable = 10;
            snapshot.pools[0].observed_free = Some(10);
            snapshot.views[0].backing_pools = vec!["shared-memory".into()];
            let initial = observe(&CapacityState::default(), snapshot).unwrap();
            let mut first = input("host-b", "first", "shared-memory", first_amount);
            first.claims.pop();
            let state = reserved(&initial, &first);
            let mut second = input("host-b", "second", "shared-memory", second_amount);
            second.claims.pop();
            let outcome = reserve(&state, &second);
            let fits = first_amount.checked_add(second_amount).is_some_and(|sum| sum <= 10);
            assert_eq!(matches!(outcome.decision, ReservationOutcome::Reserved { .. }), fits);
            assert!(charged(&outcome.state, "host-b", "shared-memory").unwrap() <= 10);
        }
    }
}

// r[verify molten.compute_capacity.backing_pools]
// r[verify molten.compute_capacity.offers]
#[test]
fn compute_capacity_delayed_debited_observation_uses_captured_charge_cursor() {
    let mut snapshot = offer("host-b", true);
    snapshot.pools.retain(|pool| pool.id == "shared-memory");
    snapshot.views[0].backing_pools = vec!["shared-memory".into()];
    let initial = observe(&CapacityState::default(), snapshot).unwrap();
    let mut first = input("host-b", "first", "shared-memory", 40);
    first.claims.pop();
    let state = reserved(&initial, &first); // Charge at tick 20, cursor 1.
    let mut late = state.offers["host-b"].clone();
    late.revision = 2;
    late.observed_at = 19; // Captured before that charge, delivered afterward.
    late.pools[0].is_observation_debited = true;
    late.observation_charge_cursor = Some(0);
    let delayed = observe(&state, late.clone()).unwrap();
    let mut second = input("host-b", "second", "shared-memory", 61);
    second.claims.pop();
    second.offer_revision = 2;
    assert_eq!(eligibility(&delayed, &second), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
    second.claims[0].quantity = 60;
    assert_eq!(eligibility(&delayed, &second), Eligibility::Eligible);
    let mut falsely_debited = late.clone();
    falsely_debited.observation_charge_cursor = Some(state.charge_cursor);
    assert_eq!(observe(&state, falsely_debited), Err(CapacityIssue::InsufficientEvidence));

    // A missing basis must never assume that any active charge was already observed.
    late.observation_charge_cursor = None;
    late.pools[0].observed_free = Some(60);
    let unknown = observe(&state, late.clone()).unwrap();
    second.claims[0].quantity = 21;
    assert_eq!(eligibility(&unknown, &second), Eligibility::Rejected(CapacityIssue::InsufficientCapacity));
    second.claims[0].quantity = 20;
    assert_eq!(eligibility(&unknown, &second), Eligibility::Eligible);
    late.observation_charge_cursor = Some(state.charge_cursor + 1);
    assert_eq!(observe(&state, late), Err(CapacityIssue::InsufficientEvidence));
}

// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.uncertainty_release]
#[test]
fn compute_capacity_released_records_have_bounded_retirement_horizon() {
    let mut state = two_hosts();
    for index in 0..(MAX_ITEMS + 2) {
        let id = format!("operation-{index}");
        let request = input("host-a", &id, "device-memory", 1);
        state = reserved(&state, &request);
        let released = release(&state, &ReleaseInput {
            owner: owner(&id),
            backend_evidence: Some(evidence(&id)),
        });
        assert_eq!(released.issue, None);
        state = released.state;
        assert!(state.allocations.is_empty());
        assert!(state.retired_allocations.len() <= MAX_ITEMS);
    }
    assert_eq!(state.retired_allocations.len(), MAX_ITEMS);
    assert_eq!(state.retired_allocations.front().unwrap().id, "operation-2");
    let latest = "operation-129";
    assert_eq!(
        reserve(&state, &input("host-a", latest, "device-memory", 1)).decision,
        ReservationOutcome::Rejected(CapacityIssue::WrongOwner)
    );
    let replay = release(&state, &ReleaseInput {
        owner: owner(latest),
        backend_evidence: None,
    });
    assert_eq!(replay.issue, None);
    assert!(replay.effects.is_empty());
    assert_eq!(replay.state, state);
    assert_eq!(lifecycle(&state, &owner(latest), LifecycleEvent::Cancel).issue, None);
    let mut changed_owner = owner(latest);
    changed_owner.owner_generation = 2;
    assert_eq!(
        release(&state, &ReleaseInput {
            owner: changed_owner,
            backend_evidence: None
        })
        .issue,
        Some(CapacityIssue::WrongOwner)
    );
    assert_eq!(
        release(&state, &ReleaseInput {
            owner: owner("operation-0"),
            backend_evidence: None
        })
        .issue,
        Some(CapacityIssue::WrongOwner)
    );
    // Once the FIFO horizon expires, the opaque ID is unknown and can be reused.
    state = reserved(&state, &input("host-a", "operation-0", "device-memory", 1));
    assert_eq!(state.allocations.len(), 1);
}

// r[verify molten.compute_capacity.uncertainty_release]
#[test]
fn compute_capacity_abort_requires_definitive_no_effect_on_intent() {
    let state = reserved(&two_hosts(), &input("host-a", "op", "device-memory", 80));
    let uncertain = abort_unstarted_intent(&state, &owner("op"), false);
    assert_eq!(uncertain.issue, Some(CapacityIssue::UncertainRelease));
    assert_eq!(uncertain.state, state);
    let mut wrong = owner("op");
    wrong.consumer_id = "wrong".into();
    assert_eq!(abort_unstarted_intent(&state, &wrong, true).issue, Some(CapacityIssue::WrongOwner));
    let quarantined = lifecycle(&state, &owner("op"), LifecycleEvent::Timeout).state;
    assert_eq!(
        abort_unstarted_intent(&quarantined, &owner("op"), true).issue,
        Some(CapacityIssue::UncertainRelease)
    );
    let active = acknowledge_activation(&state, &owner("op"), &ActivationEvidence {
        allocation_id: "op".into(),
        owner_generation: 1,
        backend_ref: "started".into(),
        is_active: true,
        enforcement: EnforcementStrength::BackendAcknowledged,
    })
    .state;
    assert_eq!(abort_unstarted_intent(&active, &owner("op"), true).issue, Some(CapacityIssue::UncertainRelease));
    let aborted = abort_unstarted_intent(&state, &owner("op"), true);
    assert_eq!(aborted.issue, None);
    assert_eq!(aborted.effects, vec![EffectIntent::PersistRelease("op".into())]);
    assert_eq!(charged(&aborted.state, "host-a", "device-memory"), Some(0));
    assert!(aborted.state.allocations.is_empty());
    assert_eq!(abort_unstarted_intent(&aborted.state, &owner("op"), true).issue, None);
    assert_eq!(
        reserve(&aborted.state, &input("host-a", "op", "device-memory", 80)).decision,
        ReservationOutcome::Rejected(CapacityIssue::WrongOwner)
    );
    assert!(matches!(
        reserve(&aborted.state, &input("host-a", "fresh", "device-memory", 80)).decision,
        ReservationOutcome::Reserved { .. }
    ));
}
