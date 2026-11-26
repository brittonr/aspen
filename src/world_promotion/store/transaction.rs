use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadState;
use molten_core::world_promotion::*;
use redb::ReadableTable;

use super::*;
use crate::error::Failure;
use crate::error::Result;
use crate::world_head::canonical_world_head_state;
use crate::world_head::parse_canonical_world_head_state;
use crate::world_promotion::CanonicalWorldPromotionRecord;
use crate::world_promotion::StoredPromotionIdentity;
use crate::world_promotion::WorldOperatorMutation;
use crate::world_promotion::WorldPromotionBookkeepingPort;
use crate::world_promotion::WorldPromotionCommitPort;
use crate::world_promotion::WorldPromotionTransitionPort;
use crate::world_promotion::parse_reservation;

impl WorldPromotionCommitPort for LocalWorldPromotionStore {
    // r[impl molten.world_commit.promotion_atomic_dispatch_and_readback]
    // r[impl molten.world_commit.promotion_authority_action_and_canonical_binding]
    fn commit_promotion(
        &mut self,
        plan: &WorldPromotionPlan,
        canonical_plan: &CanonicalWorldPromotionRecord,
        reservations: &[CanonicalWorldPromotionRecord],
        facts: &WorldPromotionTransactionFacts,
    ) -> Result<WorldPromotionCommitObservation> {
        let parsed = parse_committed_reservations(plan, reservations)?;
        let canonical_identity = crate::world_promotion::canonical_promotion_identity(&plan.identity)?;
        let canonical_grant = crate::world_promotion::canonical_promotion_grant(&facts.grant)?;
        let write = self.database().begin_write().map_err(store_error)?;
        let is_identity_recorded = {
            let identities = write.open_table(PROMOTION_IDENTITIES_TABLE).map_err(store_error)?;
            let existing = identities
                .get(plan.operation_ref.as_str())
                .map_err(store_error)?
                .map(|guard| guard.value().to_vec());
            match existing {
                Some(existing) if crate::world_promotion::parse_promotion_identity(&existing)? != plan.identity => {
                    return Ok(WorldPromotionCommitObservation::Inconsistent);
                }
                Some(_) => true,
                None => false,
            }
        };
        let live_head = {
            let heads = write.open_table(WORLD_HEADS_TABLE).map_err(store_error)?;
            heads
                .get(plan.after.branch_id.as_str())
                .map_err(store_error)?
                .map(|guard| parse_canonical_world_head_state(guard.value()))
                .transpose()?
        };
        if is_identity_recorded {
            // This operation already committed: report it without writing or consuming authority.
            let is_complete = live_head.as_ref() == Some(&plan.after) && reservations_match(&write, &parsed)?;
            return Ok(if is_complete {
                WorldPromotionCommitObservation::Applied
            } else {
                WorldPromotionCommitObservation::Inconsistent
            });
        }
        let is_nonce_consumed = {
            let consumed = write.open_table(CONSUMED_GRANTS_TABLE).map_err(store_error)?;
            consumed.get(facts.grant.nonce.as_str()).map_err(store_error)?.is_some()
        };
        let live = WorldPromotionLiveFacts {
            head: live_head,
            grant_nonce_consumed: is_nonce_consumed,
        };
        validate_promotion_transaction(plan, facts, &live)
            .map_err(|issues| Failure::invalid_harness(format!("world promotion transaction denied: {issues:?}")))?;
        let (_, state_bytes) = canonical_world_head_state(&plan.after)?;
        super::outbox::consume_grant(&write, &facts.grant.nonce, &canonical_grant.bytes)?;
        {
            let mut identities = write.open_table(PROMOTION_IDENTITIES_TABLE).map_err(store_error)?;
            identities
                .insert(plan.operation_ref.as_str(), canonical_identity.bytes.as_slice())
                .map_err(store_error)?;
        }
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

    /// Classifies what durable state shows of `plan`. The promotion record, the successor head and
    /// every planned reservation are written in one transaction, so any partial subset is
    /// `Corrupt`. Reservations may have left `Committed` since (claimed, attempted, denied, ...):
    /// only their immutable identity fields are compared, so idempotent replay keeps working.
    // r[impl molten.world_commit.promotion_atomic_dispatch_and_readback]
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
        let is_plan_recorded = {
            let promotions = read.open_table(PROMOTIONS_TABLE).map_err(store_error)?;
            promotions.get(plan.plan_ref.as_str()).map_err(store_error)?.is_some()
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
            if !super::outbox::is_same_reservation_identity(&observed, expected)
                || observed.state == WorldReleaseState::Planned
            {
                return Ok(WorldPromotionReadBackObservation::Corrupt);
            }
            present = present.saturating_add(1);
        }
        Ok(classify_read_back(plan, head, is_plan_recorded, present))
    }
    fn read_promotion_identity(
        &self,
        operation_ref: &WorldPromotionOperationRef,
    ) -> Result<Option<StoredPromotionIdentity>> {
        let read = self.database().begin_read().map_err(store_error)?;
        {
            let identities = read.open_table(PROMOTION_IDENTITIES_TABLE).map_err(store_error)?;
            if let Some(bytes) =
                identities.get(operation_ref.as_str()).map_err(store_error)?.map(|guard| guard.value().to_vec())
            {
                return Ok(Some(StoredPromotionIdentity::Current(crate::world_promotion::parse_promotion_identity(
                    &bytes,
                )?)));
            }
        }
        let promotions = read.open_table(PROMOTIONS_TABLE).map_err(store_error)?;
        for entry in promotions.iter().map_err(store_error)? {
            let (_, value) = entry.map_err(store_error)?;
            let (stored_operation, plan_ref) = crate::world_promotion::parse_promotion_operation(value.value())?;
            if &stored_operation == operation_ref {
                return Ok(Some(StoredPromotionIdentity::Legacy {
                    operation_ref: stored_operation,
                    plan_ref,
                }));
            }
        }
        Ok(None)
    }
}

