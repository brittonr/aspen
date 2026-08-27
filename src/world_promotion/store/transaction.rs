use molten_core::world_promotion::*;
use redb::ReadableTable;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::world_head::canonical_world_head_state;
use crate::world_head::parse_canonical_world_head_state;
use crate::world_promotion::CanonicalWorldPromotionRecord;
use crate::world_promotion::WorldPromotionTransactionPort;
use crate::world_promotion::parse_reservation;

impl WorldPromotionTransactionPort for LocalWorldPromotionStore {
    fn commit_promotion(
        &mut self,
        plan: &WorldPromotionPlan,
        canonical_plan: &CanonicalWorldPromotionRecord,
        reservations: &[CanonicalWorldPromotionRecord],
        facts: &WorldPromotionTransactionFacts,
    ) -> Result<WorldPromotionCommitObservation> {
        validate_promotion_transaction(plan, facts).map_err(|issues| {
            MoltenError::invalid_harness(format!("world promotion transaction denied: {issues:?}"))
        })?;
        let parsed = parse_committed_reservations(plan, reservations)?;
        let write = self.database().begin_write().map_err(store_error)?;
        let observed = {
            let heads = write.open_table(WORLD_HEADS_TABLE).map_err(store_error)?;
            heads
                .get(plan.after.branch_id.as_str())
                .map_err(store_error)?
                .map(|guard| parse_canonical_world_head_state(guard.value()))
                .transpose()?
        };
        if observed.as_ref() == Some(&plan.after) {
            let is_complete = reservations_match(&write, &parsed)?;
            return Ok(if is_complete {
                WorldPromotionCommitObservation::Applied
            } else {
                WorldPromotionCommitObservation::Inconsistent
            });
        }
        if observed.as_ref() != Some(&plan.before) {
            return Ok(observed.map_or(WorldPromotionCommitObservation::Inconsistent, |state| {
                WorldPromotionCommitObservation::NotApplied {
                    current_head: state.head,
                    current_generation: state.generation,
                }
            }));
        }
        let (_, state_bytes) = canonical_world_head_state(&plan.after)?;
        {
            let mut heads = write.open_table(WORLD_HEADS_TABLE).map_err(store_error)?;
            heads.insert(plan.after.branch_id.as_str(), state_bytes.as_slice()).map_err(store_error)?;
        }
        {
            let mut promotions = write.open_table(PROMOTIONS_TABLE).map_err(store_error)?;
            promotions.insert(plan.plan_ref.as_str(), canonical_plan.bytes.as_slice()).map_err(store_error)?;
        }
        {
            let mut table = write.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
            for (reservation, canonical) in parsed.iter().zip(reservations) {
                table
                    .insert(reservation.reservation_ref.as_str(), canonical.bytes.as_slice())
                    .map_err(store_error)?;
            }
        }
        Ok(match write.commit() {
            Ok(()) => WorldPromotionCommitObservation::Applied,
            Err(_) => WorldPromotionCommitObservation::OutcomeUnknown,
        })
    }

    fn read_back_promotion(&self, plan: &WorldPromotionPlan) -> Result<WorldPromotionReadBackObservation> {
        let read = self.database().begin_read().map_err(store_error)?;
        let head = {
            let table = read.open_table(WORLD_HEADS_TABLE).map_err(store_error)?;
            table
                .get(plan.after.branch_id.as_str())
                .map_err(store_error)?
                .map(|guard| parse_canonical_world_head_state(guard.value()))
                .transpose()
        };
        let Ok(head) = head else {
            return Ok(WorldPromotionReadBackObservation::Corrupt);
        };
        let reservations = read.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
        let mut present = 0_usize;
        for expected in &plan.reservations {
            let Some(bytes) = reservations
                .get(expected.reservation_ref.as_str())
                .map_err(store_error)?
                .map(|guard| guard.value().to_vec())
            else {
                continue;
            };
            let Ok(observed) = parse_reservation(&bytes) else {
                return Ok(WorldPromotionReadBackObservation::Corrupt);
            };
            if observed.reservation_ref != expected.reservation_ref || observed.state != WorldReleaseState::Committed {
                return Ok(WorldPromotionReadBackObservation::Corrupt);
            }
            present = present.saturating_add(1);
        }
        if head.as_ref() == Some(&plan.after) && present == plan.reservations.len() {
            Ok(WorldPromotionReadBackObservation::Reservation)
        } else if head.as_ref() == Some(&plan.before) && present == 0 {
            Ok(WorldPromotionReadBackObservation::Prior {
                head: plan.before.head.clone(),
                generation: plan.before.generation,
            })
        } else if let Some(head) = head {
            Ok(WorldPromotionReadBackObservation::Prior {
                head: head.head,
                generation: head.generation,
            })
        } else {
            Ok(WorldPromotionReadBackObservation::Missing)
        }
    }

    fn read_reservation(
        &self,
        reservation_ref: &WorldReleaseReservationRef,
    ) -> Result<Option<WorldReleaseReservation>> {
        super::outbox::read_reservation(self.database(), reservation_ref)
    }

    fn list_reservations(&self) -> Result<Vec<WorldReleaseReservation>> {
        super::outbox::list_reservations(self.database())
    }

    fn claim_reservation(
        &mut self,
        reservation_ref: &WorldReleaseReservationRef,
    ) -> Result<Option<WorldReleaseReservation>> {
        super::outbox::claim_reservation(self.database(), reservation_ref)
    }

    fn update_reservation(&mut self, reservation: &WorldReleaseReservation) -> Result<()> {
        super::outbox::update_reservation(self.database(), reservation)
    }

    fn store_attempt(&mut self, attempt: &WorldAttemptRecord) -> Result<()> {
        super::outbox::store_attempt(self.database(), attempt)
    }

    fn read_attempt(&self, attempt_ref: &WorldReleaseAttemptRef) -> Result<Option<WorldAttemptRecord>> {
        super::outbox::read_attempt(self.database(), attempt_ref)
    }
}

fn parse_committed_reservations(
    plan: &WorldPromotionPlan,
    records: &[CanonicalWorldPromotionRecord],
) -> Result<Vec<WorldReleaseReservation>> {
    let parsed = records.iter().map(|record| parse_reservation(&record.bytes)).collect::<Result<Vec<_>>>()?;
    let refs = parsed.iter().map(|reservation| reservation.reservation_ref.clone()).collect::<Vec<_>>();
    validate_reservation_set(plan, &refs)
        .map_err(|issue| MoltenError::invalid_harness(format!("reservation set mismatch: {issue:?}")))?;
    if parsed.iter().any(|reservation| reservation.state != WorldReleaseState::Committed) {
        return Err(MoltenError::invalid_harness("promotion transaction requires committed reservation records"));
    }
    Ok(parsed)
}

fn reservations_match(write: &redb::WriteTransaction, expected: &[WorldReleaseReservation]) -> Result<bool> {
    let table = write.open_table(RESERVATIONS_TABLE).map_err(store_error)?;
    for reservation in expected {
        let Some(bytes) = table
            .get(reservation.reservation_ref.as_str())
            .map_err(store_error)?
            .map(|guard| guard.value().to_vec())
        else {
            return Ok(false);
        };
        if parse_reservation(&bytes)? != *reservation {
            return Ok(false);
        }
    }
    Ok(true)
}
