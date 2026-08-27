mod adapters;
#[cfg(unix)]
mod conformance_tests;
mod content;
mod support;

use std::cell::RefCell;
use std::rc::Rc;

use adapters::*;
#[cfg(unix)]
use content::*;
use molten_core::content_replication::*;
use support::*;

use super::*;

fn ports<'a>(facts: &'a mut FactPorts, effects: &'a mut EffectPorts) -> ReconcilePorts<'a> {
    ReconcilePorts {
        authority: &mut facts.authority,
        identity: &mut facts.identity,
        membership: &mut facts.membership,
        placement: &mut facts.placement,
        time: &mut facts.clock,
        resources: &mut facts.resources,
        content: &mut effects.content,
        transport: &mut effects.transport,
        durable: &mut effects.durable,
        retention: &mut effects.retention,
        observations: &mut effects.observations,
        receipts: &mut effects.receipts,
    }
}

fn conformance_ports<'a>(
    facts: &'a mut FactPorts,
    effects: &'a mut EffectPorts,
    content: &'a mut dyn ContentPort,
    transport: &'a mut dyn TransportPort,
) -> ReconcilePorts<'a> {
    ReconcilePorts {
        authority: &mut facts.authority,
        identity: &mut facts.identity,
        membership: &mut facts.membership,
        placement: &mut facts.placement,
        time: &mut facts.clock,
        resources: &mut facts.resources,
        content,
        transport,
        durable: &mut effects.durable,
        retention: &mut effects.retention,
        observations: &mut effects.observations,
        receipts: &mut effects.receipts,
    }
}

fn durable_ports<'a>(
    facts: &'a mut FactPorts,
    effects: &'a mut EffectPorts,
    durable: &'a mut dyn DurablePort,
) -> ReconcilePorts<'a> {
    ReconcilePorts {
        authority: &mut facts.authority,
        identity: &mut facts.identity,
        membership: &mut facts.membership,
        placement: &mut facts.placement,
        time: &mut facts.clock,
        resources: &mut facts.resources,
        content: &mut effects.content,
        transport: &mut effects.transport,
        durable,
        retention: &mut effects.retention,
        observations: &mut effects.observations,
        receipts: &mut effects.receipts,
    }
}

fn active(manifest: Manifest, facts: &mut FactPorts) -> ServiceInstance {
    activate(manifest, facts.activation()).expect("active replication service")
}

#[test]
fn receiver_driven_reconcile_pins_verifies_persists_and_receipts_last() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let manifest = manifest();
    let mut facts = FactPorts::admitted(&events);
    let instance = active(manifest.clone(), &mut facts);
    events.borrow_mut().clear();
    let mut effects = EffectPorts::admitted(&manifest, &events);
    let outcome = reconcile(instance, ports(&mut facts, &mut effects)).expect("complete reconciliation");
    assert_eq!(outcome.receipt.decision, ReceiptDecision::Complete);
    assert_eq!(outcome.status.verified_replicas, DEFAULT_REPLICAS);
    assert!(outcome.status.under_replicated.is_empty());
    assert_eq!(effects.transport.calls, 1);
    assert_eq!(effects.durable.stored.len(), 1);
    assert_eq!(effects.receipts.count, 1);
    let readback = operator_status(&outcome);
    assert_eq!(readback.resource_refs, vec![digest('0')]);
    assert!(!readback.evidence_refs.is_empty());
    assert_eq!(readback.non_claims.len(), NON_CLAIMS.len());
    assert!(canonical_operator_status(&readback).is_ok());
    let events = events.borrow();
    let pin = events.iter().position(|event| *event == "pin").expect("pin event");
    let transport = events.iter().position(|event| *event == "transport").expect("transport event");
    let verify = events.iter().position(|event| *event == "verify").expect("verify event");
    let store = events.iter().position(|event| *event == "store-operation").expect("store operation event");
    assert!(pin < transport && transport < verify && verify < store);
    assert_eq!(events.last(), Some(&"publish-receipt"));
}

#[test]
fn current_authority_and_placement_deny_before_transfer_effects() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let manifest = manifest();
    let mut facts = FactPorts::admitted(&events);
    let instance = active(manifest.clone(), &mut facts);
    let mut effects = EffectPorts::admitted(&manifest, &events);

    facts.authority.admitted = false;
    let denied = reconcile(instance.clone(), ports(&mut facts, &mut effects));
    assert!(denied.is_err());
    assert_eq!(effects.transport.calls, 0);
    assert_eq!(effects.receipts.count, 0);

    facts.authority.admitted = true;
    facts.placement.placement_epoch = PLACEMENT_EPOCH.saturating_add(1);
    let stale = reconcile(instance.clone(), ports(&mut facts, &mut effects));
    assert!(stale.is_err());
    assert_eq!(effects.transport.calls, 0);

    facts.placement.placement_epoch = PLACEMENT_EPOCH;
    effects.transport.placement_epoch = PLACEMENT_EPOCH.saturating_sub(1);
    let delayed = reconcile(instance.clone(), ports(&mut facts, &mut effects));
    assert!(delayed.is_err());
    assert_eq!(effects.transport.calls, 1);
    assert!(effects.durable.stored.is_empty());

    effects.transport.placement_epoch = PLACEMENT_EPOCH;
    effects.transport.operation_mismatch = true;
    let unsolicited = reconcile(instance, ports(&mut facts, &mut effects));
    assert!(unsolicited.is_err());
    assert_eq!(effects.transport.calls, 2);
    assert!(effects.durable.stored.is_empty());
}

