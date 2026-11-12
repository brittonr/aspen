use std::path::Path;

use molten::error::Failure;
use molten::error::Result;
use molten::world_promotion::AbandonInput;
use molten::world_promotion::LocalPromotionReceipts;
use molten::world_promotion::WorldPromotionBookkeepingPort;
use molten::world_promotion::WorldPromotionReceiptPort;
use molten::world_promotion::abandon_uncertain_for_attempt;
use molten::world_promotion::read_operator_authority;
use molten_core::world_promotion::WorldReleaseAttemptRef;
use molten_core::world_promotion::WorldReleaseReservationRef;
use molten_core::world_promotion::denied_reservation;

pub(super) struct DenyArguments<'a> {
    pub state_root: &'a Path,
    pub reservation_ref: &'a str,
    pub authority: &'a Path,
    pub trusted_signer: &'a str,
}

pub(super) fn deny(arguments: DenyArguments<'_>) -> Result<()> {
    let mut store = super::open_store(arguments.state_root)?;
    let reservation_ref =
        WorldReleaseReservationRef::new(arguments.reservation_ref.to_string()).map_err(super::reference_error)?;
    let reservation = store
        .read_reservation(&reservation_ref)?
        .ok_or_else(|| Failure::invalid_harness("world deny reservation is missing"))?;
    let authority = read_operator_authority(arguments.authority, arguments.trusted_signer)?;
    if !authority.admitted || authority.generation != reservation.generation {
        return Err(Failure::invalid_harness("world deny authority is not current or admitted"));
    }
    let denied = denied_reservation(&reservation);
    store.update_reservation(&denied)?;
    let canonical = molten::world_promotion::canonical_reservation(&denied)?;
    let mut receipts = LocalPromotionReceipts::open(arguments.state_root)?;
    receipts.publish_promotion_receipt(&canonical)?;
    println!("reservation_ref={}", denied.reservation_ref);
    println!("reservation_state={}", denied.state.as_str());
    println!("receipt_ref={}", canonical.record_ref);
    println!("decision=denied");
    Ok(())
}

pub(super) struct AbandonArguments<'a> {
    pub state_root: &'a Path,
    pub request_path: &'a Path,
    pub attempt_ref: &'a str,
    pub is_unknown_outcome_acknowledged: bool,
}

pub(super) fn abandon(arguments: AbandonArguments<'_>) -> Result<()> {
    let mut store = super::open_store(arguments.state_root)?;
    let request = super::read_request(arguments.request_path)?;
    let attempt_ref = WorldReleaseAttemptRef::new(arguments.attempt_ref.to_string()).map_err(super::reference_error)?;
    let abandoned = abandon_uncertain_for_attempt(AbandonInput {
        store: &mut store,
        request: &request,
        attempt_ref: &attempt_ref,
        is_unknown_outcome_acknowledged: arguments.is_unknown_outcome_acknowledged,
    })?;
    let canonical = molten::world_promotion::canonical_observation(&abandoned)?;
    let mut receipts = LocalPromotionReceipts::open(arguments.state_root)?;
    receipts.publish_promotion_receipt(&canonical)?;
    println!("reservation_ref={}", abandoned.reservation_ref);
    println!("attempt_ref={}", abandoned.attempt_ref);
    println!("attempt_state={}", abandoned.state.as_str());
    println!("external_completion_proven=false");
    println!("receipt_ref={}", canonical.record_ref);
    println!("decision=abandoned");
    Ok(())
}
