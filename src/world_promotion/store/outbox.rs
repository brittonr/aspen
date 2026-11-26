use molten_core::world_head::WorldBranchId;
use molten_core::world_promotion::*;
use redb::ReadableTable;

use super::*;
use crate::error::Failure;
use crate::error::Result;
use crate::world_head::parse_canonical_world_head_state;
use crate::world_promotion::WorldOperatorMutation;
use crate::world_promotion::canonical_attempt;
use crate::world_promotion::canonical_promotion_grant;
use crate::world_promotion::canonical_reservation;
use crate::world_promotion::parse_attempt;
use crate::world_promotion::parse_reservation;

pub(super) fn read_reservation(
    database: &redb::Database,
    reservation_ref: &WorldReleaseReservationRef,
) -> Result<Option<WorldReleaseReservation>> {
    let read = database.begin_read().map_err(store_error)?;
    let table = read.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
    table
        .get(reservation_ref.as_str())
        .map_err(store_error)?
        .map(|guard| parse_reservation(guard.value()))
        .transpose()
}

/// Every stored attempt in attempt-reference order. Dispatch records one attempt per reservation
/// (its initial attempt identity), so the reservation bound also bounds this scan's result.
pub(super) fn list_attempts(database: &redb::Database) -> Result<Vec<WorldAttemptRecord>> {
    let read = database.begin_read().map_err(store_error)?;
    let table = read.open_table(ATTEMPTS_TABLE).map_err(store_error)?;
    let mut attempts = Vec::with_capacity(MAX_WORLD_PROMOTION_INTENTS);
    for entry in table.iter().map_err(store_error)? {
        if attempts.len() >= MAX_WORLD_PROMOTION_INTENTS {
            return Err(Failure::invalid_harness("world attempts exceed the reservation bound"));
        }
        let (_key, value) = entry.map_err(store_error)?;
        attempts.push(parse_attempt(value.value())?);
    }
    Ok(attempts)
}

pub(super) fn list_reservations(database: &redb::Database) -> Result<Vec<WorldReleaseReservation>> {
    let read = database.begin_read().map_err(store_error)?;
    let table = read.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
    let mut reservations = Vec::with_capacity(MAX_WORLD_PROMOTION_INTENTS);
    for entry in table.iter().map_err(store_error)? {
        if reservations.len() >= MAX_WORLD_PROMOTION_INTENTS {
            return Err(Failure::invalid_harness("world reservation inventory exceeds its bound"));
        }
        let (_key, value) = entry.map_err(store_error)?;
        reservations.push(parse_reservation(value.value())?);
    }
    reservations.sort_by(|left, right| left.reservation_ref.cmp(&right.reservation_ref));
    Ok(reservations)
}

pub(super) fn claim_reservation(
    database: &redb::Database,
    reservation_ref: &WorldReleaseReservationRef,
) -> Result<Option<WorldReleaseReservation>> {
    let write = database.begin_write().map_err(store_error)?;
    let current = {
        let table = write.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
        table.get(reservation_ref.as_str()).map_err(store_error)?.map(|guard| guard.value().to_vec())
    };
    let Some(bytes) = current else {
        return Ok(None);
    };
    let mut reservation = parse_reservation(&bytes)?;
    if reservation.state == WorldReleaseState::Claimed {
        return Ok(Some(reservation));
    }
    if reservation.state != WorldReleaseState::Committed {
        return Err(Failure::invalid_harness("only a committed world reservation can be claimed"));
    }
    reservation.state = WorldReleaseState::Claimed;
    let canonical = canonical_reservation(&reservation)?;
    {
        let mut table = write.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
        table
            .insert(reservation.reservation_ref.as_str(), canonical.bytes.as_slice())
            .map_err(store_error)?;
    }
    write.commit().map_err(store_error)?;
    Ok(Some(reservation))
}

pub(super) fn update_reservation(database: &redb::Database, reservation: &WorldReleaseReservation) -> Result<()> {
    let write = database.begin_write().map_err(store_error)?;
    write_reservation_transition(&write, reservation)?;
    write.commit().map_err(store_error)
}

pub(super) fn store_attempt(database: &redb::Database, attempt: &WorldAttemptRecord) -> Result<()> {
    let write = database.begin_write().map_err(store_error)?;
    write_attempt(&write, attempt)?;
    write.commit().map_err(store_error)
}

