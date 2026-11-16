use molten_core::world_promotion::*;
use transactional_reconciliation_core::QuarantineStatus;

use super::*;
use crate::error::Failure;
use crate::error::Result;

pub struct WorldPromotionPorts<'a, C, T, R> {
    pub current: &'a mut C,
    pub transaction: &'a mut T,
    pub receipts: &'a mut R,
}

#[derive(Debug, Clone)]
pub struct WorldPromotionOutcome {
    pub plan: WorldPromotionPlan,
    pub committed_reservations: Vec<WorldReleaseReservation>,
    pub persistence: WorldPromotionPersistence,
    pub canonical_plan: CanonicalWorldPromotionRecord,
    pub canonical_reservations: Vec<CanonicalWorldPromotionRecord>,
    pub canonical_receipt: CanonicalWorldPromotionRecord,
    pub replayed: bool,
}

// r[impl molten.world_promotion.transaction]
// r[impl molten.world_promotion.reconciliation]
pub fn promote_world<C, T, R>(
    request: &WorldPromotionRequest,
    ports: WorldPromotionPorts<'_, C, T, R>,
) -> Result<WorldPromotionOutcome>
where
    C: WorldPromotionCurrentPort,
    T: WorldPromotionTransactionPort,
    R: WorldPromotionReceiptPort,
{
    let plan = plan_world_promotion(request)
        .map_err(|issues| Failure::invalid_harness(format!("world promotion planning denied: {issues:?}")))?;
    let canonical_plan = canonical_promotion_plan(&plan)?;
    let committed_reservations = plan
        .reservations
        .iter()
        .cloned()
        .map(|mut reservation| {
            reservation.state = WorldReleaseState::Committed;
            reservation
        })
        .collect::<Vec<_>>();
    let canonical_reservations =
        committed_reservations.iter().map(canonical_reservation).collect::<Result<Vec<_>>>()?;
    let stored_identity = ports.transaction.read_promotion_identity(&plan.operation_ref)?;
    let decision = match &stored_identity {
        None => WorldPromotionRetryDecision::New,
        Some(StoredPromotionIdentity::Current(existing)) => evaluate_promotion_retry(&plan.identity, Some(existing)),
        Some(StoredPromotionIdentity::Legacy { .. }) => WorldPromotionRetryDecision::Replay,
    };
    if decision == WorldPromotionRetryDecision::IncompatibleRequest {
        return Err(Failure::invalid_harness(format!(
            "promotion identity conflict for operation {}",
            plan.operation_ref
        )));
    }
    let (persistence, replayed) = if decision == WorldPromotionRetryDecision::Replay {
        let observation = replay_observation(&plan, &stored_identity, ports.transaction)?;
        let persistence = classify_promotion_commit(&plan, &observation).map_err(|issues| {
            Failure::invalid_harness(format!("promotion replay classification failed: {issues:?}"))
        })?;
        (persistence, true)
    } else {
        let persistence =
            commit_persistence(&plan, &canonical_plan, &canonical_reservations, ports.current, ports.transaction)?;
        (persistence, false)
    };
    let canonical_receipt = canonical_persistence(&plan, &persistence)?;
    ports.receipts.publish_promotion_receipt(&canonical_receipt)?;
    Ok(WorldPromotionOutcome {
        plan,
        committed_reservations,
        persistence,
        canonical_plan,
        canonical_reservations,
        canonical_receipt,
        replayed,
    })
}

fn replay_observation<T: WorldPromotionTransactionPort>(
    plan: &WorldPromotionPlan,
    stored_identity: &Option<StoredPromotionIdentity>,
    transaction: &T,
) -> Result<WorldPromotionCommitObservation> {
    if matches!(stored_identity, Some(StoredPromotionIdentity::Legacy { .. })) {
        return Ok(WorldPromotionCommitObservation::OutcomeUnknown);
    }
    Ok(match transaction.read_back_promotion(plan)? {
        WorldPromotionReadBackObservation::Reservation => WorldPromotionCommitObservation::Applied,
        WorldPromotionReadBackObservation::Prior { head, generation } => WorldPromotionCommitObservation::NotApplied {
            current_head: head,
            current_generation: generation,
        },
        WorldPromotionReadBackObservation::Missing | WorldPromotionReadBackObservation::Corrupt => {
            WorldPromotionCommitObservation::OutcomeUnknown
        }
    })
}

fn commit_persistence<C, T>(
    plan: &WorldPromotionPlan,
    canonical_plan: &CanonicalWorldPromotionRecord,
    canonical_reservations: &[CanonicalWorldPromotionRecord],
    current: &mut C,
    transaction: &mut T,
) -> Result<WorldPromotionPersistence>
where
    C: WorldPromotionCurrentPort,
    T: WorldPromotionTransactionPort,
{
    let facts = current.observe_transaction(plan)?;
    let observation = transaction.commit_promotion(plan, canonical_plan, canonical_reservations, &facts)?;
    let mut persistence = classify_promotion_commit(plan, &observation)
        .map_err(|issues| Failure::invalid_harness(format!("promotion commit classification failed: {issues:?}")))?;
    if persistence.shared.quarantine() != QuarantineStatus::Clear {
        let read_back = transaction.read_back_promotion(plan)?;
        persistence = reconcile_promotion_read_back(plan, &persistence, &read_back)
            .map_err(|issues| Failure::invalid_harness(format!("promotion readback failed: {issues:?}")))?;
    }
    Ok(persistence)
}

pub struct WorldDispatchPorts<'a, T, A, D, R> {
    pub transaction: &'a mut T,
    pub admission: &'a mut A,
    pub dispatcher: &'a mut D,
    pub receipts: &'a mut R,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldDispatchDecision {
    Dispatched,
    Blocked,
}

