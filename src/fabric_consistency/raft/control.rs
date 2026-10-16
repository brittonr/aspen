use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaConfig {
    pub service_id: String,
    pub service_generation: u64,
    pub supervision_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaObservation {
    pub receipt_ref: String,
    pub kind: ReplicaObservationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicaObservationKind {
    Proposal {
        request_ref: String,
        disposition: ProposalDisposition,
        committed_index: Option<u64>,
    },
    Read {
        request_ref: String,
        mode: crate::fabric_consistency::ReadMode,
        disposition: ReadDisposition,
        observed_index: u64,
    },
    Lifecycle {
        lifecycle: ReplicaLifecycle,
    },
}

pub struct ChannelReplicaPort {
    config: ReplicaConfig,
    sender: tokio::sync::mpsc::Sender<ReplicaObservation>,
}

impl ChannelReplicaPort {
    pub fn new(
        config: ReplicaConfig,
        sender: tokio::sync::mpsc::Sender<ReplicaObservation>,
    ) -> crate::error::Result<Self> {
        validate_config(&config)?;
        Ok(Self { config, sender })
    }

    pub fn supervision_ref(&self) -> &str {
        &self.config.supervision_ref
    }

    pub fn service_id(&self) -> &str {
        &self.config.service_id
    }

    pub const fn service_generation(&self) -> u64 {
        self.config.service_generation
    }

    fn publish(&self, kind: ReplicaObservationKind) -> crate::error::Result<String> {
        let receipt_ref = receipt_ref(&self.config, &kind)?;
        self.sender
            .try_send(ReplicaObservation {
                receipt_ref: receipt_ref.clone(),
                kind,
            })
            .map_err(|error| match error {
                tokio::sync::mpsc::error::TrySendError::Full(_) => {
                    crate::error::Failure::invalid_harness("live Raft supervision queue is full")
                }
                tokio::sync::mpsc::error::TrySendError::Closed(_) => {
                    crate::error::Failure::invalid_harness("live Raft supervision receiver is unavailable")
                }
            })?;
        Ok(receipt_ref)
    }
}

impl ReplicaControlEffects for ChannelReplicaPort {
    fn proposal_outcome(
        &mut self,
        request_ref: &str,
        disposition: ProposalDisposition,
        committed_index: Option<u64>,
    ) -> crate::error::Result<String> {
        self.publish(ReplicaObservationKind::Proposal {
            request_ref: request_ref.to_string(),
            disposition,
            committed_index,
        })
    }

    fn read_outcome(
        &mut self,
        request_ref: &str,
        mode: crate::fabric_consistency::ReadMode,
        disposition: ReadDisposition,
        observed_index: u64,
    ) -> crate::error::Result<String> {
        self.publish(ReplicaObservationKind::Read {
            request_ref: request_ref.to_string(),
            mode,
            disposition,
            observed_index,
        })
    }

    fn lifecycle_changed(&mut self, lifecycle: ReplicaLifecycle) -> crate::error::Result<String> {
        self.publish(ReplicaObservationKind::Lifecycle { lifecycle })
    }
}

fn validate_config(config: &ReplicaConfig) -> crate::error::Result<()> {
    if config.service_id.is_empty()
        || !config
            .service_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(crate::error::Failure::invalid_harness("live Raft supervision service id is empty or malformed"));
    }
    if config.service_generation == 0 {
        return Err(crate::error::Failure::invalid_harness("live Raft supervision generation must be positive"));
    }
    crate::preserves_rail::validate_content_ref(&config.supervision_ref)
}

fn receipt_ref(config: &ReplicaConfig, kind: &ReplicaObservationKind) -> crate::error::Result<String> {
    let outcome = match kind {
        ReplicaObservationKind::Proposal {
            request_ref,
            disposition,
            committed_index,
        } => {
            crate::preserves_rail::validate_content_ref(request_ref)?;
            crate::preserves_rail::record("proposal", vec![
                crate::preserves_rail::string(request_ref),
                crate::preserves_rail::string(disposition.as_str()),
                optional_index(*committed_index),
            ])
        }
        ReplicaObservationKind::Read {
            request_ref,
            mode,
            disposition,
            observed_index,
        } => {
            crate::preserves_rail::validate_content_ref(request_ref)?;
            crate::preserves_rail::record("read", vec![
                crate::preserves_rail::string(request_ref),
                crate::preserves_rail::string(mode.as_str()),
                crate::preserves_rail::string(disposition.as_str()),
                crate::preserves_rail::u64_value(*observed_index),
            ])
        }
        ReplicaObservationKind::Lifecycle { lifecycle } => {
            crate::preserves_rail::record("lifecycle", vec![crate::preserves_rail::string(lifecycle.as_str())])
        }
    };
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-control-observation-v1", vec![
        crate::preserves_rail::string(&config.service_id),
        crate::preserves_rail::u64_value(config.service_generation),
        crate::preserves_rail::string(&config.supervision_ref),
        outcome,
    ]))
}

fn optional_index(index: Option<u64>) -> preserves::IOValue {
    index.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
    )
}
