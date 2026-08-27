use molten_core::world_promotion::*;
use redb::ReadableTable;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::world_promotion::canonical_attempt;
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

pub(super) fn list_reservations(database: &redb::Database) -> Result<Vec<WorldReleaseReservation>> {
    let read = database.begin_read().map_err(store_error)?;
    let table = read.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
    let mut reservations = Vec::with_capacity(MAX_WORLD_PROMOTION_INTENTS);
    for entry in table.iter().map_err(store_error)? {
        if reservations.len() >= MAX_WORLD_PROMOTION_INTENTS {
            return Err(MoltenError::invalid_harness("world reservation inventory exceeds its bound"));
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
        return Err(MoltenError::invalid_harness("only a committed world reservation can be claimed"));
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
    let current = {
        let table = write.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
        table
            .get(reservation.reservation_ref.as_str())
            .map_err(store_error)?
            .map(|guard| guard.value().to_vec())
    }
    .ok_or_else(|| MoltenError::invalid_harness("world reservation is missing"))?;
    let observed = parse_reservation(&current)?;
    if !allowed_transition(observed.state, reservation.state) {
        return Err(MoltenError::invalid_harness("world reservation state transition is invalid"));
    }
    let canonical = canonical_reservation(reservation)?;
    {
        let mut table = write.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
        table
            .insert(reservation.reservation_ref.as_str(), canonical.bytes.as_slice())
            .map_err(store_error)?;
    }
    write.commit().map_err(store_error)
}

pub(super) fn store_attempt(database: &redb::Database, attempt: &WorldAttemptRecord) -> Result<()> {
    let canonical = canonical_attempt(attempt)?;
    let write = database.begin_write().map_err(store_error)?;
    {
        let mut table = write.open_table(ATTEMPTS_TABLE).map_err(store_error)?;
        if let Some(existing) = table.get(attempt.attempt_ref.as_str()).map_err(store_error)? {
            if existing.value() == canonical.bytes.as_slice() {
                return Ok(());
            }
            let observed = parse_attempt(existing.value())?;
            if observed.reservation_ref != attempt.reservation_ref
                || !allowed_attempt_transition(observed.state, attempt.state)
            {
                return Err(MoltenError::invalid_harness("world attempt record conflicts with durable bytes"));
            }
        }
        table.insert(attempt.attempt_ref.as_str(), canonical.bytes.as_slice()).map_err(store_error)?;
    }
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
