// r[verify molten.consensus.storage_fault_validation]
// r[verify molten.consensus.storage_fault_recovery]
#[tokio::test]
async fn uncertain_leader_flush_survives_clean_close_as_a_durable_recovery_fence() {
    let (leader, _) = super::tests::elect_node_a();
    let mut ports = UncertaintyPorts::new("live-raft-uncertain-clean-restart", &leader, true);
    let outcome = execute_replica_event(&leader, uncertainty_proposal(), &mut ports).await;
    let ReplicaExecutionOutcome::Failed(failed) = outcome else {
        panic!("a real Redb synchronization failure must reject the proposal");
    };
    assert_eq!(failed.failed_kind, ReplicaEffectKind::FlushLog);
    assert_eq!(failed.retained.lifecycle, ReplicaLifecycle::Stopped);
    assert!(matches!(failed.retained.rejoin, ReplicaRejoin::Recovering(_)));
    assert_eq!(ports.durable.adapter().health(),
        crate::fabric_durability::AdapterHealth::Quarantined(
            crate::fabric_durability::QuarantineReason::OutcomeUnknown));
    assert!(ports.sent.is_empty(), "no replication may escape the uncertain local flush");

    // Restore the medium and reconcile Redb once, without Raft startup effects.
    // Subsequent clean opens must rely on the already durable intent, not on
    // a transient repair flag or a newly persisted startup recovery marker.
    ports.disk.fail_syncs(false);
    let root = ports.root.clone();
    let disk = ports.disk.clone();
    drop(ports);
    let reconciled = uncertainty_port(&root, &disk).reconcile_repaired_storage()
        .expect("reconcile storage without executing Raft startup");
    drop(reconciled);
    for _ in 0..2 {
        let reopened = uncertainty_port(&root, &disk);
        assert!(!reopened.adapter().is_repair_observed(), "exercise the clean-open safety gap");
        let group = super::tests::active_group();
        let recovery = reopened.plan_recovery(recovery_start_plan(&group,
            super::tests::started_state(&group, super::tests::NODE_A))).expect("recovery from durable intent");
        let state = recovery.start_plan.state;
        assert_eq!(state.current_term, leader.current_term);
        assert_eq!(state.voted_for, leader.voted_for);
        assert!(matches!(state.rejoin, ReplicaRejoin::Recovering(_)));
        let timeout = apply_replica_event(&state, ReplicaEvent::ElectionTimeout {
            timer_ref: state.active_election_timer_ref.clone(),
        }).expect("recovering node queries peers instead of campaigning");
        assert_eq!(timeout.next.role, ReplicaRole::Follower);
        assert!(!timeout.effects.iter().any(|effect| matches!(effect,
            ReplicaEffect::Send { envelope: ReplicaMessageEnvelope { message: Message::RequestVote { .. }, .. } })));
        let denied = apply_replica_event(&state, ReplicaEvent::Message {
            envelope: ReplicaMessageEnvelope {
                group_binding_ref: state.profile.group_binding_ref.clone(),
                service_generation: state.profile.service_generation,
                from: super::tests::NODE_B.to_string(),
                to: state.node_id.clone(),
                message: Message::RequestVote {
                    term: state.current_term + 1,
                    candidate_id: super::tests::NODE_B.to_string(),
                    last_log_index: state.log.last().map_or(0, |entry| entry.index),
                    last_log_term: state.log.last().map_or(0, |entry| entry.term),
                    config_epoch: state.membership.config_epoch,
                    fencing_epoch: state.profile.fencing_epoch,
                },
            },
        }).expect("higher-term vote request while recovering");
        assert!(matches!(super::tests::sent_envelope_to(&denied, super::tests::NODE_B).message,
            Message::VoteResponse { granted: false, .. }));
        drop(reopened);
    }
}

