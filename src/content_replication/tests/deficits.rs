use super::*;

const FIRST_ATTEMPT: u32 = 1;
const SECOND_ATTEMPT: u32 = 2;
const SOURCE_COUNT: usize = 1;

struct Fixture {
    instance: ServiceInstance,
    facts: FactPorts,
    effects: EffectPorts,
    events: Events,
}

impl Fixture {
    fn new(manifest: Manifest) -> Self {
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut facts = FactPorts::admitted(&events);
        let instance = active(manifest.clone(), &mut facts);
        let effects = EffectPorts::admitted(&manifest, &events);
        events.borrow_mut().clear();
        Self {
            instance,
            facts,
            effects,
            events,
        }
    }

    fn run(&mut self) -> crate::error::Result<ReconcileOutcome> {
        reconcile(self.instance.clone(), ports(&mut self.facts, &mut self.effects))
    }
}

// r[verify molten.audit_f07.deficit_status]
// r[verify molten.audit_f07.receipt_decision]
#[test]
fn action_free_deficit_publishes_partial_without_transfer_effects() {
    let mut manifest = manifest();
    manifest.policy.minimum_fault_domains = SOURCE_COUNT;
    let mut fixture = Fixture::new(manifest);
    fixture.facts.membership.peers.truncate(SOURCE_COUNT);
    let before = fixture.effects.content.inventory.clone();
    let outcome = fixture.run().expect("action-free deficit");
    assert_eq!(outcome.plan.decision, Decision::Partial);
    assert_eq!(outcome.plan.issues, vec![Issue::InsufficientPeers]);
    assert!(outcome.plan.actions.is_empty());
    assert_eq!(outcome.receipt.decision, ReceiptDecision::Partial);
    assert_eq!(outcome.status.verified_replicas, SOURCE_COUNT);
    assert_eq!(outcome.status.under_replicated, vec![digest('7')]);
    assert_eq!(fixture.effects.content.inventory, before);
    assert_eq!(fixture.effects.transport.calls, 0);
    assert!(fixture.effects.durable.history.is_empty());
    assert!(!fixture.events.borrow().contains(&"verify"));
    assert_eq!(fixture.effects.durable.status_count, 1);
    assert_eq!(fixture.effects.receipts.count, 1);
}

// r[verify molten.audit_f07.per_content_accounting]
// r[verify molten.audit_f07.receipt_decision]
#[test]
fn successful_selected_target_does_not_clear_unselected_demand() {
    let mut manifest = manifest();
    manifest.policy.desired_replicas = 3;
    manifest.policy.minimum_verified_replicas = SOURCE_COUNT;
    manifest.policy.minimum_fault_domains = SOURCE_COUNT;
    let mut fixture = Fixture::new(manifest);
    let outcome = fixture.run().expect("one eligible target for two missing replicas");
    assert_eq!(outcome.plan.decision, Decision::Partial);
    assert_eq!(outcome.plan.issues, vec![Issue::InsufficientPeers]);
    assert_eq!(fixture.effects.transport.calls, 1);
    assert_eq!(fixture.effects.durable.stored[0].outcome, OperationOutcome::Verified);
    assert_eq!(outcome.status.verified_replicas, DEFAULT_REPLICAS);
    assert_eq!(outcome.status.under_replicated, vec![digest('7')]);
    assert_eq!(outcome.receipt.decision, ReceiptDecision::Partial);
}

// r[verify molten.audit_f04.current_availability]
// r[verify molten.audit_f04.fresh_repair]
// r[verify molten.audit_f04.replay_preservation]
// r[verify molten.audit_f07.per_content_accounting]
#[test]
fn new_reconciliation_repairs_same_epoch_loss_without_rewriting_saved_success() {
    let mut fixture = Fixture::new(manifest());
    let original = fixture.run().expect("initial transfer");
    let saved_operation = fixture.effects.durable.history[0].clone();
    let saved_record = canonical_operation(&saved_operation).expect("original operation record");
    let saved_receipt = original.canonical_receipt.clone();
    fixture.instance = original.instance;
    let repaired = fixture.run().expect("fresh same-epoch repair");
    let repair = fixture.effects.durable.history.last().expect("new durable repair");
    assert_eq!(fixture.effects.transport.calls, 2);
    assert_eq!(saved_operation.attempt, FIRST_ATTEMPT);
    assert_eq!(repair.attempt, SECOND_ATTEMPT);
    assert_ne!(repair.operation_id, saved_operation.operation_id);
    assert_eq!(fixture.effects.durable.history[0], saved_operation);
    assert_eq!(
        canonical_operation(&repaired.receipt.operations[0]).expect("saved readback").bytes,
        saved_record.bytes
    );
    assert_eq!(canonical_receipt(&original.receipt).expect("historical receipt").bytes, saved_receipt.bytes);
    assert_eq!(repaired.receipt.decision, ReceiptDecision::Complete);
    assert_eq!(repaired.status.verified_replicas, DEFAULT_REPLICAS);
    assert!(repaired.status.under_replicated.is_empty());
}

