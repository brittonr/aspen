use transactional_reconciliation_core::PersistenceState;
use transactional_reconciliation_core::QuarantineReason;
use transactional_reconciliation_core::QuarantineStatus;

use super::*;

pub const WORLD_PROMOTION_RECONCILIATION_SCHEMA: &str = "molten.world-promotion-reconciliation.v1";
const RECONCILIATION_RECORD: &str = "world-promotion-reconciliation";

pub fn canonical_persistence(
    plan: &WorldPromotionPlan,
    persistence: &WorldPromotionPersistence,
) -> Result<CanonicalWorldPromotionRecord> {
    require_non_claims(&persistence.non_claims)?;
    canonical(crate::preserves_rail::record(RECONCILIATION_RECORD, vec![
        crate::preserves_rail::string(WORLD_PROMOTION_RECONCILIATION_SCHEMA),
        named("plan-ref", string(plan.plan_ref.as_str())),
        named("state", string(persistence_state(persistence.shared.state()))),
        named("quarantine", string(quarantine(persistence.shared.quarantine()))),
        named("dispatch-eligible", boolean(persistence.dispatch_eligible)),
        named("mutation-authorized-by-evidence", boolean(persistence.mutation_authorized_by_evidence)),
        non_claims(),
    ]))
}

fn persistence_state(state: PersistenceState) -> &'static str {
    match state {
        PersistenceState::NotPublished => "not-published",
        PersistenceState::Published => "published",
        PersistenceState::PublicationUnknown => "publication-unknown",
        PersistenceState::Conflicting => "conflicting",
    }
}

fn quarantine(status: QuarantineStatus) -> &'static str {
    match status {
        QuarantineStatus::Clear => "clear",
        QuarantineStatus::Quarantined(QuarantineReason::CommitOutcomeUnknown) => "commit-outcome-unknown",
        QuarantineStatus::Quarantined(QuarantineReason::RepairReported) => "repair-reported",
        QuarantineStatus::Quarantined(QuarantineReason::Corrupt) => "corrupt",
        QuarantineStatus::Quarantined(QuarantineReason::Missing) => "missing",
        QuarantineStatus::Quarantined(QuarantineReason::Inconsistent) => "inconsistent",
    }
}
