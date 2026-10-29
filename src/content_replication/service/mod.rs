use molten_core::content_replication::*;

use super::*;

pub struct ActivationPorts<'a> {
    pub authority: &'a mut dyn AuthorityPort,
    pub identity: &'a mut dyn IdentityPort,
    pub membership: &'a mut dyn MembershipPort,
    pub placement: &'a mut dyn PlacementPort,
}

// r[impl molten.content_replication.same_core]
pub struct ReconcilePorts<'a> {
    pub authority: &'a mut dyn AuthorityPort,
    pub identity: &'a mut dyn IdentityPort,
    pub membership: &'a mut dyn MembershipPort,
    pub placement: &'a mut dyn PlacementPort,
    pub time: &'a mut dyn TimePort,
    pub resources: &'a mut dyn ResourcePort,
    pub content: &'a mut dyn ContentPort,
    pub transport: &'a mut dyn TransportPort,
    pub durable: &'a mut dyn DurablePort,
    pub retention: &'a mut dyn RetentionPort,
    pub observations: &'a mut dyn ObservationPort,
    pub receipts: &'a mut dyn ReceiptPort,
}

// r[impl molten.content_replication.manifest]
pub fn activate(manifest: Manifest, ports: ActivationPorts<'_>) -> crate::error::Result<ServiceInstance> {
    let issues = validate_manifest(&manifest);
    if !issues.is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!("content-replication manifest denied: {issues:?}")));
    }
    let authority = ports.authority.observe(&manifest)?;
    validate_authority(&manifest, &authority)?;
    let identity = ports.identity.observe(&manifest)?;
    validate_identity(&manifest, &identity)?;
    let membership = ports.membership.observe(&manifest)?;
    validate_membership(&manifest, &membership)?;
    let placement = ports.placement.observe(&manifest)?;
    validate_placement(&manifest, &placement)?;
    let _canonical_manifest = canonical_manifest(&manifest)?;
    Ok(ServiceInstance {
        manifest,
        state: LifecycleState::Active,
        restart_count: 0,
        last_plan_ref: None,
    })
}

pub fn restart(instance: &ServiceInstance, ports: ActivationPorts<'_>) -> crate::error::Result<ServiceInstance> {
    if !matches!(instance.state, LifecycleState::Stopped | LifecycleState::Failed) {
        return Err(crate::error::Failure::invalid_harness(
            "content-replication restart requires a stopped or failed instance",
        ));
    }
    let mut restarted = activate(instance.manifest.clone(), ports)?;
    restarted.restart_count = instance
        .restart_count
        .checked_add(1)
        .ok_or_else(|| crate::error::Failure::invalid_harness("content-replication restart count overflow"))?;
    Ok(restarted)
}

pub fn drain(instance: &ServiceInstance) -> crate::error::Result<ServiceInstance> {
    if instance.state != LifecycleState::Active {
        return Err(crate::error::Failure::invalid_harness("content-replication drain requires an active instance"));
    }
    let mut drained = instance.clone();
    drained.state = LifecycleState::Draining;
    Ok(drained)
}

pub fn stop(instance: &ServiceInstance) -> crate::error::Result<ServiceInstance> {
    if !matches!(instance.state, LifecycleState::Active | LifecycleState::Draining) {
        return Err(crate::error::Failure::invalid_harness(
            "content-replication stop requires an active or draining instance",
        ));
    }
    let mut stopped = instance.clone();
    stopped.state = LifecycleState::Stopped;
    Ok(stopped)
}

pub fn operator_status(outcome: &ReconcileOutcome) -> OperatorStatusView {
    OperatorStatusView {
        service_id: outcome.instance.manifest.service_id.clone(),
        generation: outcome.status.generation,
        placement_epoch: outcome.status.placement_epoch,
        desired_replicas: outcome.status.desired_replicas,
        verified_replicas: outcome.status.verified_replicas,
        under_replicated: outcome.status.under_replicated.clone(),
        active_plan_ref: outcome.plan.plan_ref.clone(),
        active_operations: outcome.status.active_operations.clone(),
        resource_refs: outcome.resource_refs.clone(),
        failures: outcome.status.failures.clone(),
        pins: outcome.status.pins.clone(),
        evidence_refs: outcome.receipt.evidence_refs.clone(),
        repair_completeness: outcome.repair_plan.completeness.as_str().to_string(),
        repair_unrecoverable: outcome.repair_plan.unrecoverable_objects.clone(),
        repair_non_claims: outcome.repair_plan.non_claims.clone(),
        non_claims: outcome.status.non_claims.clone(),
    }
}

