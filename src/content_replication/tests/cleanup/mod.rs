use super::*;

mod rejections;

const PEER_A: &str = "peer-a";
const PEER_B: &str = "peer-b";
const PEER_C: &str = "peer-c";
const ZONE_A: &str = "zone-a";
const ZONE_B: &str = "zone-b";

struct CleanupContent {
    inventory: Inventory,
    is_cleanup_denied: bool,
    is_result_malformed: bool,
    events: Events,
}

impl ContentPort for CleanupContent {
    fn inventory(&mut self, _manifest: &Manifest) -> crate::error::Result<Inventory> {
        self.events.borrow_mut().push("inventory");
        Ok(self.inventory.clone())
    }

    fn verify(
        &mut self,
        _action: &Action,
        _envelope: &TransferEnvelope,
    ) -> crate::error::Result<VerificationObservation> {
        Err(crate::error::Failure::invalid_harness("cleanup fixture does not admit transfers"))
    }

    fn cleanup(&mut self, action: &Action, _admission: &CleanupObservation) -> crate::error::Result<String> {
        self.events.borrow_mut().push("cleanup");
        if self.is_cleanup_denied {
            return Err(crate::error::Failure::invalid_harness("controlled content cleanup denial"));
        }
        self.inventory
            .replicas
            .retain(|replica| replica.content_ref != action.content_ref || replica.peer_id != action.target_peer);
        Ok(if self.is_result_malformed {
            "malformed-cleanup-result".to_string()
        } else {
            digest('2')
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RetentionResponse {
    Valid,
    Denied,
    StaleGeneration,
    Malformed,
    AdapterError,
}

struct CleanupRetention {
    has_active_pin: bool,
    clearance_ref: String,
    response: RetentionResponse,
    events: Events,
}

impl RetentionPort for CleanupRetention {
    fn acquire_pin(&mut self, _action: &Action) -> crate::error::Result<PinObservation> {
        Err(crate::error::Failure::invalid_harness("cleanup fixture does not acquire transfer pins"))
    }

    fn authorize_cleanup(&mut self, action: &Action) -> crate::error::Result<CleanupObservation> {
        self.events.borrow_mut().push("authorize-cleanup");
        if self.response == RetentionResponse::AdapterError {
            return Err(crate::error::Failure::invalid_harness("controlled retention cleanup denial"));
        }
        Ok(CleanupObservation {
            cleanup_ref: if self.response == RetentionResponse::Malformed {
                "malformed-cleanup-admission".to_string()
            } else {
                digest('7')
            },
            operation_id: action.operation_id.clone(),
            content_ref: action.content_ref.clone(),
            generation: if self.response == RetentionResponse::StaleGeneration {
                GENERATION.saturating_sub(1)
            } else {
                GENERATION
            },
            admitted: self.response != RetentionResponse::Denied
                && !self.has_active_pin
                && action.cleanup_authority_ref == Some(digest('a'))
                && action.prior_result_ref.as_deref() == Some(self.clearance_ref.as_str()),
        })
    }
}

struct CleanupFixture {
    instance: ServiceInstance,
    facts: FactPorts,
    effects: EffectPorts,
    content: CleanupContent,
    retention: CleanupRetention,
    events: Events,
}

impl CleanupFixture {
    fn new() -> Self {
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let manifest = manifest();
        let mut facts = FactPorts::admitted(&events);
        facts.membership.peers = vec![peer(PEER_A, ZONE_A), peer(PEER_B, ZONE_A), peer(PEER_C, ZONE_B)];
        let instance = active(manifest.clone(), &mut facts);
        let mut effects = EffectPorts::admitted(&manifest, &events);
        effects.durable.history.push(PriorOperation {
            operation_id: digest('e'),
            content_ref: manifest.contents[0].content_ref.clone(),
            source_peer: Some(PEER_A.to_string()),
            target_peer: "peer-z".to_string(),
            generation: GENERATION,
            membership_epoch: MEMBERSHIP_EPOCH,
            placement_epoch: PLACEMENT_EPOCH,
            attempt: 1,
            outcome: OperationOutcome::Failed,
            result_ref: Some(digest('f')),
        });
        let replicas = [(PEER_A, ZONE_A), (PEER_B, ZONE_A), (PEER_C, ZONE_B)]
            .iter()
            .map(|(id, domain)| {
                let mut replica = source_replica(&manifest);
                replica.peer_id = (*id).to_string();
                replica.fault_domain = (*domain).to_string();
                replica.pinned = false;
                replica.cleanup_clearance_ref = Some(digest('d'));
                replica
            })
            .collect();
        events.borrow_mut().clear();
        Self {
            instance,
            facts,
            effects,
            content: CleanupContent {
                inventory: Inventory { replicas },
                is_cleanup_denied: false,
                is_result_malformed: false,
                events: events.clone(),
            },
            retention: CleanupRetention {
                has_active_pin: false,
                clearance_ref: digest('d'),
                response: RetentionResponse::Valid,
                events: events.clone(),
            },
            events,
        }
    }

    fn run(&mut self) -> crate::error::Result<ReconcileOutcome> {
        let mut ports = super::ports(&mut self.facts, &mut self.effects);
        ports.content = &mut self.content;
        ports.retention = &mut self.retention;
        reconcile(self.instance.clone(), ports)
    }

    fn retention_state(&self) -> (bool, String) {
        (self.retention.has_active_pin, self.retention.clearance_ref.clone())
    }

    fn assert_no_operation_success(&self, history: &[PriorOperation]) {
        assert_eq!(self.effects.durable.history, history);
        assert!(self.effects.durable.stored.is_empty());
        assert!(!self.events.borrow().contains(&"store-operation"));
        assert!(!self.events.borrow().contains(&"publish-operation"));
        assert_eq!(self.effects.transport.calls, 0);
    }

    fn assert_rejected_before_deletion(&mut self) {
        let inventory = self.content.inventory.clone();
        let retention = self.retention_state();
        let history = self.effects.durable.history.clone();
        assert!(self.run().is_err());
        assert_eq!(self.content.inventory, inventory);
        assert_eq!(self.retention_state(), retention);
        self.assert_no_operation_success(&history);
        assert_eq!(self.effects.durable.status_count, 0);
        assert_eq!(self.effects.receipts.count, 0);
    }
}

// r[verify molten.audit_f05.residual_policy]
// r[verify molten.audit_f05.authority_preservation]
// r[verify molten.audit_f05.validation_claims]
#[test]
fn controlled_cleanup_preserves_unique_domain_and_admits_before_deletion() {
    let mut fixture = CleanupFixture::new();
    let retention = fixture.retention_state();
    let outcome = fixture.run().expect("admitted cleanup");
    let peers = fixture
        .content
        .inventory
        .replicas
        .iter()
        .map(|replica| replica.peer_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(peers, vec![PEER_A, PEER_C]);
    assert_eq!(fixture.content.inventory.replicas[0].fault_domain, ZONE_A);
    assert_eq!(fixture.content.inventory.replicas[1].fault_domain, ZONE_B);
    assert_eq!(fixture.retention_state(), retention);
    assert_eq!(outcome.receipt.decision, ReceiptDecision::Complete);
    let stored = fixture.effects.durable.stored.first().expect("durable cleanup operation");
    assert_eq!(stored.target_peer, PEER_B);
    assert_eq!(stored.outcome, OperationOutcome::Verified);
    assert_eq!(stored.result_ref, Some(digest('2')));
    assert_eq!(fixture.effects.transport.calls, 0);
    let events = fixture.events.borrow();
    let admission = events.iter().position(|event| *event == "authorize-cleanup").expect("cleanup admission");
    let cleanup = events.iter().position(|event| *event == "cleanup").expect("cleanup call");
    let durable = events.iter().position(|event| *event == "store-operation").expect("durable cleanup");
    let published = events.iter().position(|event| *event == "publish-operation").expect("cleanup observation");
    assert!(admission < cleanup);
    assert!(cleanup < durable);
    assert!(durable < published);
    assert_eq!(events.last(), Some(&"publish-receipt"));
}
