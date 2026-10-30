// r[verify molten.consensus.repair_admission]
// r[verify molten.consensus.storage_fault_validation]
#[test]
fn a_locally_repaired_replica_store_reports_no_rejoin_role() {
    let root = crate::test_support::process_workspace("live-raft-repaired-store").expect("workspace");
    let disk = crate::fabric_durability::tests::FaultDisk::default();
    let open = |disk: &crate::fabric_durability::tests::FaultDisk| {
        crate::fabric_durability::RedbDurableStateAdapter::open_with(
            &root,
            crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
            crate::fabric_durability::tests::descriptor(),
            disk.source(),
            crate::fabric_durability::PublicationSource::Linux,
        )
        .expect("Redb adapter over the simulated medium")
    };
    let mut port = RedbReplicaPort::new(
        open(&disk),
        super::tests::test_ref("repaired-durable-log"),
        super::tests::test_ref("repaired-snapshot-store"),
    )
    .expect("replica durability port");
    port.persist_hard_state(TERM, Some("node-a"), false).expect("durable vote");
    assert_eq!(port.storage_admission().role, Some(molten_core::consensus_recovery::RejoinRole::Voter));

    let repaired_disk = disk.power_loss_image();
    let repaired = RedbReplicaPort::new(
        open(&repaired_disk),
        super::tests::test_ref("repaired-durable-log"),
        super::tests::test_ref("repaired-snapshot-store"),
    )
    .expect("replica durability port over the repaired store");
    let admission = repaired.storage_admission();
    assert_eq!(admission.role, None, "local repair alone admits no role");
    assert!(admission.blockers.contains(&molten_core::consensus_recovery::AdmissionBlocker::StorageNotServing));
    assert!(
        admission
            .blockers
            .contains(&molten_core::consensus_recovery::AdmissionBlocker::ProtocolNotReconciled)
    );

    let group = super::tests::active_group();
    persist_repaired_startup(repaired, &group);
    for _ in 0..2 {
        let mut reopened = RedbReplicaPort::new(open(&repaired_disk),
            super::tests::test_ref("repaired-durable-log"),
            super::tests::test_ref("repaired-snapshot-store")).expect("cleanly reopened repaired store");
        assert!(!reopened.adapter().is_repair_observed(), "previous repair flag is intentionally not durable");
        let recovered = reopened.plan_recovery(recovery_start_plan(&group,
            super::tests::started_state(&group, super::tests::NODE_A))).expect("durable recovery marker");
        assert_eq!(recovered.start_plan.state.current_term, TERM);
        assert!(matches!(recovered.start_plan.state.rejoin, ReplicaRejoin::Recovering(_)));
        persist_recovery_effects(&mut reopened, &recovered.start_plan.initial_effects);
    }
}

fn persist_repaired_startup(repaired: RedbReplicaPort, group: &crate::fabric_consistency::Group) {
    let mut reconciled = repaired.reconcile_repaired_storage().expect("explicit storage reconciliation");
    assert_eq!(reconciled.adapter().health(), crate::fabric_durability::AdapterHealth::Serving);
    assert!(reconciled.adapter().is_repair_observed(), "the repair stays recorded");
    assert_eq!(reconciled.storage_admission().role, None,
        "storage reconciliation is not protocol recovery: the store stays suspect for voting");
    let state = super::tests::started_state(group, super::tests::NODE_A);
    let recovery = reconciled.plan_recovery(recovery_start_plan(group, state)).expect("readable repaired startup");
    assert!(matches!(recovery.start_plan.state.rejoin, ReplicaRejoin::Recovering(_)));
    let timeout = apply_replica_event(&recovery.start_plan.state, ReplicaEvent::ElectionTimeout {
        timer_ref: recovery.start_plan.state.active_election_timer_ref.clone(),
    }).expect("repaired startup queries peers");
    assert_eq!(timeout.next.role, ReplicaRole::Follower);
    assert!(timeout.effects.iter().all(|effect| !matches!(effect,
        ReplicaEffect::Send { envelope: ReplicaMessageEnvelope { message: Message::RequestVote { .. }, .. } }
    )));
    assert!(matches!(recovery.start_plan.initial_effects.first(), Some(ReplicaEffect::PersistHardState {
        term: TERM, is_recovery_required: true, ..
    })));
    persist_recovery_effects(&mut reconciled, &recovery.start_plan.initial_effects);
}