// r[verify molten.consensus.storage_fault_recovery]
#[tokio::test]
async fn successful_leader_batches_clear_the_durable_intent_only_after_application_and_retain_vote() {
    let (leader, follower) = super::tests::elect_node_a();
    let mut ports = UncertaintyPorts::new("live-raft-successful-intent-restart", &leader, false);
    let ReplicaExecutionOutcome::Applied(proposed) =
        execute_replica_event(&leader, uncertainty_proposal(), &mut ports).await else {
            panic!("healthy proposal must finish");
        };
    let restart = ports.recovery_state(&ports.disk.power_loss_image());
    assert_eq!(restart.rejoin, ReplicaRejoin::Voter);
    assert_eq!(restart.voted_for, leader.voted_for);
    assert_eq!(restart.log, proposed.next.log);
    let append = ports.sent.iter().find(|envelope| envelope.to == follower.node_id)
        .expect("proposal replication").clone();
    let replicated = apply_replica_event(&follower, ReplicaEvent::Message { envelope: append })
        .expect("follower replication");
    let response = super::tests::sent_envelope_to(&replicated, &leader.node_id);
    let ReplicaExecutionOutcome::Applied(committed) = execute_replica_event(&proposed.next,
        ReplicaEvent::Message { envelope: response }, &mut ports).await else {
            panic!("healthy commit and application must finish");
        };
    assert_eq!(ports.applied, committed.next.log);
    assert_eq!(committed.next.commit_index, ENTRY_INDEX);
    let disk = ports.disk.clone();
    let root = ports.root.clone();
    drop(ports);
    let reopened = uncertainty_port(&root, &disk);
    assert!(!reopened.adapter().is_repair_observed());
    let group = super::tests::active_group();
    let recovered = reopened.plan_recovery(recovery_start_plan(&group,
        super::tests::started_state(&group, super::tests::NODE_A))).expect("successful batch restart");
    assert_eq!(recovered.start_plan.state.rejoin, ReplicaRejoin::Voter);
    assert_eq!(recovered.start_plan.state.current_term, leader.current_term);
    assert_eq!(recovered.start_plan.state.voted_for, leader.voted_for);
    assert_eq!(recovered.durable_commit_index, committed.next.commit_index);
    assert_eq!(recovered.start_plan.state.log, committed.next.log);
}

fn uncertainty_proposal() -> ReplicaEvent {
    ReplicaEvent::Propose {
        request_ref: super::tests::test_ref("uncertainty-request"),
        command_ref: super::tests::test_ref("uncertainty-command"),
        command_schema_ref: super::tests::test_ref("uncertainty-schema"),
    }
}

fn uncertainty_port(root: &std::path::Path, disk: &crate::fabric_durability::tests::FaultDisk) -> RedbReplicaPort {
    let adapter = crate::fabric_durability::RedbDurableStateAdapter::open_with(root,
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
        crate::fabric_durability::tests::descriptor(), disk.source(),
        crate::fabric_durability::PublicationSource::Linux).expect("Redb over the fault medium");
    RedbReplicaPort::new(adapter, super::tests::test_ref("uncertainty-log"),
        super::tests::test_ref("uncertainty-snapshots")).expect("replica durability")
}

struct UncertaintyPorts {
    durable: RedbReplicaPort,
    disk: crate::fabric_durability::tests::FaultDisk,
    root: crate::test_support::ProcessWorkspace,
    fail_flush: bool,
    sent: Vec<ReplicaMessageEnvelope>,
    applied: Vec<ReplicatedEntry>,
}

impl UncertaintyPorts {
    fn new(label: &str, leader: &ReplicaState, fail_flush: bool) -> Self {
        let root = crate::test_support::process_workspace(label).expect("workspace");
        let disk = crate::fabric_durability::tests::FaultDisk::default();
        let mut durable = uncertainty_port(&root, &disk);
        durable.persist_hard_state(leader.current_term, leader.voted_for.as_deref(), false)
            .expect("healthy leader's acknowledged election");
        Self { durable, disk, root, fail_flush, sent: Vec::new(), applied: Vec::new() }
    }