#[derive(Debug, Clone)]
pub struct WorldDispatchOutcome {
    pub decision: WorldDispatchDecision,
    pub reservation: WorldReleaseReservation,
    pub attempt: Option<WorldAttemptRecord>,
    pub issue_codes: Vec<String>,
    pub canonical_receipt: CanonicalWorldPromotionRecord,
}

// r[impl molten.world_promotion.dispatch]
pub fn dispatch_world_reservation<T, A, D, R>(
    plan: &WorldPromotionPlan,
    reservation_ref: &WorldReleaseReservationRef,
    attempt_ref: &WorldReleaseAttemptRef,
    ports: WorldDispatchPorts<'_, T, A, D, R>,
) -> Result<WorldDispatchOutcome>
where
    T: WorldPromotionTransactionPort,
    A: WorldEffectAdmissionPort,
    D: WorldEffectDispatcherPort,
    R: WorldPromotionReceiptPort,
{
    let reservation = ports
        .transaction
        .claim_reservation(reservation_ref)?
        .ok_or_else(|| Failure::invalid_harness("world reservation does not exist"))?;
    let facts = ports.admission.observe_dispatch(plan, &reservation)?;
    let dispatch = match plan_world_dispatch(plan, &reservation, &facts, attempt_ref) {
        Ok(dispatch) => dispatch,
        Err(issues) => return block_dispatch(reservation, issues, ports.transaction, ports.receipts),
    };
    let mut attempting_reservation = reservation;
    attempting_reservation.state = WorldReleaseState::Attempting;
    ports.transaction.update_reservation(&attempting_reservation)?;
    let attempting = WorldAttemptRecord {
        reservation_ref: attempting_reservation.reservation_ref.clone(),
        attempt_ref: dispatch.attempt_ref.clone(),
        state: WorldReleaseState::Attempting,
        observation_ref: None,
        outcome: WorldAttemptOutcome::Unknown,
        external_completion_proven: false,
    };
    ports.transaction.store_attempt(&attempting)?;
    let observation = match ports.dispatcher.dispatch(&dispatch) {
        Ok(observation) => observation,
        Err(error) => {
            return Err(
                match record_uncertain_attempt(
                    &attempting_reservation,
                    &dispatch.attempt_ref,
                    ports.transaction,
                    ports.receipts,
                ) {
                    Ok(_) => error,
                    Err(recording_error) => Failure::invalid_harness(format!(
                        "world dispatch failed: {error}; recording its uncertain outcome also failed: {recording_error}"
                    )),
                },
            );
        }
    };
    let attempt = classify_attempt_observation(&dispatch, observation);
    ports.transaction.store_attempt(&attempt)?;
    let mut terminal_reservation = attempting_reservation;
    terminal_reservation.state = attempt.state;
    ports.transaction.update_reservation(&terminal_reservation)?;
    let canonical_receipt = canonical_observation(&attempt)?;
    ports.receipts.publish_promotion_receipt(&canonical_receipt)?;
    Ok(WorldDispatchOutcome {
        decision: WorldDispatchDecision::Dispatched,
        reservation: terminal_reservation,
        attempt: Some(attempt),
        issue_codes: Vec::new(),
        canonical_receipt,
    })
}

/// Classify attempts a previous process left `attempting` (it stopped between storing the attempt
/// and recording the dispatcher's result) as uncertain, so retry and abandon apply. The local store
/// is held by one process at a time, so no dispatcher can still own such an attempt.
// r[impl molten.world_promotion.reconciliation]
pub fn recover_interrupted_attempts<T, R>(transaction: &mut T, receipts: &mut R) -> Result<Vec<WorldAttemptRecord>>
where
    T: WorldPromotionTransactionPort,
    R: WorldPromotionReceiptPort,
{
    let attempts = transaction.list_attempts_in_progress()?;
    let mut recovered = Vec::with_capacity(attempts.len());
    for attempt in attempts {
        let reservation = transaction
            .read_reservation(&attempt.reservation_ref)?
            .ok_or_else(|| Failure::invalid_harness("an in-progress world attempt has no reservation"))?;
        recovered.push(record_uncertain_attempt(&reservation, &attempt.attempt_ref, transaction, receipts)?);
    }
    Ok(recovered)
}

/// The external effect's outcome is unknown: record the attempt and its reservation as uncertain
/// and publish that observation receipt.
fn record_uncertain_attempt<T, R>(
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
    transaction.store_attempt(&attempt)?;
    if reservation.state == WorldReleaseState::Attempting {
        let mut uncertain = reservation.clone();
        uncertain.state = WorldReleaseState::Uncertain;
        transaction.update_reservation(&uncertain)?;
    }
    receipts.publish_promotion_receipt(&canonical_observation(&attempt)?)?;
    Ok(attempt)
}

fn block_dispatch<T, R>(
    reservation: WorldReleaseReservation,
    issues: Vec<WorldPromotionIssue>,
    transaction: &mut T,
    receipts: &mut R,
) -> Result<WorldDispatchOutcome>
where
    T: WorldPromotionTransactionPort,
    R: WorldPromotionReceiptPort,
{
    let blocked = blocked_reservation(&reservation);
    transaction.update_reservation(&blocked)?;
    let canonical_receipt = canonical_reservation(&blocked)?;
    receipts.publish_promotion_receipt(&canonical_receipt)?;
    Ok(WorldDispatchOutcome {
        decision: WorldDispatchDecision::Blocked,
        reservation: blocked,
        attempt: None,
        issue_codes: issues.into_iter().map(|issue| format!("{issue:?}")).collect(),
        canonical_receipt,
    })
}
