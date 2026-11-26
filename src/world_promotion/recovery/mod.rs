//! Recovery of release attempts whose outcome a previous process did not record.

use molten_core::world_promotion::*;

use super::*;
use crate::error::Failure;
use crate::error::Result;

/// Classify attempts a previous process left `attempting` (it stopped between storing the attempt
/// and recording the dispatcher's result) as uncertain, so retry and abandon apply. The local store
/// is held by one process at a time, so no dispatcher can still own such an attempt.
///
/// Stores written before reservation and attempt transitions shared one write transaction can also
/// hold pairs that no single transition left behind: an abandoned attempt whose reservation stayed
/// `uncertain` (or `attempting`) is settled to `abandoned`; an `attempting` reservation whose
/// attempt row was never written gains an uncertain attempt; and one whose attempt already
/// recorded a result takes that result's state.
// r[impl molten.world_promotion.reconciliation]
// r[impl molten.world_commit.promotion_atomic_dispatch_and_readback]
pub fn recover_interrupted_attempts<T, R>(transaction: &mut T, receipts: &mut R) -> Result<Vec<WorldAttemptRecord>>
where
    T: WorldPromotionTransactionPort,
    R: WorldPromotionReceiptPort,
{
    let attempts = transaction.list_attempts()?;
    let mut recovered = Vec::with_capacity(MAX_WORLD_PROMOTION_INTENTS);
    for attempt in &attempts {
        if !matches!(attempt.state, WorldReleaseState::Attempting | WorldReleaseState::Abandoned) {
            continue;
        }
        let reservation = transaction
            .read_reservation(&attempt.reservation_ref)?
            .ok_or_else(|| Failure::invalid_harness("an unsettled world attempt has no reservation"))?;
        if attempt.state == WorldReleaseState::Attempting {
            recovered.push(record_uncertain_attempt(&reservation, &attempt.attempt_ref, transaction, receipts)?);
        } else if matches!(reservation.state, WorldReleaseState::Uncertain | WorldReleaseState::Attempting) {
            settle_abandoned(reservation, attempt, transaction, receipts)?;
            recovered.push(attempt.clone());
        }
    }
    for reservation in transaction.list_reservations()? {
        if reservation.state == WorldReleaseState::Attempting {
            recovered.push(settle_attempting_reservation(reservation, transaction, receipts)?);
        }
    }
    Ok(recovered)
}

/// Moves the reservation of an already abandoned attempt to `abandoned`, through `uncertain` when
/// it is still `attempting`, so it matches its attempt.
fn settle_abandoned<T, R>(
    mut reservation: WorldReleaseReservation,
    attempt: &WorldAttemptRecord,
    transaction: &mut T,
    receipts: &mut R,
) -> Result<()>
where
    T: WorldPromotionTransactionPort,
    R: WorldPromotionReceiptPort,
{
    if reservation.state == WorldReleaseState::Attempting {
        reservation.state = WorldReleaseState::Uncertain;
        transaction.record_attempt_transition(&reservation, attempt)?;
    }
    reservation.state = WorldReleaseState::Abandoned;
    transaction.record_attempt_transition(&reservation, attempt)?;
    receipts.publish_promotion_receipt(&canonical_observation(attempt)?)
}

/// Settles an `attempting` reservation that has no in-progress attempt against its initial attempt.
fn settle_attempting_reservation<T, R>(
    reservation: WorldReleaseReservation,
    transaction: &mut T,
    receipts: &mut R,
) -> Result<WorldAttemptRecord>
where
    T: WorldPromotionTransactionPort,
    R: WorldPromotionReceiptPort,
{
    let attempt_ref = initial_attempt_ref(&reservation)
        .map_err(|issues| Failure::invalid_harness(format!("world attempt identity failed: {issues:?}")))?;
    match transaction.read_attempt(&attempt_ref)? {
        None => record_uncertain_attempt(&reservation, &attempt_ref, transaction, receipts),
        Some(attempt)
            if attempt.reservation_ref == reservation.reservation_ref
                && matches!(
                    attempt.state,
                    WorldReleaseState::Observed | WorldReleaseState::Uncertain | WorldReleaseState::Conflict
                ) =>
        {
            let mut settled = reservation;
            settled.state = attempt.state;
            transaction.record_attempt_transition(&settled, &attempt)?;
            receipts.publish_promotion_receipt(&canonical_observation(&attempt)?)?;
            Ok(attempt)
        }
        Some(_) => Err(Failure::invalid_harness(
            "an attempting world reservation has an attempt record in an unrecoverable state",
        )),
    }
}

/// The external effect's outcome is unknown: record the attempt and its reservation as uncertain
/// in one write transaction and publish that observation receipt.
pub(super) fn record_uncertain_attempt<T, R>(
    reservation: &WorldReleaseReservation,
    attempt_ref: &WorldReleaseAttemptRef,
    transaction: &mut T,
    receipts: &mut R,
) -> Result<WorldAttemptRecord>
where
    T: WorldPromotionTransactionPort,
    R: WorldPromotionReceiptPort,
{
    let attempt = WorldAttemptRecord {
        reservation_ref: reservation.reservation_ref.clone(),
        attempt_ref: attempt_ref.clone(),
        state: WorldReleaseState::Uncertain,
        observation_ref: None,
        outcome: WorldAttemptOutcome::Unknown,
        external_completion_proven: false,
    };
    let mut uncertain = reservation.clone();
    if reservation.state == WorldReleaseState::Attempting {
        uncertain.state = WorldReleaseState::Uncertain;
    }
    transaction.record_attempt_transition(&uncertain, &attempt)?;
    receipts.publish_promotion_receipt(&canonical_observation(&attempt)?)?;
    Ok(attempt)
}
