use super::records::*;
use super::*;

/// The canonical operator status record.
///
/// The record carries the closure repair completeness, the unrecoverable
/// identities, and the repair non-claims. A status without the
/// bytes-not-authority claim is refused: recovered bytes never make a replica
/// the current placement and never decide which head is authoritative.
pub fn canonical_operator_status(view: &OperatorStatusView) -> crate::error::Result<CanonicalReplicationRecord> {
    validate_operator_status(view)?;
    canonical(
        "operator-status",
        record("content-replication-operator-status-v2", vec![
            field("service-id", string(&view.service_id)),
            field("generation", number(view.generation)),
            field("placement-epoch", number(view.placement_epoch)),
            field("desired-replicas", count(view.desired_replicas)?),
            field("verified-replicas", count(view.verified_replicas)?),
            field("under-replicated", strings(&view.under_replicated)),
            field("active-plan-ref", string(&view.active_plan_ref)),
            field("active-operations", strings(&view.active_operations)),
            field("resource-refs", strings(&view.resource_refs)),
            field("failures", strings(&view.failures)),
            field("pins", strings(&view.pins)),
            field("evidence-refs", strings(&view.evidence_refs)),
            field("repair-completeness", string(&view.repair_completeness)),
            field("repair-unrecoverable", strings(&view.repair_unrecoverable)),
            field("repair-non-claims", strings(&view.repair_non_claims)),
            non_claims(),
        ]),
    )
}

const REPAIR_COMPLETE: &str = "complete";
const REPAIR_INCOMPLETE: &str = "incomplete";
const REPAIR_BYTES_NOT_AUTHORITY: &str = "repair-recovered-bytes-are-not-authority";

fn validate_operator_status(view: &OperatorStatusView) -> crate::error::Result<()> {
    if view.repair_completeness != REPAIR_COMPLETE && view.repair_completeness != REPAIR_INCOMPLETE {
        return Err(crate::error::Failure::invalid_harness(
            "content-replication operator status repair completeness is outside the closed vocabulary",
        ));
    }
    if view.repair_completeness == REPAIR_INCOMPLETE && view.repair_unrecoverable.is_empty() {
        return Err(crate::error::Failure::invalid_harness(
            "content-replication operator status incomplete repair must name the unrecoverable identities",
        ));
    }
    if !view.repair_non_claims.iter().any(|claim| claim == REPAIR_BYTES_NOT_AUTHORITY) {
        return Err(crate::error::Failure::invalid_harness(
            "content-replication operator status must carry the bytes-not-authority non-claim",
        ));
    }
    Ok(())
}