/// Writes a reservation transition and its attempt record in one write transaction: either both
/// become durable or neither does.
// r[impl molten.world_commit.promotion_atomic_dispatch_and_readback]
pub(super) fn record_attempt_transition(
    database: &redb::Database,
    reservation: &WorldReleaseReservation,
    attempt: &WorldAttemptRecord,
) -> Result<()> {
    if attempt.reservation_ref != reservation.reservation_ref {
        return Err(Failure::invalid_harness("world attempt names another reservation"));
    }
    let write = database.begin_write().map_err(store_error)?;
    write_reservation_transition(&write, reservation)?;
    write_attempt(&write, attempt)?;
    write.commit().map_err(store_error)
}

pub(super) fn read_attempt(
    database: &redb::Database,
    attempt_ref: &WorldReleaseAttemptRef,
) -> Result<Option<WorldAttemptRecord>> {
    let read = database.begin_read().map_err(store_error)?;
    let table = read.open_table(ATTEMPTS_TABLE).map_err(store_error)?;
    table
        .get(attempt_ref.as_str())
        .map_err(store_error)?
        .map(|guard| parse_attempt(guard.value()))
        .transpose()
}

pub(super) fn observe_live(
    database: &redb::Database,
    branch_id: &WorldBranchId,
    nonce: &WorldPromotionGrantNonce,
) -> Result<WorldPromotionLiveFacts> {
    let read = database.begin_read().map_err(store_error)?;
    let head = {
        let heads = read.open_table(WORLD_HEADS_TABLE).map_err(store_error)?;
        heads
            .get(branch_id.as_str())
            .map_err(store_error)?
            .map(|guard| parse_canonical_world_head_state(guard.value()))
            .transpose()?
    };
    let consumed = read.open_table(CONSUMED_GRANTS_TABLE).map_err(store_error)?;
    Ok(WorldPromotionLiveFacts {
        head,
        grant_nonce_consumed: consumed.get(nonce.as_str()).map_err(store_error)?.is_some(),
    })
}

/// Applies one validated operator mutation in one write transaction. The live head must still be
/// the head the grant was validated against and the grant nonce must still be unconsumed; the
/// nonce is consumed together with the reservation and attempt writes.
// r[impl molten.world_commit.promotion_authority_action_and_canonical_binding]
// r[impl molten.world_commit.promotion_atomic_dispatch_and_readback]
pub(super) fn apply_operator_mutation(database: &redb::Database, mutation: &WorldOperatorMutation) -> Result<()> {
    if let (Some(reservation), Some(attempt)) = (&mutation.reservation, &mutation.attempt)
        && attempt.reservation_ref != reservation.reservation_ref
    {
        return Err(Failure::invalid_harness("world attempt names another reservation"));
    }
    let canonical_grant = canonical_promotion_grant(&mutation.grant)?;
    let write = database.begin_write().map_err(store_error)?;
    let live_head = {
        let heads = write.open_table(WORLD_HEADS_TABLE).map_err(store_error)?;
        heads
            .get(mutation.grant.branch_id.as_str())
            .map_err(store_error)?
            .map(|guard| parse_canonical_world_head_state(guard.value()))
            .transpose()?
    };
    if live_head.as_ref() != Some(&mutation.validated_head) {
        return Err(Failure::invalid_harness(
            "world operator authority denied: the branch head changed after validation",
        ));
    }
    consume_grant(&write, &mutation.grant.nonce, &canonical_grant.bytes)?;
    if let Some(reservation) = &mutation.reservation {
        write_reservation_transition(&write, reservation)?;
    }
    if let Some(attempt) = &mutation.attempt {
        write_attempt(&write, attempt)?;
    }
    write.commit().map_err(store_error)
}

/// Records a grant nonce as consumed by `grant_bytes`; a nonce already consumed is denied.
pub(super) fn consume_grant(
    write: &redb::WriteTransaction,
    nonce: &WorldPromotionGrantNonce,
    grant_bytes: &[u8],
) -> Result<()> {
    let mut consumed = write.open_table(CONSUMED_GRANTS_TABLE).map_err(store_error)?;
    if consumed.get(nonce.as_str()).map_err(store_error)?.is_some() {
        return Err(Failure::invalid_harness(format!("world promotion authority denied: {:?}", [
            WorldPromotionIssue::AuthorityNonceConsumed
        ])));
    }
    consumed.insert(nonce.as_str(), grant_bytes).map_err(store_error)?;
    Ok(())
}