// r[verify molten.audit_f04.fresh_repair]
// r[verify molten.audit_f07.receipt_decision]
#[test]
fn unavailable_fresh_repair_cannot_reuse_historical_success_as_availability() {
    let mut fixture = Fixture::new(manifest());
    let original = fixture.run().expect("initial transfer");
    fixture.instance = original.instance;
    fixture.effects.transport.outcome = Some(OperationOutcome::Failed);
    let unavailable = fixture.run().expect("classified unavailable transfer");
    assert_eq!(unavailable.receipt.decision, ReceiptDecision::Partial);
    assert_eq!(unavailable.status.verified_replicas, SOURCE_COUNT);
    assert_eq!(unavailable.status.under_replicated, vec![digest('7')]);
    let failed = fixture.effects.durable.stored.last().expect("failed fresh operation");
    assert_eq!(failed.attempt, SECOND_ATTEMPT);
    assert_eq!(failed.outcome, OperationOutcome::Failed);
    assert_eq!(unavailable.status.failures, vec![failed.operation_id.clone()]);
    assert_eq!(fixture.effects.durable.history[0].outcome, OperationOutcome::Verified);
}

// r[verify molten.audit_f04.fresh_repair]
// r[verify molten.audit_f04.replay_preservation]
// r[verify molten.audit_f07.receipt_decision]
#[test]
fn verification_stale_envelope_and_denied_plan_preserve_prior_durable_state() {
    for rejection in ["verification", "stale-envelope", "denied-plan"] {
        let mut fixture = Fixture::new(manifest());
        let original = fixture.run().expect("initial transfer");
        fixture.instance = original.instance;
        match rejection {
            "verification" => fixture.effects.content.verify_admitted = false,
            "stale-envelope" => fixture.effects.transport.placement_epoch = PLACEMENT_EPOCH.saturating_sub(1),
            "denied-plan" => {
                let duplicate = fixture.effects.content.inventory.replicas[0].clone();
                fixture.effects.content.inventory.replicas.push(duplicate);
            }
            _ => unreachable!("closed rejection fixture"),
        }
        let inventory = fixture.effects.content.inventory.clone();
        let history = fixture.effects.durable.history.clone();
        let stored = fixture.effects.durable.stored.clone();
        fixture.events.borrow_mut().clear();
        assert!(fixture.run().is_err());
        assert_eq!(fixture.effects.content.inventory, inventory);
        assert_eq!(fixture.effects.durable.history, history);
        assert_eq!(fixture.effects.durable.stored, stored);
        assert_eq!(fixture.effects.durable.status_count, 1);
        assert_eq!(fixture.effects.receipts.count, 1);
        assert!(!fixture.events.borrow().contains(&"publish-operation"));
        assert!(!fixture.events.borrow().contains(&"publish-receipt"));
    }
}

// r[verify molten.audit_f04.fresh_repair]
// r[verify molten.audit_f07.receipt_decision]
#[test]
fn exhausted_fresh_repair_retains_loss_without_another_transfer() {
    let mut manifest = manifest();
    manifest.repair.max_attempts = FIRST_ATTEMPT;
    let mut fixture = Fixture::new(manifest);
    let original = fixture.run().expect("only admitted attempt");
    fixture.instance = original.instance;
    let history = fixture.effects.durable.history.clone();
    let exhausted = fixture.run().expect("bounded exhausted repair");
    assert_eq!(exhausted.plan.issues, vec![Issue::RepairExhausted]);
    assert_eq!(exhausted.receipt.decision, ReceiptDecision::Partial);
    assert_eq!(exhausted.status.verified_replicas, SOURCE_COUNT);
    assert_eq!(exhausted.status.under_replicated, vec![digest('7')]);
    assert_eq!(fixture.effects.transport.calls, 1);
    assert_eq!(fixture.effects.durable.history, history);
}

