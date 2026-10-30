use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaIngressExecution {
    pub transport_evidence: crate::fabric_transport::FrameEvidence,
    pub execution: ReplicaExecutionOutcome,
}

pub struct ScopedLiveReplica<P: BoundLiveReplicaEffectPorts> {
    state: ReplicaState,
    ports: P,
    inbox: tokio::sync::mpsc::Receiver<ReplicaEvent>,
    startup_observations: Vec<ReplicaEffectObservation>,
    evidence: ReplicaLedger,
    production_admitted: bool,
}

impl<P: BoundLiveReplicaEffectPorts> ScopedLiveReplica<P> {
    pub async fn start(
        plan: ReplicaStartPlan,
        mut ports: P,
        inbox: tokio::sync::mpsc::Receiver<ReplicaEvent>,
    ) -> crate::error::Result<Self> {
        ports.validate_start(&plan)?;
        let mut evidence = ReplicaLedger::new(&plan)?;
        let startup = execute_scoped_replica_start(&plan, &mut ports).await;
        let (state, startup_observations) = match startup {
            ReplicaExecutionOutcome::Applied(executed) => {
                evidence.observe_transport(&executed.next, &executed.observations, &executed.transport_failures)?;
                (executed.next, executed.observations)
            }
            ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
                return Err(crate::error::Failure::invalid_harness(format!("live Raft startup denied: {diagnostic}")));
            }
            ReplicaExecutionOutcome::Failed(failed) => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "live Raft startup effect {} failed: {}",
                    failed.failed_kind.as_str(),
                    failed.diagnostic
                )));
            }
        };
        Ok(Self {
            state,
            ports,
            inbox,
            startup_observations,
            evidence,
            production_admitted: plan.production_admitted,
        })
    }

    pub const fn state(&self) -> &ReplicaState {
        &self.state
    }

    pub const fn ports(&self) -> &P {
        &self.ports
    }

    pub fn ports_mut(&mut self) -> &mut P {
        &mut self.ports
    }

    pub fn startup_observations(&self) -> &[ReplicaEffectObservation] {
        &self.startup_observations
    }

    pub const fn production_admitted(&self) -> bool {
        self.production_admitted
    }

    pub const fn evidence(&self) -> &ReplicaLedger {
        &self.evidence
    }

    pub fn aggregate_health_evidence(&self) -> crate::error::Result<ReplicaAggregateHealth> {
        self.evidence.aggregate_health(&self.state, self.production_admitted)
    }

    pub async fn handle_event(&mut self, event: ReplicaEvent) -> ReplicaExecutionOutcome {
        let before = self.state.clone();
        let evidence_event = event.clone();
        let outcome = execute_replica_event(&self.state, event, &mut self.ports).await;
        match &outcome {
            ReplicaExecutionOutcome::Applied(executed) => self.state.clone_from(&executed.next),
            ReplicaExecutionOutcome::Failed(failed) => self.state.clone_from(&failed.retained),
            ReplicaExecutionOutcome::Denied { .. } => {}
        }
        if let Err(error) = self.evidence.observe(&before, &evidence_event, &outcome) {
            self.evidence.note_internal_error(error.to_string());
        }
        outcome
    }

    pub async fn run_next(&mut self, timeout: std::time::Duration) -> crate::error::Result<ReplicaExecutionOutcome> {
        if timeout.is_zero() {
            return Err(crate::error::Failure::invalid_harness("live Raft inbox timeout must be positive"));
        }
        let event = tokio::time::timeout(timeout, self.inbox.recv())
            .await
            .map_err(|_| crate::error::Failure::invalid_harness("live Raft inbox receive timed out"))?
            .ok_or_else(|| crate::error::Failure::invalid_harness("live Raft inbox closed"))?;
        Ok(self.handle_event(event).await)
    }

    pub async fn accept_one(
        &mut self,
        listener: &mut crate::fabric_transport::Listener,
        session_ref: &str,
        timeout: std::time::Duration,
    ) -> crate::error::Result<ReplicaIngressExecution> {
        let received = receive_replica_event(listener, session_ref, timeout).await?;
        let execution = self.handle_event(received.event).await;
        Ok(ReplicaIngressExecution {
            transport_evidence: received.transport_evidence,
            execution,
        })
    }
}