// r[verify molten.consensus.repair_admission]
// r[verify molten.consensus.storage_fault_validation]
#[test]
fn recovery_of_entries_without_hard_state_fences_the_term_before_querying() {
    let root = crate::test_support::process_workspace("live-raft-lost-hard-state").expect("workspace");
    let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
        &root,
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
        crate::fabric_durability::tests::descriptor(),
    ).expect("Redb adapter");
    let mut port = RedbReplicaPort::new(adapter,
        super::tests::test_ref("lost-state-log"), super::tests::test_ref("lost-state-snapshots"),
    ).expect("replica durability");
    port.persist_entries(None, &[entry(ENTRY_INDEX, "lost-state-entry")]).expect("entry survived");
    port.flush_log(ENTRY_INDEX).expect("entry durable");
    port.persist_commit(ENTRY_INDEX).expect("commit survived");
    let group = super::tests::active_group();
    let state = super::tests::started_state(&group, super::tests::NODE_A);
    let recovery = port.plan_recovery(recovery_start_plan(&group, state)).expect("lost hard state recovery");
    assert_eq!(recovery.start_plan.state.current_term, TERM);
    assert!(matches!(recovery.start_plan.state.rejoin, ReplicaRejoin::Recovering(_)));
    assert!(matches!(recovery.start_plan.initial_effects.first(),
        Some(ReplicaEffect::PersistHardState { term: TERM, voted_for: Some(vote), is_recovery_required: true }) if vote == super::tests::NODE_A
    ));
}

// r[verify molten.consensus.repair_admission]
// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn recovery_required_survives_clean_restarts_and_higher_term_vote_requests() {
    let root = crate::test_support::process_workspace("live-raft-durable-recovery-fence").expect("workspace");
    let open = || {
        let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
            &root,
            crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
            crate::fabric_durability::tests::descriptor(),
        ).expect("Redb adapter");
        RedbReplicaPort::new(adapter, super::tests::test_ref("restart-fence-log"),
            super::tests::test_ref("restart-fence-snapshots")).expect("durability port")
    };
    let group = super::tests::active_group();
    let startup = || recovery_start_plan(&group, super::tests::started_state(&group, super::tests::NODE_A));
    let mut port = open();
    port.persist_entries(None, &[entry(ENTRY_INDEX, "restart-fence-entry")]).expect("surviving entry");
    port.flush_log(ENTRY_INDEX).expect("entry durable without hard state");
    let first = port.plan_recovery(startup()).expect("lost hard-state startup");
    persist_recovery_effects(&mut port, &first.start_plan.initial_effects);
    drop(port);

    for term in [2, 3, 4] {
        let mut port = open();
        assert!(!port.adapter().is_repair_observed(), "clean reopen must rely on the durable marker");
        let recovery = port.plan_recovery(startup()).expect("restart before peer admission");
        let state = recovery.start_plan.state;
        assert!(matches!(state.rejoin, ReplicaRejoin::Recovering(_)));
        assert_eq!(state.current_term, term - 1);
        assert_eq!(state.log, vec![entry(ENTRY_INDEX, "restart-fence-entry")]);
        persist_recovery_effects(&mut port, &recovery.start_plan.initial_effects);
        persist_denied_higher_term_vote(&mut port, &state, term);
        drop(port);
    }
    let port = open();
    let final_restart = port.plan_recovery(startup()).expect("final restart");
    assert_eq!(final_restart.start_plan.state.current_term, 4);
    assert!(matches!(final_restart.start_plan.state.rejoin, ReplicaRejoin::Recovering(_)));
}