fn classify_read_back(
    plan: &WorldPromotionPlan,
    head: Option<WorldHeadState>,
    is_plan_recorded: bool,
    present: usize,
) -> WorldPromotionReadBackObservation {
    let is_complete = is_plan_recorded && present == plan.reservations.len();
    let is_absent = !is_plan_recorded && present == 0;
    match head {
        // Applied, possibly superseded by a later promotion of the same branch.
        Some(head) if is_complete && head.generation >= plan.after.generation => {
            if head.generation == plan.after.generation && head != plan.after {
                WorldPromotionReadBackObservation::Corrupt
            } else {
                WorldPromotionReadBackObservation::Reservation
            }
        }
        Some(head) if is_absent && head != plan.after => WorldPromotionReadBackObservation::Prior {
            head: head.head,
            generation: head.generation,
        },
        None if is_absent => WorldPromotionReadBackObservation::Missing,
        _ => WorldPromotionReadBackObservation::Corrupt,
    }
}

impl WorldPromotionBookkeepingPort for LocalWorldPromotionStore {
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

    fn list_attempts(&self) -> Result<Vec<WorldAttemptRecord>> {
        super::outbox::list_attempts(self.database())
    }
}

impl WorldPromotionTransitionPort for LocalWorldPromotionStore {
    fn record_attempt_transition(
        &mut self,
        reservation: &WorldReleaseReservation,
        attempt: &WorldAttemptRecord,
    ) -> Result<()> {
        super::outbox::record_attempt_transition(self.database(), reservation, attempt)
    }

    fn observe_live(
        &self,
        branch_id: &WorldBranchId,
        nonce: &WorldPromotionGrantNonce,
    ) -> Result<WorldPromotionLiveFacts> {
        super::outbox::observe_live(self.database(), branch_id, nonce)
    }

    fn apply_operator_mutation(&mut self, mutation: &WorldOperatorMutation) -> Result<()> {
        super::outbox::apply_operator_mutation(self.database(), mutation)
    }
}

fn parse_committed_reservations(
    plan: &WorldPromotionPlan,
    records: &[CanonicalWorldPromotionRecord],
) -> Result<Vec<WorldReleaseReservation>> {
    let parsed = records.iter().map(|record| parse_reservation(&record.bytes)).collect::<Result<Vec<_>>>()?;
    let refs = parsed.iter().map(|reservation| reservation.reservation_ref.clone()).collect::<Vec<_>>();
    validate_reservation_set(plan, &refs)
        .map_err(|issue| Failure::invalid_harness(format!("reservation set mismatch: {issue:?}")))?;
    if parsed.iter().any(|reservation| reservation.state != WorldReleaseState::Committed) {
        return Err(Failure::invalid_harness("promotion transaction requires committed reservation records"));
    }
    Ok(parsed)
}

/// True when every expected reservation is stored with unchanged identity fields. Their states may
/// have advanced since the promotion committed.
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
        if !super::outbox::is_same_reservation_identity(&parse_reservation(&bytes)?, reservation) {
            return Ok(false);
        }
    }
    Ok(true)
}