// r[verify molten.audit_f07.validation_claims]
// r[verify molten.audit_f04.validation_claims]
#[test]
fn complete_and_partial_canonical_status_round_trip_without_changing_saved_records() {
    let mut fixture = Fixture::new(manifest());
    let complete = fixture.run().expect("complete receipt");
    let saved_bytes = complete.canonical_status.bytes.clone();
    let saved_ref = complete.canonical_status.record_ref.clone();
    fixture.instance = complete.instance;
    fixture.effects.transport.outcome = Some(OperationOutcome::Failed);
    let partial = fixture.run().expect("partial current receipt");
    for record in [&complete.canonical_status, &partial.canonical_status] {
        let readback = crate::preserves_rail::parse_canonical_bytes(&record.bytes).expect("canonical status readback");
        assert_eq!(readback, record.value);
    }
    assert_eq!(complete.receipt.decision, ReceiptDecision::Complete);
    assert_eq!(partial.receipt.decision, ReceiptDecision::Partial);
    assert_ne!(partial.canonical_status.record_ref, saved_ref);
    assert_eq!(canonical_status(&complete.status).expect("historical status").bytes, saved_bytes);
    let mut truncated = partial.canonical_status.bytes.clone();
    truncated.pop();
    assert!(crate::preserves_rail::parse_canonical_bytes(&truncated).is_err());
}

// r[verify molten.audit_f07.validation_claims]
#[test]
fn malformed_status_bindings_counts_references_and_nonclaims_fail_closed() {
    let mut fixture = Fixture::new(manifest());
    let complete = fixture.run().expect("valid status producer");
    for malformed in 0..7 {
        let mut status = complete.status.clone();
        match malformed {
            0 => status.plan_ref = "not-a-reference".to_string(),
            1 => status.verified_replicas = status.desired_replicas.saturating_add(1),
            2 => status.generation = 0,
            3 => status.under_replicated = vec![digest('7'), digest('7')],
            4 => {
                status.non_claims.pop();
            }
            5 => status.active_operations.push("not-a-reference".to_string()),
            _ => status.under_replicated = vec![digest('7'); MAX_CONTENTS.saturating_add(1)],
        }
        assert!(canonical_status(&status).is_err());
    }
}

struct EligibilityFixture {
    instance: ServiceInstance,
    facts: FactPorts,
    effects: EffectPorts,
}

impl EligibilityFixture {
    fn new(manifest: Manifest) -> Self {
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut facts = FactPorts::admitted(&events);
        let instance = active(manifest.clone(), &mut facts);
        let effects = EffectPorts::admitted(&manifest, &events);
        Self {
            instance,
            facts,
            effects,
        }
    }

    fn run(&mut self) -> crate::error::Result<ReconcileOutcome> {
        reconcile(self.instance.clone(), ports(&mut self.facts, &mut self.effects))
    }
}

// r[verify molten.observations.eligibility]
// r[verify molten.observations.tests]
#[test]
fn an_admitted_publication_carries_the_bound_eligibility_receipt() {
    let mut fixture = EligibilityFixture::new(manifest());
    let outcome = fixture.run().expect("admitted publication");
    assert_eq!(outcome.eligibility_receipts.len(), 1, "the published content carries one receipt");
    let receipt = &outcome.eligibility_receipts[0];
    assert_eq!(receipt.content_ref, digest('7'), "the receipt binds the exact content identity");
    assert!(!receipt.operation_id.is_empty(), "the receipt binds the operation");
    assert_eq!(receipt.receiver_set.len(), 2, "the receipt binds the receiver set");
    assert_eq!(receipt.retention_generation, manifest().generation);
    assert_eq!(receipt.placement_epoch, manifest().placement_epoch);
    assert_eq!(
        receipt.storage_guarantee,
        StorageGuaranteeKind::ReplicatedAcrossFaultDomains {
            replicas: 2,
            fault_domains: 2,
        },
        "the receipt names the storage guarantee instead of a bare replica count"
    );
    assert!(
        outcome.receipt.evidence_refs.contains(&receipt.receipt_ref),
        "the receipt is part of the published evidence"
    );
}

// r[verify molten.observations.eligibility]
#[test]
fn a_publication_that_cannot_satisfy_the_profile_never_claims_the_content() {
    let mut manifest = manifest();
    manifest.policy.minimum_fault_domains = DEFAULT_REPLICAS;
    let mut fixture = EligibilityFixture::new(manifest);
    // Every admitted peer now lives in one fault domain, so the deployment
    // cannot satisfy the profile's spread requirement.
    fixture.facts.membership.peers = vec![peer("peer-a", "zone-a"), peer("peer-b", "zone-a")];
    let outcome = fixture.run().expect("the publication reports the deficit instead of claiming the content");
    assert!(outcome.eligibility_receipts.is_empty(), "no receipt is issued for content the profile cannot admit");
    assert!(
        outcome.status.under_replicated.contains(&digest('7')),
        "the status reports the content as under replicated"
    );
}
