// The closure repair projection of the replication shell.
//
// The repair planner works on the required closure set and the bounded
// inventory of per-replica, per-object verification facts, so the operator
// status can report whether the closure is complete.

use molten_core::content_replication::*;

/// The closure repair input over the observed inventory.
///
/// The required closure is the manifest's content set, and the inventory facts
/// are the per-replica, per-object verification facts. Donor eligibility is per
/// object, so a cohort whose members each hold a different intact subset still
/// repairs the whole closure.
pub(super) fn closure_repair_input(manifest: &Manifest, inventory: &Inventory) -> ClosureInput {
    let required_objects = manifest.contents.iter().map(|rule| rule.content_ref.clone()).collect();
    let facts = inventory
        .replicas
        .iter()
        .map(|replica| ReplicaObjectFact {
            peer_id: replica.peer_id.clone(),
            fault_domain: replica.fault_domain.clone(),
            object_ref: replica.content_ref.clone(),
            present: replica.present,
            identity_verified: replica.identity_verified,
        })
        .collect();
    ClosureInput {
        required_objects,
        inventory: facts,
        max_transfers: manifest.contents.len(),
    }
}
