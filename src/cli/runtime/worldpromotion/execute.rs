use std::path::Path;

use molten::error::Failure;
use molten::error::Result;

pub(super) fn promote(
    state_root: &Path,
    request_path: &Path,
    authority_path: &Path,
    closure_path: &Path,
    trusted_signer: &str,
) -> Result<()> {
    let request = super::read_request(request_path)?;
    let branch_id = request.branch_id.as_str().to_string();
    let mut current = molten::world_promotion::read_observed_current(
        state_root,
        authority_path,
        closure_path,
        trusted_signer,
        &branch_id,
    )?;
    let mut request = request;
    if request.authority.authority_ref != current.authority_ref || request.authority.policy_ref != current.policy_ref {
        return Err(Failure::invalid_harness("promotion request authority does not match the signed authority grant"));
    }
    request.authority.authority_ref = current.authority_ref.clone();
    request.authority.policy_ref = current.policy_ref.clone();
    request.authority.observed_generation = current.authority_generation;
    request.authority.admitted = current.authority_admitted;
    request.policy_ref = current.policy_ref.clone();
    request.intent_closure_complete = current.closure.complete;
    let mut store = super::open_store(state_root)?;
    let mut receipts = molten::world_promotion::LocalPromotionReceipts::open(state_root)?;
    let outcome = molten::world_promotion::promote_world(&request, molten::world_promotion::WorldPromotionPorts {
        current: &mut current,
        transaction: &mut store,
        receipts: &mut receipts,
    })?;
    println!("plan_ref={}", outcome.plan.plan_ref);
    println!("replayed={}", outcome.replayed);
    println!("persistence={:?}", outcome.persistence);
    println!("external_effects_completed=false");
    Ok(())
}