    fn recovery_state(&self, disk: &crate::fabric_durability::tests::FaultDisk) -> ReplicaState {
        let group = super::tests::active_group();
        let reopened = uncertainty_port(&self.root, disk).reconcile_repaired_storage()
            .expect("read back the power-loss image after explicit storage reconciliation");
        // Inspect actual durable records, independently of the transient repair
        // flag that would itself force recovery in the live startup wrapper.
        plan_replica(recovery_start_plan(&group,
            super::tests::started_state(&group, super::tests::NODE_A)),
            &reopened.adapter().state().durable_log, None, None)
            .expect("replay the actual durable recovery marker").start_plan.state
    }

    fn assert_durable_intent(&self) {
        let state = self.recovery_state(&self.disk.power_loss_image());
        assert!(matches!(state.rejoin, ReplicaRejoin::Recovering(_)),
            "data and application effects must run behind a machine-loss-durable recovery fence");
        assert_eq!(state.voted_for.as_deref(), Some(super::tests::NODE_A));
    }
}

impl ReplicaDurabilityEffects for UncertaintyPorts {
    fn persist_hard_state(&mut self, term: u64, vote: Option<&str>, required: bool) -> crate::error::Result<String> {
        self.durable.persist_hard_state(term, vote, required)
    }

    fn persist_entries(&mut self, truncate: Option<u64>, entries: &[ReplicatedEntry]) -> crate::error::Result<String> {
        self.assert_durable_intent();
        // The fault is armed only after the executor has acknowledged its
        // intent, not during election hard state or the intent transaction.
        self.disk.fail_syncs(self.fail_flush);
        self.durable.persist_entries(truncate, entries)
    }

    fn flush_log(&mut self, index: u64) -> crate::error::Result<String> {
        self.durable.flush_log(index)
    }

    fn persist_commit(&mut self, index: u64) -> crate::error::Result<String> {
        self.assert_durable_intent();
        self.durable.persist_commit(index)
    }

    fn persist_snapshot(&mut self, snapshot: &ReplicaSnapshot, is_suffix_retained: bool) -> crate::error::Result<String> {
        self.durable.persist_snapshot(snapshot, is_suffix_retained)
    }
}

impl ReplicaTransportEffects for UncertaintyPorts {
    fn send<'a>(&'a mut self, envelope: &'a ReplicaMessageEnvelope) -> ReplicaTransportFuture<'a> {
        self.sent.push(envelope.clone());
        Box::pin(async { Ok(super::tests::test_ref("uncertainty-send")) })
    }
}

impl ReplicaTimeEffects for UncertaintyPorts {
    fn arm_election_timer(&mut self, _: &str) -> crate::error::Result<String> {
        Ok(super::tests::test_ref("uncertainty-election-timer"))
    }
    fn arm_heartbeat_timer(&mut self) -> crate::error::Result<String> {
        Ok(super::tests::test_ref("uncertainty-heartbeat-timer"))
    }
}

impl ReplicaApplicationEffects for UncertaintyPorts {
    fn restore_snapshot(&mut self, _: &ReplicaSnapshot) -> crate::error::Result<String> {
        Ok(super::tests::test_ref("uncertainty-restore"))
    }
    fn apply_committed(&mut self, entries: &[ReplicatedEntry]) -> crate::error::Result<String> {
        self.assert_durable_intent();
        self.applied.extend_from_slice(entries);
        Ok(super::tests::test_ref("uncertainty-apply"))
    }
}

impl ReplicaControlEffects for UncertaintyPorts {
    fn proposal_outcome(&mut self, _: &str, _: ProposalDisposition, _: Option<u64>) -> crate::error::Result<String> {
        Ok(super::tests::test_ref("uncertainty-proposal-outcome"))
    }
    fn read_outcome(&mut self, _: &str, _: crate::fabric_consistency::ReadMode, _: ReadDisposition, _: u64)
        -> crate::error::Result<String> {
        Ok(super::tests::test_ref("uncertainty-read-outcome"))
    }
    fn lifecycle_changed(&mut self, _: ReplicaLifecycle) -> crate::error::Result<String> {
        Ok(super::tests::test_ref("uncertainty-lifecycle"))
    }
}
