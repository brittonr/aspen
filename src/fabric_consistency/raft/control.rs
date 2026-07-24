use super::*;
use crate::error::MoltenError;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaControlConfig {
    pub service_id: String,
    pub service_generation: u64,
    pub supervision_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaControlObservation {
    pub receipt_ref: String,
    pub kind: ReplicaControlObservationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicaControlObservationKind {
    Proposal {
        request_ref: String,
        disposition: ProposalDisposition,
        committed_index: Option<u64>,
    },
    Read {
        request_ref: String,
        mode: crate::fabric_consistency::ConsistencyReadMode,
        disposition: ReadDisposition,
        observed_index: u64,
    },
    Lifecycle {
        lifecycle: ReplicaLifecycle,
    },
}

pub struct ChannelReplicaControlPort {
    config: ReplicaControlConfig,
    sender: tokio::sync::mpsc::UnboundedSender<ReplicaControlObservation>,
}

impl ChannelReplicaControlPort {
    pub fn new(
        config: ReplicaControlConfig,
        sender: tokio::sync::mpsc::UnboundedSender<ReplicaControlObservation>,
    ) -> Result<Self> {
        validate_control_config(&config)?;
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

    fn publish(&self, kind: ReplicaControlObservationKind) -> Result<String> {
        let receipt_ref = control_receipt_ref(&self.config, &kind)?;
        self.sender
            .send(ReplicaControlObservation {
                receipt_ref: receipt_ref.clone(),
                kind,
            })
            .map_err(|_| MoltenError::invalid_harness("live Raft supervision receiver is unavailable"))?;
        Ok(receipt_ref)
    }
}

impl ReplicaControlEffects for ChannelReplicaControlPort {
    fn proposal_outcome(
        &mut self,
        request_ref: &str,
        disposition: ProposalDisposition,
        committed_index: Option<u64>,
    ) -> Result<String> {
        self.publish(ReplicaControlObservationKind::Proposal {
            request_ref: request_ref.to_string(),
            disposition,
            committed_index,
        })
    }

    fn read_outcome(
        &mut self,
        request_ref: &str,
        mode: crate::fabric_consistency::ConsistencyReadMode,
        disposition: ReadDisposition,
        observed_index: u64,
    ) -> Result<String> {
        self.publish(ReplicaControlObservationKind::Read {
            request_ref: request_ref.to_string(),
            mode,
            disposition,
            observed_index,
        })
    }

    fn lifecycle_changed(&mut self, lifecycle: ReplicaLifecycle) -> Result<String> {
        self.publish(ReplicaControlObservationKind::Lifecycle { lifecycle })
    }
}

fn validate_control_config(config: &ReplicaControlConfig) -> Result<()> {
    if config.service_id.is_empty()
        || !config
            .service_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(MoltenError::invalid_harness("live Raft supervision service id is empty or malformed"));
    }
    if config.service_generation == 0 {
        return Err(MoltenError::invalid_harness("live Raft supervision generation must be positive"));
    }
    crate::preserves_rail::validate_content_ref(&config.supervision_ref)
}

fn control_receipt_ref(config: &ReplicaControlConfig, kind: &ReplicaControlObservationKind) -> Result<String> {
    let outcome = match kind {
        ReplicaControlObservationKind::Proposal {
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
        ReplicaControlObservationKind::Read {
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
        ReplicaControlObservationKind::Lifecycle { lifecycle } => {
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
