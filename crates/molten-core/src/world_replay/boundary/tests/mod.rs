use super::super::tests::fixture::*;
use super::super::tests::sealed::*;
use super::super::*;

mod admission;
mod evidence;
mod native;

fn admit(request: &WorldReplayPlanRequest) -> Result<ReplayObservationAdmission, Vec<ReplayBoundaryBlocker>> {
    admit_replay_observations(&request.boundary, &request.trace)
}

fn blockers(request: &WorldReplayPlanRequest) -> Vec<ReplayBoundaryBlocker> {
    admit(request).expect_err("the boundary is blocked")
}

/// Apply `change` to the ledger, then re-identify it, so only the changed
/// fact can block.
fn with_ledger(request: &mut WorldReplayPlanRequest, change: impl FnOnce(&mut ReplayObservationLedger)) {
    change(&mut request.boundary.ledger);
    request.boundary.ledger = reidentify_ledger(request.boundary.ledger.clone());
}

/// Apply `change` to the inventory, then re-identify it and rebind the ledger.
fn with_inventory(request: &mut WorldReplayPlanRequest, change: impl FnOnce(&mut ReplaySourceInventory)) {
    change(&mut request.boundary.inventory);
    request.boundary.inventory = reidentify_inventory(request.boundary.inventory.clone());
    let inventory_ref = request.boundary.inventory.inventory_ref.clone();
    with_ledger(request, |ledger| ledger.inventory_ref = inventory_ref);
}

fn first_step(request: &WorldReplayPlanRequest) -> u64 {
    request.trace.steps[0].position
}

fn second_step(request: &WorldReplayPlanRequest) -> u64 {
    request.trace.steps[1].position
}