pub fn reconcile(
    mut instance: ServiceInstance,
    mut ports: ReconcilePorts<'_>,
) -> crate::error::Result<ReconcileOutcome> {
    require_active(&instance)?;
    let facts = observe_current_facts(&instance.manifest, FactPorts {
        authority_port: &mut *ports.authority,
        identity_port: &mut *ports.identity,
        membership_port: &mut *ports.membership,
        placement_port: &mut *ports.placement,
        time_port: &mut *ports.time,
    })?;
    let planning = plan_reconciliation(&instance.manifest, &facts, &mut ports)?;
    let Planning {
        plan,
        repair_plan,
        history,
        observed_inventory,
    } = planning;
    let canonical_plan = canonical_plan(&plan)?;
    ports.observations.publish_plan(&canonical_plan)?;
    let mut evidence_refs = facts.evidence_refs();
    let mut resource_refs = Vec::new();
    evidence_refs.push(canonical_plan.record_ref.clone());
    let resources = ports.resources.reserve(&plan)?;
    validate_resources(&instance.manifest, &plan, &resources)?;
    resource_refs.push(resources.reservation_ref.clone());
    evidence_refs.push(resources.reservation_ref);
    let execution = execute_actions(&instance.manifest, &plan, &history, &mut evidence_refs, ports)?;
    // The eligibility gate sits between the transfers and the metadata
    // publication. The requirement is the profile's replica policy, and the
    // receipt binds the content, the operation, the receivers, the retention
    // generation, the placement epoch, and the storage guarantee.
    let eligibility = publication_eligibility(&instance.manifest, &plan, &observed_inventory, &execution.operations)?;
    evidence_refs.extend(eligibility.iter().map(|receipt| receipt.receipt_ref.clone()));
    let ExecutionResult {
        operations,
        durable,
        observations,
        receipts,
    } = execution;
    let published = publish_status_and_receipt(
        &instance,
        &plan,
        operations,
        PublicationPorts {
            durable,
            observations,
            receipts,
        },
        evidence_refs,
    )?;
    let (status, canonical_status, receipt, canonical_receipt) = published;
    instance.last_plan_ref = Some(plan.plan_ref.clone());
    Ok(ReconcileOutcome {
        instance,
        plan,
        repair_plan,
        eligibility_receipts: eligibility,
        status,
        receipt,
        resource_refs,
        canonical_plan,
        canonical_status,
        canonical_receipt,
    })
}

/// The planning phase of a reconciliation.
struct Planning {
    plan: Plan,
    repair_plan: ClosurePlan,
    history: Vec<PriorOperation>,
    observed_inventory: Inventory,
}

/// Plan the reconciliation over the current facts and the observed inventory.
fn plan_reconciliation(
    manifest: &Manifest,
    facts: &CurrentFacts,
    ports: &mut ReconcilePorts<'_>,
) -> crate::error::Result<Planning> {
    let inventory = ports.content.inventory(manifest)?;
    let observed_inventory = inventory.clone();
    let repair_plan = plan_closure(&closure_repair_input(manifest, &inventory)).map_err(|issue| {
        crate::error::Failure::invalid_harness(format!("closure repair planning denied: {issue:?}"))
    })?;
    let history = ports.durable.load_history(manifest)?;
    let input = ReconcileInput {
        manifest: manifest.clone(),
        inventory,
        peers: facts.membership.peers.clone(),
        history: history.clone(),
        observed_tick: facts.time.observed_tick,
    };
    let plan = molten_core::content_replication::plan(&input)
        .map_err(|issue| crate::error::Failure::invalid_harness(format!("replication planning denied: {issue:?}")))?;
    if plan.decision == Decision::Denied {
        return Err(crate::error::Failure::invalid_harness(format!("replication planning denied: {:?}", plan.issues)));
    }
    Ok(Planning {
        plan,
        repair_plan,
        history,
        observed_inventory,
    })
}

/// The ports the publication step writes through.
struct PublicationPorts<'a> {
    durable: &'a mut dyn DurablePort,
    observations: &'a mut dyn ObservationPort,
    receipts: &'a mut dyn ReceiptPort,
}

/// Publish the durable status, then the receipt that binds it.
fn publish_status_and_receipt(
    instance: &ServiceInstance,
    plan: &Plan,
    operations: Vec<PriorOperation>,
    ports: PublicationPorts<'_>,
    evidence_refs: Vec<String>,
) -> crate::error::Result<(Status, CanonicalReplicationRecord, ExecutionReceipt, CanonicalReplicationRecord)> {
    let status = molten_core::content_replication::status(plan, &operations);
    let canonical_status = canonical_status(&status)?;
    let durable_status_ref = ports.durable.store_status(&canonical_status)?;
    validate_ref(&durable_status_ref, "replication durable status")?;
    ports.observations.publish_status(&canonical_status)?;
    let mut evidence_refs = evidence_refs;
    evidence_refs.push(durable_status_ref);
    evidence_refs.push(canonical_status.record_ref.clone());
    evidence_refs.sort();
    evidence_refs.dedup();
    let receipt = execution_receipt(ReceiptInput {
        instance,
        plan,
        status: &status,
        canonical_status: &canonical_status,
        operations,
        evidence_refs,
    });
    let canonical_receipt = canonical_receipt(&receipt)?;
    ports.receipts.publish_receipt(&canonical_receipt)?;
    Ok((status, canonical_status, receipt, canonical_receipt))
}

mod eligibility;
mod execution;
mod facts;
mod repair;
mod replay;
mod validation;

use eligibility::*;
use execution::*;
use facts::*;
use repair::*;
use replay::*;
use validation::*;

#[cfg(test)]
#[path = "../tests/replay.rs"]
mod replay_tests;