#[test]
fn cancellation_persists_partial_progress_and_restart_reconciles() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let manifest = manifest();
    let mut facts = FactPorts::admitted(&events);
    let instance = active(manifest.clone(), &mut facts);
    let mut effects = EffectPorts::admitted(&manifest, &events);
    effects.transport.outcome = Some(OperationOutcome::Cancelled);
    let partial = reconcile(instance, ports(&mut facts, &mut effects)).expect("partial reconciliation");
    assert_eq!(partial.receipt.decision, ReceiptDecision::Partial);
    assert_eq!(effects.durable.history[0].outcome, OperationOutcome::Cancelled);

    let stopped = stop(&partial.instance).expect("stopped instance");
    let restarted = restart(&stopped, facts.activation()).expect("restarted instance");
    assert_eq!(restarted.restart_count, 1);
    effects.transport.outcome = None;
    let complete = reconcile(restarted, ports(&mut facts, &mut effects)).expect("completed retry");
    assert_eq!(complete.receipt.decision, ReceiptDecision::Complete);
    assert_eq!(complete.receipt.operations.len(), 2);
    assert_eq!(complete.receipt.operations.last().expect("retry").attempt, 2);
}

#[test]
fn retention_pin_denial_and_corrupt_envelope_never_advance_state() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let manifest = manifest();
    let mut facts = FactPorts::admitted(&events);
    let instance = active(manifest.clone(), &mut facts);
    let mut effects = EffectPorts::admitted(&manifest, &events);
    effects.retention.pin_admitted = false;
    let denied = reconcile(instance.clone(), ports(&mut facts, &mut effects));
    assert!(denied.is_err());
    assert_eq!(effects.transport.calls, 0);
    assert!(effects.durable.stored.is_empty());

    effects.retention.pin_admitted = true;
    effects.transport.outcome = Some(OperationOutcome::Corrupt);
    let corrupt = reconcile(instance, ports(&mut facts, &mut effects));
    assert!(corrupt.is_err());
    assert_eq!(effects.transport.calls, 1);
    assert!(effects.durable.stored.is_empty());
    assert_eq!(effects.receipts.count, 0);
}

#[test]
fn cleanup_requires_explicit_clearance_and_does_not_use_transport() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut manifest = manifest();
    manifest.policy.desired_replicas = 1;
    manifest.policy.minimum_verified_replicas = 1;
    manifest.policy.minimum_fault_domains = 1;
    let mut facts = FactPorts::admitted(&events);
    let instance = active(manifest.clone(), &mut facts);
    let mut effects = EffectPorts::admitted(&manifest, &events);
    let mut extra = source_replica(&manifest);
    extra.peer_id = "peer-b".to_string();
    extra.fault_domain = "zone-b".to_string();
    extra.pinned = false;
    extra.cleanup_clearance_ref = Some(digest('d'));
    effects.content.inventory.replicas.push(extra);
    let outcome = reconcile(instance, ports(&mut facts, &mut effects)).expect("cleanup reconciliation");
    assert_eq!(outcome.receipt.decision, ReceiptDecision::Complete);
    assert_eq!(effects.content.cleanup_count, 1);
    assert_eq!(effects.transport.calls, 0);
    let events = events.borrow();
    let admission = events.iter().position(|event| *event == "authorize-cleanup").expect("cleanup admission");
    let cleanup = events.iter().position(|event| *event == "cleanup").expect("cleanup effect");
    assert!(admission < cleanup);
}

#[test]
fn lifecycle_and_canonical_nonclaims_fail_closed() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let manifest = manifest();
    let mut facts = FactPorts::admitted(&events);
    let active = active(manifest.clone(), &mut facts);
    assert!(restart(&active, facts.activation()).is_err());
    let draining = drain(&active).expect("draining instance");
    let stopped = stop(&draining).expect("stopped instance");
    assert_eq!(stopped.state, LifecycleState::Stopped);

    let plan = molten_core::content_replication::plan(&ReconcileInput {
        manifest: manifest.clone(),
        inventory: Inventory {
            replicas: vec![source_replica(&manifest)],
        },
        peers: vec![peer("peer-a", "zone-a"), peer("peer-b", "zone-b")],
        history: Vec::new(),
        observed_tick: 1,
    })
    .expect("plan");
    let status = molten_core::content_replication::status(&plan, &[]);
    let canonical_status = canonical_status(&status).expect("canonical status");
    let mut receipt = ExecutionReceipt {
        decision: ReceiptDecision::Partial,
        service_id: manifest.service_id,
        generation: GENERATION,
        plan_ref: plan.plan_ref,
        status_ref: canonical_status.record_ref,
        operations: Vec::new(),
        evidence_refs: Vec::new(),
        issues: Vec::new(),
        non_claims: NON_CLAIMS.iter().map(ToString::to_string).collect(),
    };
    assert!(canonical_receipt(&receipt).is_ok());
    receipt.non_claims.pop();
    assert!(canonical_receipt(&receipt).is_err());
}