fn persist_denied_higher_term_vote(port: &mut RedbReplicaPort, state: &ReplicaState, term: u64) {
    let vote = apply_replica_event(state, ReplicaEvent::Message {
        envelope: ReplicaMessageEnvelope {
            group_binding_ref: state.profile.group_binding_ref.clone(),
            service_generation: state.profile.service_generation,
            from: super::tests::NODE_B.to_string(),
            to: state.node_id.clone(),
            message: Message::RequestVote {
                term,
                candidate_id: super::tests::NODE_B.to_string(),
                last_log_index: ENTRY_INDEX,
                last_log_term: TERM,
                config_epoch: state.membership.config_epoch,
                fencing_epoch: state.profile.fencing_epoch,
            },
        },
    }).expect("higher-term vote remains fenced");
    assert_eq!(vote.next.current_term, term);
    assert!(matches!(vote.next.rejoin, ReplicaRejoin::Recovering(_)));
    assert!(matches!(super::tests::sent_envelope_to(&vote, super::tests::NODE_B).message,
        Message::VoteResponse { granted: false, .. }));
    persist_recovery_effects(port, &vote.effects);
    let timeout = apply_replica_event(&vote.next, ReplicaEvent::ElectionTimeout {
        timer_ref: vote.next.active_election_timer_ref.clone(),
    }).expect("recovery timeout");
    assert_eq!(timeout.next.role, ReplicaRole::Follower);
    assert!(timeout.effects.iter().any(|effect| matches!(effect,
        ReplicaEffect::Send { envelope: ReplicaMessageEnvelope { message: Message::RecoveryQuery { .. }, .. } })));
    assert!(!timeout.effects.iter().any(|effect| matches!(effect,
        ReplicaEffect::Send { envelope: ReplicaMessageEnvelope { message: Message::RequestVote { .. }, .. } })));
}

fn persist_recovery_effects(port: &mut RedbReplicaPort, effects: &[ReplicaEffect]) {
    for effect in effects {
        match effect {
            ReplicaEffect::PersistHardState { term, voted_for, is_recovery_required } => {
                port.persist_hard_state(*term, voted_for.as_deref(), *is_recovery_required).expect("persist recovery fence");
            }
            ReplicaEffect::PersistEntries { truncate_from, entries } => {
                port.persist_entries(*truncate_from, entries).expect("persist recovery entries");
            }
            ReplicaEffect::FlushLog { through_index } => { port.flush_log(*through_index).expect("flush recovery log"); }
            ReplicaEffect::PersistCommit { through_index } => { port.persist_commit(*through_index).expect("persist recovery commit"); }
            ReplicaEffect::PersistSnapshot { snapshot, is_suffix_retained } => {
                port.persist_snapshot(snapshot, *is_suffix_retained).expect("persist recovery snapshot");
            }
            _ => {}
        }
    }
}

