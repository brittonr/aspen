// The publication eligibility gate of the replication shell.
//
// The requirement is the profile's replica policy. The observations are the
// receivers that hold the published content, the fault domains they cover, the
// retention generation, and the placement epoch. The storage guarantee names
// the fault-domain spread, so a replica count alone never satisfies the gate.

use molten_core::content_replication::*;

const MAX_ELIGIBILITY_RECEIVERS: usize = 64;

/// The publication eligibility receipts for the published content.
pub(super) fn publication_eligibility(
    manifest: &Manifest,
    plan: &Plan,
    inventory: &Inventory,
    operations: &[PriorOperation],
) -> crate::error::Result<Vec<PublicationReceipt>> {
    let mut receipts = Vec::with_capacity(plan.replica_deficits.len());
    for deficit in &plan.replica_deficits {
        let mut receivers: Vec<String> = Vec::with_capacity(MAX_ELIGIBILITY_RECEIVERS);
        let mut domains: Vec<String> = Vec::with_capacity(MAX_ELIGIBILITY_RECEIVERS);
        for replica in &inventory.replicas {
            if replica.content_ref == deficit.content_ref && replica.present && replica.identity_verified {
                receivers.push(replica.peer_id.clone());
                domains.push(replica.fault_domain.clone());
            }
        }
        let mut operation_id = String::new();
        for action in &plan.actions {
            if action.content_ref != deficit.content_ref || action.kind == ActionKind::Cleanup {
                continue;
            }
            let is_verified = operations.iter().any(|operation| {
                operation.operation_id == action.operation_id && operation.outcome == OperationOutcome::Verified
            });
            if is_verified {
                receivers.push(action.target_peer.clone());
                domains.push(action.fault_domain.clone());
                operation_id = action.operation_id.clone();
                break;
            }
        }
        if operation_id.is_empty() {
            continue;
        }
        receivers.sort();
        receivers.dedup();
        domains.sort();
        domains.dedup();
        let guarantee = if manifest.policy.minimum_verified_replicas > 1 || manifest.policy.minimum_fault_domains > 1 {
            StorageGuaranteeKind::ReplicatedAcrossFaultDomains {
                replicas: receivers.len(),
                fault_domains: domains.len(),
            }
        } else {
            StorageGuaranteeKind::Local
        };
        let observations = PublicationObservations {
            content_ref: deficit.content_ref.clone(),
            operation_id,
            receiver_set: receivers,
            retention_generation: manifest.generation,
            placement_epoch: manifest.placement_epoch,
            storage_guarantee: guarantee,
        };
        let receipt = check_publication(&manifest.policy, &observations).map_err(|issue| {
            crate::error::Failure::invalid_harness(format!(
                "publication eligibility denied for {}: {}",
                observations.content_ref,
                issue.as_str()
            ))
        })?;
        receipts.push(receipt);
    }
    Ok(receipts)
}
