use molten_core::dag_sync::*;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub struct DagSyncPorts<'a, A, R, T, C, P, O, E> {
    pub authority: &'a mut A,
    pub resources: &'a mut R,
    pub transport: &'a mut T,
    pub content: &'a mut C,
    pub progress: &'a mut P,
    pub observations: &'a mut O,
    pub receipts: &'a mut E,
}

#[derive(Debug, Clone)]
pub struct DagSyncOutcome {
    pub plan: DagSyncPlan,
    pub progress: DagSyncProgress,
    pub receipt: DagSyncReceipt,
    pub authority_ref: String,
    pub resource_ref: String,
    pub evidence_refs: Vec<String>,
    pub canonical_receipt: CanonicalDagRecord,
}

// r[impl molten.dag_sync.receiver_driven]
// r[impl molten.dag_sync.content_adapter_boundary]
// r[impl molten.dag_sync.resume_fencing]
#[allow(
    clippy::too_many_lines,
    reason = "the imperative shell keeps authority, resource, transfer, verification, persistence, observation, and receipt order visible"
)]
pub fn run_dag_sync<A, R, T, C, P, O, E>(
    graph: &DagGraph,
    mut request: DagSyncRequest,
    ports: DagSyncPorts<'_, A, R, T, C, P, O, E>,
) -> Result<DagSyncOutcome>
where
    A: DagAuthorityPort,
    R: DagResourcePort,
    T: DagTransportPort,
    C: DagContentVerificationPort,
    P: DagProgressPort,
    O: DagObservationPort,
    E: DagReceiptPort,
{
    let loaded = ports.progress.load(&request.epoch_ref)?;
    if request.progress.is_some() && loaded.is_some() && request.progress != loaded {
        return Err(MoltenError::invalid_harness("DAG-sync caller progress differs from durable progress"));
    }
    if request.progress.is_none() {
        request.progress = loaded;
    }
    let result = plan_dag_sync(graph, &request);
    let plan = result
        .plan
        .ok_or_else(|| MoltenError::invalid_harness(format!("DAG-sync planning denied: {:?}", result.issues)))?;
    let authority = ports.authority.observe_authority(&plan)?;
    validate_authority(&authority, &plan)?;
    let resources = ports.resources.reserve(&plan)?;
    validate_resources(&resources, &plan)?;
    let mut progress = request.progress.unwrap_or_else(|| initial_progress(&plan, &request.policy_ref));
    let mut terminal_issue = None;
    let mut evidence_refs = vec![authority.authority_ref.clone(), resources.reservation_ref.clone()];

    for fetch in &plan.requests {
        if progress.verified.contains(&fetch.object_ref) {
            continue;
        }
        let envelope = match received_envelope(ports.transport.request(fetch)?)? {
            Ok(envelope) => envelope,
            Err(issue) => {
                terminal_issue = Some(issue);
                break;
            }
        };
        validate_envelope(&envelope, fetch)?;
        evidence_refs.push(envelope.transport_observation_ref.clone());
        let response = ports.content.verify(&plan, &envelope, &authority.authority_ref)?;
        let next = admit_dag_response(&plan, &progress, &response)
            .map_err(|issue| MoltenError::invalid_harness(format!("DAG response denied: {issue:?}")))?;
        let canonical_response = canonical_dag_response(&response)?;
        ports.observations.publish_response(&canonical_response)?;
        evidence_refs.push(canonical_response.record_ref);
        let durable_ref = ports.progress.store(&next)?;
        validate_ref(&durable_ref, "DAG durable progress")?;
        evidence_refs.push(durable_ref);
        let canonical_progress = canonical_dag_progress(&next)?;
        ports.observations.publish_progress(&canonical_progress)?;
        evidence_refs.push(canonical_progress.record_ref);
        progress = next;
    }

    let receipt = sync_receipt(&plan, &progress, terminal_issue);
    let canonical_receipt = canonical_dag_receipt(&receipt)?;
    ports.receipts.publish_receipt(&canonical_receipt)?;
    evidence_refs.push(canonical_receipt.record_ref.clone());
    evidence_refs.sort();
    evidence_refs.dedup();
    Ok(DagSyncOutcome {
        plan,
        progress,
        receipt,
        authority_ref: authority.authority_ref,
        resource_ref: resources.reservation_ref,
        evidence_refs,
        canonical_receipt,
    })
}

