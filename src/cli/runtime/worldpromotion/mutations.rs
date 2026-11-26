use std::path::Path;

use molten::error::Result;
use molten::world_promotion::AbandonInput;
use molten::world_promotion::DenyInput;
use molten::world_promotion::LocalPromotionReceipts;
use molten::world_promotion::WorldPromotionReceiptPort;
use molten::world_promotion::abandon_uncertain_for_attempt;
use molten::world_promotion::deny_reservation;
use molten::world_promotion::read_promotion_grant;
use molten_core::world_promotion::WorldReleaseAttemptRef;
use molten_core::world_promotion::WorldReleaseReservationRef;

pub(super) struct DenyArguments<'a> {
    pub state_root: &'a Path,
    pub request_path: &'a Path,
    pub reservation_ref: &'a str,
    pub authority: &'a Path,
    pub trusted_signer: &'a str,
}

pub(super) fn deny(arguments: DenyArguments<'_>) -> Result<()> {
    let operator = read_promotion_grant(arguments.authority, arguments.trusted_signer)?;
    let mut store = super::open_store(arguments.state_root)?;
    let request = super::read_request(arguments.request_path)?;
    let reservation_ref =
        WorldReleaseReservationRef::new(arguments.reservation_ref.to_string()).map_err(super::reference_error)?;
    let denied = deny_reservation(DenyInput {
        store: &mut store,
        request: &request,
        operator: &operator,
        reservation_ref: &reservation_ref,
        observed_unix_seconds: super::observed_unix_seconds()?,
    })?;
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
    pub authority: &'a Path,
    pub trusted_signer: &'a str,
    pub attempt_ref: &'a str,
    pub is_unknown_outcome_acknowledged: bool,
}

pub(super) fn abandon(arguments: AbandonArguments<'_>) -> Result<()> {
    let operator = read_promotion_grant(arguments.authority, arguments.trusted_signer)?;
    let mut store = super::open_store(arguments.state_root)?;
    let request = super::read_request(arguments.request_path)?;
    let attempt_ref = WorldReleaseAttemptRef::new(arguments.attempt_ref.to_string()).map_err(super::reference_error)?;
    let abandoned = abandon_uncertain_for_attempt(AbandonInput {
        store: &mut store,
        request: &request,
        operator: &operator,
        attempt_ref: &attempt_ref,
        is_unknown_outcome_acknowledged: arguments.is_unknown_outcome_acknowledged,
        observed_unix_seconds: super::observed_unix_seconds()?,
    })?;
    let canonical = molten::world_promotion::canonical_observation(&abandoned.attempt)?;
    let mut receipts = LocalPromotionReceipts::open(arguments.state_root)?;
    receipts.publish_promotion_receipt(&canonical)?;
    println!("reservation_ref={}", abandoned.reservation.reservation_ref);
    println!("reservation_state={}", abandoned.reservation.state.as_str());
    println!("attempt_ref={}", abandoned.attempt.attempt_ref);
    println!("attempt_state={}", abandoned.attempt.state.as_str());
    println!("external_completion_proven=false");
    println!("receipt_ref={}", canonical.record_ref);
    println!("decision=abandoned");
    Ok(())
}
