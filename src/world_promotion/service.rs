use molten_core::world_promotion::*;
use transactional_reconciliation_core::QuarantineStatus;

use super::*;
use crate::error::MoltenError;
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
        .map_err(|issues| MoltenError::invalid_harness(format!("world promotion planning denied: {issues:?}")))?;
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
    let facts = ports.current.observe_transaction(&plan)?;
    let observation = ports.transaction.commit_promotion(&plan, &canonical_plan, &canonical_reservations, &facts)?;
    let mut persistence = classify_promotion_commit(&plan, &observation).map_err(|issues| {
        MoltenError::invalid_harness(format!("promotion commit classification failed: {issues:?}"))
    })?;
    if persistence.shared.quarantine() != QuarantineStatus::Clear {
        let read_back = ports.transaction.read_back_promotion(&plan)?;
        persistence = reconcile_promotion_read_back(&plan, &persistence, &read_back)
            .map_err(|issues| MoltenError::invalid_harness(format!("promotion readback failed: {issues:?}")))?;
    }
    let canonical_receipt = canonical_persistence(&plan, &persistence)?;
    ports.receipts.publish_promotion_receipt(&canonical_receipt)?;
    Ok(WorldPromotionOutcome {
        plan,
        committed_reservations,
        persistence,
        canonical_plan,
        canonical_reservations,
        canonical_receipt,
    })
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
        .ok_or_else(|| MoltenError::invalid_harness("world reservation does not exist"))?;
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
        external_completion_proven: false,
    };
    ports.transaction.store_attempt(&attempting)?;
    let observation = ports.dispatcher.dispatch(&dispatch)?;
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