fn initial_progress(plan: &DagSyncPlan, policy_ref: &DagPolicyRef) -> DagSyncProgress {
    DagSyncProgress {
        epoch_ref: plan.epoch_ref.clone(),
        generation: plan.generation,
        strategy: plan.strategy,
        policy_ref: policy_ref.clone(),
        root_refs: plan.roots.clone(),
        schema_refs: plan.schema_refs.clone(),
        peers: plan.peers.clone(),
        verified: Vec::new(),
        steps_completed: 0,
    }
}

/// The received envelope, or the terminal issue for a validated deferral or cancellation.
fn received_envelope(outcome: DagTransferOutcome) -> Result<std::result::Result<DagTransportEnvelope, DagSyncIssue>> {
    match outcome {
        DagTransferOutcome::Received(envelope) => Ok(Ok(envelope)),
        DagTransferOutcome::Deferred(observation_ref) => {
            validate_ref(&observation_ref, "DAG transfer deferral")?;
            Ok(Err(DagSyncIssue::TransferDeferred))
        }
        DagTransferOutcome::Cancelled(observation_ref) => {
            validate_ref(&observation_ref, "DAG transfer cancellation")?;
            Ok(Err(DagSyncIssue::TransferCancelled))
        }
    }
}

/// A complete receipt when every missing object was verified, and a partial one otherwise.
fn sync_receipt(
    plan: &DagSyncPlan,
    progress: &DagSyncProgress,
    terminal_issue: Option<DagSyncIssue>,
) -> DagSyncReceipt {
    let missing = plan
        .missing
        .iter()
        .filter(|object| !progress.verified.contains(object))
        .cloned()
        .collect::<Vec<_>>();
    let decision = if missing.is_empty() {
        DagSyncDecision::Complete
    } else {
        DagSyncDecision::Partial
    };
    DagSyncReceipt {
        decision,
        plan_ref: Some(plan.plan_ref.clone()),
        epoch_ref: plan.epoch_ref.clone(),
        generation: plan.generation,
        strategy: plan.strategy,
        requested: plan.requests.len(),
        verified: progress.verified.len(),
        missing,
        issues: terminal_issue.into_iter().collect::<Vec<_>>(),
        non_claims: DAG_SYNC_NON_CLAIMS.iter().map(ToString::to_string).collect(),
    }
}

fn validate_authority(observation: &DagAuthorityObservation, plan: &DagSyncPlan) -> Result<()> {
    validate_ref(&observation.authority_ref, "DAG authority observation")?;
    if !observation.admitted
        || observation.plan_ref != plan.plan_ref
        || observation.epoch_ref != plan.epoch_ref
        || observation.generation != plan.generation
    {
        return Err(MoltenError::invalid_harness("DAG authority observation denied or drifted"));
    }
    Ok(())
}

fn validate_resources(observation: &DagResourceObservation, plan: &DagSyncPlan) -> Result<()> {
    validate_ref(&observation.reservation_ref, "DAG resource reservation")?;
    if !observation.admitted || observation.plan_ref != plan.plan_ref {
        return Err(MoltenError::invalid_harness("DAG resource reservation denied or drifted"));
    }
    Ok(())
}

// r[impl molten.dag_sync.receiver_driven]
fn validate_envelope(envelope: &DagTransportEnvelope, request: &DagFetchRequest) -> Result<()> {
    validate_ref(&envelope.transport_observation_ref, "DAG transport observation")?;
    if envelope.object_ref != request.object_ref
        || envelope.assigned_peer != request.assigned_peer
        || envelope.encoded_bytes == 0
        || envelope.encoded_bytes > MAX_DAG_BYTES
    {
        return Err(MoltenError::invalid_harness("DAG transport envelope was unsolicited, misassigned, or over-bound"));
    }
    Ok(())
}

fn validate_ref(reference: &str, field: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|_| MoltenError::invalid_harness(format!("{field} is not a canonical content reference")))
}
