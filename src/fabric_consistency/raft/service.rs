use std::time::Duration;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_transport::CrossProcessFrameEvidence;
use crate::fabric_transport::IrohCrossProcessListener;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaIngressExecution {
    pub transport_evidence: CrossProcessFrameEvidence,
    pub execution: ReplicaExecutionOutcome,
}

pub struct ScopedLiveReplicaService<P: BoundLiveReplicaEffectPorts> {
    state: ReplicaState,
    ports: P,
    inbox: tokio::sync::mpsc::UnboundedReceiver<ReplicaEvent>,
    startup_observations: Vec<ReplicaEffectObservation>,
    production_admitted: bool,
}

impl<P: BoundLiveReplicaEffectPorts> ScopedLiveReplicaService<P> {
    pub async fn start(
        plan: ReplicaStartPlan,
        mut ports: P,
        inbox: tokio::sync::mpsc::UnboundedReceiver<ReplicaEvent>,
    ) -> Result<Self> {
        ports.validate_start(&plan)?;
        let startup = execute_scoped_replica_start(&plan, &mut ports).await;
        let (state, startup_observations) = match startup {
            ReplicaExecutionOutcome::Applied(executed) => (executed.next, executed.observations),
            ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
                return Err(MoltenError::invalid_harness(format!("live Raft startup denied: {diagnostic}")));
            }
            ReplicaExecutionOutcome::Failed(failed) => {
                return Err(MoltenError::invalid_harness(format!(
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

    pub async fn handle_event(&mut self, event: ReplicaEvent) -> ReplicaExecutionOutcome {
        let outcome = execute_replica_event(&self.state, event, &mut self.ports).await;
        if let ReplicaExecutionOutcome::Applied(executed) = &outcome {
            self.state.clone_from(&executed.next);
        }
        outcome
    }

    pub async fn run_next(&mut self, timeout: Duration) -> Result<ReplicaExecutionOutcome> {
        if timeout.is_zero() {
            return Err(MoltenError::invalid_harness("live Raft inbox timeout must be positive"));
        }
        let event = tokio::time::timeout(timeout, self.inbox.recv())
            .await
            .map_err(|_| MoltenError::invalid_harness("live Raft inbox receive timed out"))?
            .ok_or_else(|| MoltenError::invalid_harness("live Raft inbox closed"))?;
        Ok(self.handle_event(event).await)
    }

    pub async fn accept_one(
        &mut self,
        listener: &mut IrohCrossProcessListener,
        session_ref: &str,
        timeout: Duration,
    ) -> Result<ReplicaIngressExecution> {
        let received = receive_replica_event(listener, session_ref, timeout).await?;
        let execution = self.handle_event(received.event).await;
        Ok(ReplicaIngressExecution {
            transport_evidence: received.transport_evidence,
            execution,
        })
    }
}