// r[verify molten.consensus.repair_admission]
#[test]
fn peer_admission_durably_clears_the_recovery_marker_and_retains_the_self_vote() {
    let root = crate::test_support::process_workspace("live-raft-recovery-admission-restart").expect("workspace");
    let open = || {
        let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
            &root,
            crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
            crate::fabric_durability::tests::descriptor(),
        ).expect("Redb adapter");
        RedbReplicaPort::new(adapter, super::tests::test_ref("admission-restart-log"),
            super::tests::test_ref("admission-restart-snapshots")).expect("durability port")
    };
    let group = super::tests::active_group();
    let startup = || recovery_start_plan(&group, super::tests::started_state(&group, super::tests::NODE_A));
    let mut port = open();
    port.persist_hard_state(TERM, None, true).expect("durable recovery marker");
    let recovery = port.plan_recovery(startup()).expect("recover marked empty log");
    persist_recovery_effects(&mut port, &recovery.start_plan.initial_effects);
    let state = recovery.start_plan.state;
    let queries = apply_replica_event(&state, ReplicaEvent::ElectionTimeout {
        timer_ref: state.active_election_timer_ref.clone(),
    }).expect("query both peers");
    let mut state = queries.next.clone();
    let mut peers = std::collections::BTreeMap::from([
        (super::tests::NODE_B.to_string(), super::tests::started_state(&group, super::tests::NODE_B)),
        (super::tests::NODE_C.to_string(), super::tests::started_state(&group, super::tests::NODE_C)),
    ]);
    let mut pending = queries.effects.iter().filter_map(|effect| match effect {
        ReplicaEffect::Send { envelope } => Some(envelope.clone()),
        _ => None,
    }).collect::<std::collections::VecDeque<_>>();
    for _ in 0..32 {
        let Some(envelope) = pending.pop_front() else { break };
        let transition = if envelope.to == super::tests::NODE_A {
            let received = apply_replica_event(&state, ReplicaEvent::Message { envelope }).expect("receive peer evidence");
            persist_recovery_effects(&mut port, &received.effects);
            state = received.next.clone();
            received
        } else {
            let peer = peers.get_mut(&envelope.to).expect("known recovery peer");
            let report = apply_replica_event(peer, ReplicaEvent::Message { envelope }).expect("peer report");
            *peer = report.next.clone();
            report
        };
        pending.extend(transition.effects.into_iter().filter_map(|effect| {
            if let ReplicaEffect::Send { envelope } = effect { Some(envelope) } else { None }
        }));
    }
    assert!(matches!(state.rejoin, ReplicaRejoin::VoteFenced { through_term } if through_term == state.current_term));
    assert!(state.current_term > TERM, "admission must cross the durable higher-term barrier");
    assert_eq!(state.voted_for.as_deref(), Some(super::tests::NODE_A));
    drop(port);
    let port = open();
    let reopened = port.plan_recovery(startup()).expect("admitted restart").start_plan.state;
    assert_eq!(reopened.rejoin, ReplicaRejoin::Voter);
    assert_eq!(reopened.current_term, state.current_term);
    assert_eq!(reopened.voted_for.as_deref(), Some(super::tests::NODE_A));
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn legacy_hard_state_history_remains_readable_before_v2_recovery_markers() {
    let root = crate::test_support::process_workspace("live-raft-legacy-hard-state").expect("workspace");
    let mut adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
        &root,
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
        crate::fabric_durability::tests::descriptor(),
    ).expect("Redb adapter");
    let value = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("raft-hard-state-v1", vec![
        crate::preserves_rail::u64_value(TERM),
        crate::preserves_rail::record("some", vec![crate::preserves_rail::string(super::tests::NODE_B)]),
    ])).expect("legacy encoding");
    let descriptor = adapter.state().descriptor.clone();
    let request = crate::fabric_durability::AppendRequest {
        adapter_id: descriptor.adapter_id,
        namespace_id: descriptor.namespace_id,
        generation: descriptor.generation,
        expected_sequence: adapter.state().next_log_sequence().expect("legacy sequence"),
        value_ref: crate::preserves_rail::content_ref_from_bytes(&value),
        value,
        durability: crate::fabric_durability::DurabilityLevel::MachineLoss,
    };
    adapter.append(&request).expect("legacy durable history");
    let mut port = RedbReplicaPort::new(adapter, super::tests::test_ref("legacy-hard-state-log"),
        super::tests::test_ref("legacy-hard-state-snapshots")).expect("durability port");
    let group = super::tests::active_group();
    let startup = || recovery_start_plan(&group, super::tests::started_state(&group, super::tests::NODE_A));
    let legacy = port.plan_recovery(startup()).expect("legacy replay").start_plan.state;
    assert_eq!(legacy.rejoin, ReplicaRejoin::Voter);
    assert_eq!(legacy.current_term, TERM);
    assert_eq!(legacy.voted_for.as_deref(), Some(super::tests::NODE_B));
    port.persist_hard_state(TERM, Some(super::tests::NODE_B), true).expect("v2 durable marker");
    let marked = port.plan_recovery(startup()).expect("mixed version history").start_plan.state;
    assert!(matches!(marked.rejoin, ReplicaRejoin::Recovering(_)));
    assert_eq!(marked.current_term, TERM);
    assert_eq!(marked.voted_for.as_deref(), Some(super::tests::NODE_B));
}