/// Validates one reservation transition against the stored record inside `write`: the record
/// must exist, every identity field must be unchanged, and the state change must be allowed.
// r[impl molten.world_commit.promotion_atomic_dispatch_and_readback]
fn write_reservation_transition(write: &redb::WriteTransaction, reservation: &WorldReleaseReservation) -> Result<()> {
    let mut table = write.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
    let current = table
        .get(reservation.reservation_ref.as_str())
        .map_err(store_error)?
        .map(|guard| guard.value().to_vec())
        .ok_or_else(|| Failure::invalid_harness("world reservation is missing"))?;
    let observed = parse_reservation(&current)?;
    if !is_same_reservation_identity(&observed, reservation) {
        return Err(Failure::invalid_harness("world reservation identity fields are immutable"));
    }
    if !allowed_transition(observed.state, reservation.state) {
        return Err(Failure::invalid_harness("world reservation state transition is invalid"));
    }
    let canonical = canonical_reservation(reservation)?;
    table
        .insert(reservation.reservation_ref.as_str(), canonical.bytes.as_slice())
        .map_err(store_error)?;
    Ok(())
}

fn write_attempt(write: &redb::WriteTransaction, attempt: &WorldAttemptRecord) -> Result<()> {
    let canonical = canonical_attempt(attempt)?;
    let mut table = write.open_table(ATTEMPTS_TABLE).map_err(store_error)?;
    let existing = table.get(attempt.attempt_ref.as_str()).map_err(store_error)?.map(|guard| guard.value().to_vec());
    if let Some(existing) = existing {
        if existing == canonical.bytes {
            return Ok(());
        }
        let observed = parse_attempt(&existing)?;
        if observed.reservation_ref != attempt.reservation_ref
            || !allowed_attempt_transition(observed.state, attempt.state)
        {
            return Err(Failure::invalid_harness("world attempt record conflicts with durable bytes"));
        }
    }
    table.insert(attempt.attempt_ref.as_str(), canonical.bytes.as_slice()).map_err(store_error)?;
    Ok(())
}

/// True when two reservation records differ at most in their mutable state.
pub(super) fn is_same_reservation_identity(left: &WorldReleaseReservation, right: &WorldReleaseReservation) -> bool {
    left.reservation_ref == right.reservation_ref
        && left.promotion_ref == right.promotion_ref
        && left.operation_ref == right.operation_ref
        && left.candidate_head == right.candidate_head
        && left.intent_ref == right.intent_ref
        && left.semantic_ref == right.semantic_ref
        && left.handler_ref == right.handler_ref
        && left.adapter_ref == right.adapter_ref
        && left.generation == right.generation
}

fn allowed_attempt_transition(before: WorldReleaseState, after: WorldReleaseState) -> bool {
    before == after
        || matches!(
            (before, after),
            (WorldReleaseState::Attempting, WorldReleaseState::Observed)
                | (WorldReleaseState::Attempting, WorldReleaseState::Uncertain)
                | (WorldReleaseState::Attempting, WorldReleaseState::Conflict)
                | (WorldReleaseState::Observed, WorldReleaseState::Acknowledged)
                | (WorldReleaseState::Uncertain, WorldReleaseState::Reconciled)
                | (WorldReleaseState::Uncertain, WorldReleaseState::Abandoned)
        )
}

fn allowed_transition(before: WorldReleaseState, after: WorldReleaseState) -> bool {
    before == after
        || matches!(
            (before, after),
            (WorldReleaseState::Committed, WorldReleaseState::Claimed)
                | (WorldReleaseState::Committed, WorldReleaseState::Blocked)
                | (WorldReleaseState::Committed, WorldReleaseState::Denied)
                | (WorldReleaseState::Claimed, WorldReleaseState::Attempting)
                | (WorldReleaseState::Claimed, WorldReleaseState::Blocked)
                | (WorldReleaseState::Attempting, WorldReleaseState::Observed)
                | (WorldReleaseState::Attempting, WorldReleaseState::Uncertain)
                | (WorldReleaseState::Attempting, WorldReleaseState::Conflict)
                | (WorldReleaseState::Observed, WorldReleaseState::Acknowledged)
                | (WorldReleaseState::Uncertain, WorldReleaseState::Reconciled)
                | (WorldReleaseState::Uncertain, WorldReleaseState::Abandoned)
                | (WorldReleaseState::Blocked, WorldReleaseState::Denied)
        )
}
