use super::*;

#[derive(Debug, Default)]
struct RecordingPorts {
    executed: Vec<ReplicaEffectKind>,
    fail_on: Option<ReplicaEffectKind>,
}

impl RecordingPorts {
    fn record(&mut self, kind: ReplicaEffectKind) -> crate::error::Result<String> {
        self.executed.push(kind);
        if self.fail_on == Some(kind) {
            return Err(crate::error::MoltenError::invalid_harness(format!("injected {} failure", kind.as_str())));
        }
        Ok(super::tests::test_ref(kind.as_str()))
    }
}

impl ReplicaDurabilityEffects for RecordingPorts {
    fn persist_hard_state(&mut self, _term: u64, _voted_for: Option<&str>) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::PersistHardState)
    }

    fn persist_entries(
        &mut self,
        _truncate_from: Option<u64>,
        _entries: &[ReplicatedEntry],
    ) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::PersistEntries)
    }

    fn flush_log(&mut self, _through_index: u64) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::FlushLog)
    }

    fn persist_commit(&mut self, _through_index: u64) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::PersistCommit)
    }

    fn persist_snapshot(&mut self, _snapshot: &ReplicaSnapshot) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::PersistSnapshot)
    }
}

impl ReplicaTransportEffects for RecordingPorts {
    fn send<'a>(&'a mut self, _envelope: &'a ReplicaMessageEnvelope) -> ReplicaTransportFuture<'a> {
        Box::pin(async move { self.record(ReplicaEffectKind::Send) })
    }
}

impl ReplicaTimeEffects for RecordingPorts {
    fn arm_election_timer(&mut self, _timer_ref: &str) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::ArmElectionTimer)
    }

    fn arm_heartbeat_timer(&mut self) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::ArmHeartbeatTimer)
    }
}

impl ReplicaApplicationEffects for RecordingPorts {
    fn restore_snapshot(&mut self, _snapshot: &ReplicaSnapshot) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::RestoreApplicationSnapshot)
    }

    fn apply_committed(&mut self, _entries: &[ReplicatedEntry]) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::ApplyCommitted)
    }
}

impl ReplicaControlEffects for RecordingPorts {
    fn proposal_outcome(
        &mut self,
        _request_ref: &str,
        _disposition: ProposalDisposition,
        _committed_index: Option<u64>,
    ) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::ProposalOutcome)
    }

    fn read_outcome(
        &mut self,
        _request_ref: &str,
        _mode: crate::fabric_consistency::ConsistencyReadMode,
        _disposition: ReadDisposition,
        _observed_index: u64,
    ) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::ReadOutcome)
    }

    fn lifecycle_changed(&mut self, _lifecycle: ReplicaLifecycle) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::LifecycleChanged)
    }
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn effect_shell_executes_election_effects_in_declared_order() {
    let group = super::tests::active_group();
    let state = super::tests::started_state(&group, super::tests::NODE_A);
    let mut ports = RecordingPorts::default();
    let outcome = execute_replica_event(
        &state,
        ReplicaEvent::ElectionTimeout {
            timer_ref: state.active_election_timer_ref.clone(),
        },
        &mut ports,
    )
    .await;
    let ReplicaExecutionOutcome::Applied(applied) = outcome else {
        panic!("expected applied election effects");
    };

    let expected = vec![
        ReplicaEffectKind::PersistHardState,
        ReplicaEffectKind::Send,
        ReplicaEffectKind::Send,
        ReplicaEffectKind::ArmElectionTimer,
    ];
    assert_eq!(ports.executed, expected);
    assert_eq!(applied.observations.iter().map(|item| item.kind).collect::<Vec<_>>(), expected);
    assert_eq!(applied.next.role, ReplicaRole::Candidate);
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn effect_shell_stops_before_transport_when_log_flush_fails() {
    let (leader, _follower) = super::tests::elect_node_a();
    let mut ports = RecordingPorts {
        executed: Vec::new(),
        fail_on: Some(ReplicaEffectKind::FlushLog),
    };
    let outcome = execute_replica_event(
        &leader,
        ReplicaEvent::Propose {
            request_ref: super::tests::test_ref("shell-failure-request"),
            command_ref: super::tests::test_ref("shell-failure-command"),
            command_schema_ref: super::tests::test_ref("shell-failure-schema"),
        },
        &mut ports,
    )
    .await;
    let ReplicaExecutionOutcome::Failed(failed) = outcome else {
        panic!("expected failed proposal effects");
    };

    assert_eq!(failed.retained, leader);
    assert_eq!(failed.planned.log.len(), failed.retained.log.len() + 1);
    assert_eq!(failed.failed_kind, ReplicaEffectKind::FlushLog);
    assert_eq!(failed.completed.len(), 1);
    assert_eq!(ports.executed, vec![ReplicaEffectKind::PersistEntries, ReplicaEffectKind::FlushLog]);
    assert!(!ports.executed.contains(&ReplicaEffectKind::Send));
}

// r[verify molten.fabric_consistency.live_raft]
#[tokio::test]
async fn durable_commit_failure_prevents_application_and_state_publication() {
    let (leader, follower) = super::tests::elect_node_a();
    let proposal = apply_replica_event(&leader, ReplicaEvent::Propose {
        request_ref: super::tests::test_ref("commit-failure-request"),
        command_ref: super::tests::test_ref("commit-failure-command"),
        command_schema_ref: super::tests::test_ref("commit-failure-schema"),
    })
    .expect("proposal plan");
    let append = super::tests::sent_envelope_to(&proposal, &follower.node_id);
    let replicated =
        apply_replica_event(&follower, ReplicaEvent::Message { envelope: append }).expect("follower replication");
    let response = super::tests::sent_envelope_to(&replicated, &leader.node_id);
    let mut ports = RecordingPorts {
        executed: Vec::new(),
        fail_on: Some(ReplicaEffectKind::PersistCommit),
    };

    let outcome = execute_replica_event(&proposal.next, ReplicaEvent::Message { envelope: response }, &mut ports).await;
    let ReplicaExecutionOutcome::Failed(failed) = outcome else {
        panic!("expected durable commit failure");
    };
    assert_eq!(failed.failed_kind, ReplicaEffectKind::PersistCommit);
    assert_eq!(failed.retained.commit_index, INITIAL_COMMIT_INDEX);
    assert_eq!(ports.executed, vec![ReplicaEffectKind::PersistCommit]);
    assert!(!ports.executed.contains(&ReplicaEffectKind::ApplyCommitted));
}
