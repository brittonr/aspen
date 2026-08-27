use molten_core::content_replication::*;

use super::super::*;
use super::support::*;
use crate::error::Result;

pub struct Content {
    pub inventory: Inventory,
    pub verify_admitted: bool,
    pub cleanup_count: usize,
    pub events: Events,
}

impl ContentPort for Content {
    fn inventory(&mut self, _manifest: &Manifest) -> Result<Inventory> {
        self.events.borrow_mut().push("inventory");
        Ok(self.inventory.clone())
    }

    fn verify(&mut self, action: &Action, envelope: &TransferEnvelope) -> Result<VerificationObservation> {
        self.events.borrow_mut().push("verify");
        Ok(VerificationObservation {
            verification_ref: digest('1'),
            operation_id: action.operation_id.clone(),
            replica: Replica {
                content_ref: action.content_ref.clone(),
                peer_id: action.target_peer.clone(),
                fault_domain: action.fault_domain.clone(),
                generation: envelope.generation,
                membership_epoch: envelope.membership_epoch,
                placement_epoch: envelope.placement_epoch,
                present: self.verify_admitted,
                identity_verified: self.verify_admitted,
                pinned: true,
                protected: envelope.protected,
                manifest_ref: envelope.manifest_ref.clone(),
                cleanup_clearance_ref: None,
            },
            identity_verified: self.verify_admitted,
            authorization_admitted: self.verify_admitted,
        })
    }

    fn cleanup(&mut self, _action: &Action, _admission: &CleanupObservation) -> Result<String> {
        self.events.borrow_mut().push("cleanup");
        self.cleanup_count = self.cleanup_count.saturating_add(1);
        Ok(digest('2'))
    }
}

pub struct Transport {
    pub outcome: Option<OperationOutcome>,
    pub placement_epoch: u64,
    pub operation_mismatch: bool,
    pub calls: usize,
    pub events: Events,
}

impl TransportPort for Transport {
    fn fetch(&mut self, action: &Action) -> Result<TransferOutcome> {
        self.events.borrow_mut().push("transport");
        self.calls = self.calls.saturating_add(1);
        let reference = digest('3');
        match self.outcome.unwrap_or(OperationOutcome::Verified) {
            OperationOutcome::Verified | OperationOutcome::Planned => Ok(TransferOutcome::Received(TransferEnvelope {
                transport_verification_ref: reference.clone(),
                transfer_ref: reference,
                operation_id: if self.operation_mismatch {
                    digest('f')
                } else {
                    action.operation_id.clone()
                },
                content_ref: action.content_ref.clone(),
                manifest_ref: digest('8'),
                source_peer: action.source_peer.clone().unwrap_or_default(),
                target_peer: action.target_peer.clone(),
                generation: GENERATION,
                membership_epoch: MEMBERSHIP_EPOCH,
                placement_epoch: self.placement_epoch,
                encoded_bytes: action.encoded_bytes,
                protected: action.preserve_protected_form,
            })),
            OperationOutcome::Cancelled => Ok(TransferOutcome::Cancelled(reference)),
            OperationOutcome::Uncertain => Ok(TransferOutcome::Uncertain(reference)),
            OperationOutcome::Corrupt => Ok(TransferOutcome::Received(TransferEnvelope {
                transport_verification_ref: reference.clone(),
                transfer_ref: reference,
                operation_id: if self.operation_mismatch {
                    digest('f')
                } else {
                    action.operation_id.clone()
                },
                content_ref: action.content_ref.clone(),
                manifest_ref: digest('8'),
                source_peer: action.source_peer.clone().unwrap_or_default(),
                target_peer: action.target_peer.clone(),
                generation: GENERATION,
                membership_epoch: MEMBERSHIP_EPOCH,
                placement_epoch: self.placement_epoch,
                encoded_bytes: action.encoded_bytes,
                protected: !action.preserve_protected_form,
            })),
            OperationOutcome::Failed => Ok(TransferOutcome::Unavailable(reference)),
        }
    }
}

