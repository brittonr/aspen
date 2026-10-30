use super::*;

#[path = "loss.rs"]
mod loss;

#[derive(Debug, Default)]
pub(super) struct RecordingPorts {
    pub(super) executed: Vec<ReplicaEffectKind>,
    pub(super) fail_on: Option<ReplicaEffectKind>,
    pub(super) fail_at: Option<usize>,
    pub(super) hard_states: Vec<(u64, Option<String>, bool)>,
    pub(super) unavailable_peer: Option<String>,
    pub(super) sent: Vec<ReplicaMessageEnvelope>,
}

impl RecordingPorts {
    fn record(&mut self, kind: ReplicaEffectKind) -> crate::error::Result<String> {
        if matches!(
            kind,
            ReplicaEffectKind::PersistEntries
                | ReplicaEffectKind::FlushLog
                | ReplicaEffectKind::PersistCommit
                | ReplicaEffectKind::PersistSnapshot
                | ReplicaEffectKind::RestoreApplicationSnapshot
                | ReplicaEffectKind::ApplyCommitted
        ) {
            assert!(
                self.hard_states.last().is_some_and(|state| state.2),
                "data/application effects require durable intent"
            );
        }
        self.executed.push(kind);
        if self.fail_on == Some(kind) || self.fail_at == Some(self.executed.len() - 1) {
            return Err(crate::error::Failure::invalid_harness(format!("injected {} failure", kind.as_str())));
        }
        Ok(super::tests::test_ref(kind.as_str()))
    }
}

impl ReplicaDurabilityEffects for RecordingPorts {
    fn persist_hard_state(
        &mut self,
        term: u64,
        voted_for: Option<&str>,
        is_recovery_required: bool,
    ) -> crate::error::Result<String> {
        let reference = self.record(ReplicaEffectKind::PersistHardState)?;
        self.hard_states.push((term, voted_for.map(str::to_owned), is_recovery_required));
        Ok(reference)
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

    fn persist_snapshot(
        &mut self,
        _snapshot: &ReplicaSnapshot,
        _is_suffix_retained: bool,
    ) -> crate::error::Result<String> {
        self.record(ReplicaEffectKind::PersistSnapshot)
    }
}

impl ReplicaTransportEffects for RecordingPorts {
    fn send<'a>(&'a mut self, envelope: &'a ReplicaMessageEnvelope) -> ReplicaTransportFuture<'a> {
        Box::pin(async move {
            if self.unavailable_peer.as_deref() == Some(envelope.to.as_str()) {
                self.executed.push(ReplicaEffectKind::Send);
                return Err(crate::error::Failure::invalid_harness("peer unavailable"));
            }
            let reference = self.record(ReplicaEffectKind::Send)?;
            self.sent.push(envelope.clone());
            Ok(reference)
        })
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
        _mode: crate::fabric_consistency::ReadMode,
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
async fn election_persists_its_vote_before_sending_and_clears_intent_after_success() {
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

    let first_send = ports.executed.iter().position(|kind| *kind == ReplicaEffectKind::Send).expect("vote requests");
    assert!(ports.executed[..first_send].contains(&ReplicaEffectKind::PersistHardState));
    assert_eq!(
        ports.hard_states.first(),
        Some(&(applied.next.current_term, Some(applied.next.node_id.clone()), true))
    );
    assert_eq!(ports.hard_states.last(), Some(&(applied.next.current_term, applied.next.voted_for.clone(), false)));
    assert_eq!(applied.next.role, ReplicaRole::Candidate);
}

// r[verify molten.fabric_consistency.live_service_ports]
#[tokio::test]
async fn effect_shell_stops_before_transport_when_log_flush_fails() {
    let (leader, _follower) = super::tests::elect_node_a();
    let mut ports = RecordingPorts {
        executed: Vec::new(),
        fail_on: Some(ReplicaEffectKind::FlushLog),
        ..RecordingPorts::default()
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

    assert_eq!(failed.retained.lifecycle, ReplicaLifecycle::Stopped);
    assert_eq!(failed.retained.role, ReplicaRole::Follower);
    assert!(matches!(failed.retained.rejoin, ReplicaRejoin::Recovering(_)));
    assert!(
        apply_replica_event(&failed.retained, ReplicaEvent::ElectionTimeout {
            timer_ref: failed.retained.active_election_timer_ref.clone(),
        })
        .is_err(),
        "uncertain storage must be reopened before any further campaign"
    );
    assert_eq!(failed.planned.log.len(), failed.retained.log.len() + 1);
    assert_eq!(failed.failed_kind, ReplicaEffectKind::FlushLog);
    assert!(
        ports.hard_states.last().is_some_and(|state| state.2),
        "failed flush must leave durable recovery intent"
    );
    assert!(!failed.completed.iter().any(|item| item.kind == ReplicaEffectKind::FlushLog));
    assert!(!ports.executed.contains(&ReplicaEffectKind::Send));
}

// r[verify molten.fabric_consistency.live_raft]
#[tokio::test]
async fn commit_or_application_failure_prevents_state_publication_and_retains_intent() {
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
    for failed_kind in [ReplicaEffectKind::PersistCommit, ReplicaEffectKind::ApplyCommitted] {
        let mut ports = RecordingPorts {
            fail_on: Some(failed_kind),
            ..RecordingPorts::default()
        };
        let outcome = execute_replica_event(
            &proposal.next,
            ReplicaEvent::Message {
                envelope: response.clone(),
            },
            &mut ports,
        )
        .await;
        let ReplicaExecutionOutcome::Failed(failed) = outcome else {
            panic!("expected commit/application failure");
        };
        assert_eq!(failed.failed_kind, failed_kind);
        assert_eq!(failed.retained.commit_index, INITIAL_COMMIT_INDEX);
        assert_eq!(failed.retained.lifecycle, ReplicaLifecycle::Stopped);
        assert!(matches!(failed.retained.rejoin, ReplicaRejoin::Recovering(_)));
        assert!(
            ports.hard_states.last().is_some_and(|state| state.2),
            "uncertain commit/application must leave durable recovery intent"
        );
        if failed_kind == ReplicaEffectKind::PersistCommit {
            assert!(!ports.executed.contains(&ReplicaEffectKind::ApplyCommitted));
        }
    }
}
