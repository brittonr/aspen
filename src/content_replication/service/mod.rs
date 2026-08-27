use molten_core::content_replication::*;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub struct ActivationPorts<'a> {
    pub authority: &'a mut dyn AuthorityPort,
    pub identity: &'a mut dyn IdentityPort,
    pub membership: &'a mut dyn MembershipPort,
    pub placement: &'a mut dyn PlacementPort,
}

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

pub fn activate(manifest: Manifest, ports: ActivationPorts<'_>) -> Result<ServiceInstance> {
    let issues = validate_manifest(&manifest);
    if !issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("content-replication manifest denied: {issues:?}")));
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

pub fn restart(instance: &ServiceInstance, ports: ActivationPorts<'_>) -> Result<ServiceInstance> {
    if !matches!(instance.state, LifecycleState::Stopped | LifecycleState::Failed) {
        return Err(MoltenError::invalid_harness("content-replication restart requires a stopped or failed instance"));
    }
    let mut restarted = activate(instance.manifest.clone(), ports)?;
    restarted.restart_count = instance
        .restart_count
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("content-replication restart count overflow"))?;
    Ok(restarted)
}

pub fn drain(instance: &ServiceInstance) -> Result<ServiceInstance> {
    if instance.state != LifecycleState::Active {
        return Err(MoltenError::invalid_harness("content-replication drain requires an active instance"));
    }
    let mut drained = instance.clone();
    drained.state = LifecycleState::Draining;
    Ok(drained)
}

pub fn stop(instance: &ServiceInstance) -> Result<ServiceInstance> {
    if !matches!(instance.state, LifecycleState::Active | LifecycleState::Draining) {
        return Err(MoltenError::invalid_harness("content-replication stop requires an active or draining instance"));
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
        non_claims: outcome.status.non_claims.clone(),
    }
}

pub fn reconcile(mut instance: ServiceInstance, ports: ReconcilePorts<'_>) -> Result<ReconcileOutcome> {
    require_active(&instance)?;
    let facts = observe_current_facts(
        &instance.manifest,
        &mut *ports.authority,
        &mut *ports.identity,
        &mut *ports.membership,
        &mut *ports.placement,
        &mut *ports.time,
    )?;
    let inventory = ports.content.inventory(&instance.manifest)?;
    let history = ports.durable.load_history(&instance.manifest)?;
    let input = ReconcileInput {
        manifest: instance.manifest.clone(),
        inventory,
        peers: facts.membership.peers.clone(),
        history: history.clone(),
        observed_tick: facts.time.observed_tick,
    };
    let plan = molten_core::content_replication::plan(&input)
        .map_err(|issue| MoltenError::invalid_harness(format!("replication planning denied: {issue:?}")))?;
    let canonical_plan = canonical_plan(&plan)?;
    ports.observations.publish_plan(&canonical_plan)?;
    let mut evidence_refs = facts.evidence_refs();
    let mut resource_refs = Vec::new();
    evidence_refs.push(canonical_plan.record_ref.clone());
    if plan.decision != Decision::Denied {
        let resources = ports.resources.reserve(&plan)?;
        validate_resources(&instance.manifest, &plan, &resources)?;
        resource_refs.push(resources.reservation_ref.clone());
        evidence_refs.push(resources.reservation_ref);
    }
    let execution = execute_actions(&instance.manifest, &plan, &history, &mut evidence_refs, ports)?;
    let status = molten_core::content_replication::status(&plan, &execution.operations);
    let canonical_status = canonical_status(&status)?;
    let durable_status_ref = execution.durable.store_status(&canonical_status)?;
    validate_ref(&durable_status_ref, "replication durable status")?;
    execution.observations.publish_status(&canonical_status)?;
    evidence_refs.push(durable_status_ref);
    evidence_refs.push(canonical_status.record_ref.clone());
    evidence_refs.sort();
    evidence_refs.dedup();
    let receipt = execution_receipt(&instance, &plan, &status, &canonical_status, execution.operations, evidence_refs);
    let canonical_receipt = canonical_receipt(&receipt)?;
    execution.receipts.publish_receipt(&canonical_receipt)?;
    instance.last_plan_ref = Some(plan.plan_ref.clone());
    Ok(ReconcileOutcome {
        instance,
        plan,
        status,
        receipt,
        resource_refs,
        canonical_plan,
        canonical_status,
        canonical_receipt,
    })
}

struct CurrentFacts {
    authority: AuthorityObservation,
    identity: IdentityObservation,
    membership: MembershipObservation,
    placement: PlacementObservation,
    time: TimeObservation,
}

impl CurrentFacts {
    fn evidence_refs(&self) -> Vec<String> {
        vec![
            self.authority.observation_ref.clone(),
            self.identity.observation_ref.clone(),
            self.membership.observation_ref.clone(),
            self.placement.observation_ref.clone(),
            self.time.observation_ref.clone(),
        ]
    }
}

fn observe_current_facts(
    manifest: &Manifest,
    authority_port: &mut dyn AuthorityPort,
    identity_port: &mut dyn IdentityPort,
    membership_port: &mut dyn MembershipPort,
    placement_port: &mut dyn PlacementPort,
    time_port: &mut dyn TimePort,
) -> Result<CurrentFacts> {
    let authority = authority_port.observe(manifest)?;
    validate_authority(manifest, &authority)?;
    let identity = identity_port.observe(manifest)?;
    validate_identity(manifest, &identity)?;
    let membership = membership_port.observe(manifest)?;
    validate_membership(manifest, &membership)?;
    let placement = placement_port.observe(manifest)?;
    validate_placement(manifest, &placement)?;
    let time = time_port.observe(manifest)?;
    validate_ref(&time.observation_ref, "replication time observation")?;
    Ok(CurrentFacts {
        authority,
        identity,
        membership,
        placement,
        time,
    })
}

mod execution;
mod validation;

use execution::*;
use validation::*;