pub struct Durable {
    pub history: Vec<PriorOperation>,
    pub stored: Vec<PriorOperation>,
    pub status_count: usize,
    pub events: Events,
}

impl DurablePort for Durable {
    fn load_history(&mut self, _manifest: &Manifest) -> Result<Vec<PriorOperation>> {
        self.events.borrow_mut().push("history");
        Ok(self.history.clone())
    }

    fn store_operation(&mut self, operation: &PriorOperation) -> Result<String> {
        self.events.borrow_mut().push("store-operation");
        self.stored.push(operation.clone());
        self.history.push(operation.clone());
        Ok(digest('4'))
    }

    fn store_status(&mut self, _status: &CanonicalReplicationRecord) -> Result<String> {
        self.events.borrow_mut().push("store-status");
        self.status_count = self.status_count.saturating_add(1);
        Ok(digest('5'))
    }
}

pub struct Retention {
    pub pin_admitted: bool,
    pub cleanup_admitted: bool,
    pub events: Events,
}

impl RetentionPort for Retention {
    fn acquire_pin(&mut self, action: &Action) -> Result<PinObservation> {
        self.events.borrow_mut().push("pin");
        Ok(PinObservation {
            pin_ref: digest('6'),
            operation_id: action.operation_id.clone(),
            content_ref: action.content_ref.clone(),
            generation: GENERATION,
            admitted: self.pin_admitted,
        })
    }

    fn authorize_cleanup(&mut self, action: &Action) -> Result<CleanupObservation> {
        self.events.borrow_mut().push("authorize-cleanup");
        Ok(CleanupObservation {
            cleanup_ref: digest('7'),
            operation_id: action.operation_id.clone(),
            content_ref: action.content_ref.clone(),
            generation: GENERATION,
            admitted: self.cleanup_admitted,
        })
    }
}

pub struct Observations {
    pub events: Events,
}

impl ObservationPort for Observations {
    fn publish_plan(&mut self, _plan: &CanonicalReplicationRecord) -> Result<()> {
        self.events.borrow_mut().push("publish-plan");
        Ok(())
    }

    fn publish_operation(&mut self, _operation: &CanonicalReplicationRecord) -> Result<()> {
        self.events.borrow_mut().push("publish-operation");
        Ok(())
    }

    fn publish_status(&mut self, _status: &CanonicalReplicationRecord) -> Result<()> {
        self.events.borrow_mut().push("publish-status");
        Ok(())
    }
}

pub struct Receipts {
    pub count: usize,
    pub events: Events,
}

impl ReceiptPort for Receipts {
    fn publish_receipt(&mut self, _receipt: &CanonicalReplicationRecord) -> Result<()> {
        self.events.borrow_mut().push("publish-receipt");
        self.count = self.count.saturating_add(1);
        Ok(())
    }
}

pub struct EffectPorts {
    pub content: Content,
    pub transport: Transport,
    pub durable: Durable,
    pub retention: Retention,
    pub observations: Observations,
    pub receipts: Receipts,
}

impl EffectPorts {
    pub fn admitted(manifest: &Manifest, events: &Events) -> Self {
        Self {
            content: Content {
                inventory: Inventory {
                    replicas: vec![source_replica(manifest)],
                },
                verify_admitted: true,
                cleanup_count: 0,
                events: events.clone(),
            },
            transport: Transport {
                outcome: None,
                placement_epoch: PLACEMENT_EPOCH,
                operation_mismatch: false,
                calls: 0,
                events: events.clone(),
            },
            durable: Durable {
                history: Vec::new(),
                stored: Vec::new(),
                status_count: 0,
                events: events.clone(),
            },
            retention: Retention {
                pin_admitted: true,
                cleanup_admitted: true,
                events: events.clone(),
            },
            observations: Observations { events: events.clone() },
            receipts: Receipts {
                count: 0,
                events: events.clone(),
            },
        }
    }
}
