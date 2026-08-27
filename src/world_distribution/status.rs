use molten_core::world_distribution::WorldClaimAdmission;
use molten_core::world_distribution::WorldRetentionReport;
use molten_core::world_distribution::distribution_non_claims;

use super::WorldSyncOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldDistributionStatusView {
    pub commit_ref: String,
    pub dag_plan_ref: String,
    pub epoch_ref: String,
    pub complete: bool,
    pub missing: Vec<String>,
    pub verified: usize,
    pub admitted_claims: usize,
    pub denied_claims: usize,
    pub conflict_ref: Option<String>,
    pub retention_complete: bool,
    pub retained_refs: Vec<String>,
    pub unresolved_remote: Vec<String>,
    pub resources: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub activation_authorized: bool,
    pub deletion_authorized: bool,
    pub non_claims: Vec<String>,
}

pub fn world_distribution_status(
    sync: &WorldSyncOutcome,
    claims: Option<&WorldClaimAdmission>,
    retention: Option<&WorldRetentionReport>,
) -> WorldDistributionStatusView {
    let mut missing = sync
        .missing
        .iter()
        .map(|object| format!("{}:{}", object.domain().as_str(), object.as_str()))
        .collect::<Vec<_>>();
    let mut retained_refs = retention.map_or_else(Vec::new, |report| report.retained_refs.clone());
    let mut unresolved_remote = retention.map_or_else(Vec::new, |report| report.unresolved_remote.clone());
    let mut evidence_refs = sync.dag.evidence_refs.clone();
    if let Some(report) = retention {
        evidence_refs.extend(report.evidence_refs.iter().cloned());
    }
    missing.sort();
    retained_refs.sort();
    retained_refs.dedup();
    unresolved_remote.sort();
    unresolved_remote.dedup();
    evidence_refs.push(sync.canonical_receipt.record_ref.clone());
    evidence_refs.sort();
    evidence_refs.dedup();
    WorldDistributionStatusView {
        commit_ref: sync.initial_plan.projection.requested.as_str().to_string(),
        dag_plan_ref: sync.dag.plan.plan_ref.as_str().to_string(),
        epoch_ref: sync.dag.plan.epoch_ref.as_str().to_string(),
        complete: sync.complete,
        missing,
        verified: sync.dag.receipt.verified,
        admitted_claims: claims.map_or(0, |admission| admission.admitted.len()),
        denied_claims: claims.map_or(0, |admission| admission.denied.len()),
        conflict_ref: claims
            .and_then(|admission| admission.conflict.as_ref().map(|conflict| conflict.conflict_ref.clone())),
        retention_complete: retention.is_some_and(|report| report.reference_index_complete),
        retained_refs,
        unresolved_remote,
        resources: vec![sync.dag.resource_ref.clone()],
        evidence_refs,
        activation_authorized: false,
        deletion_authorized: false,
        non_claims: distribution_non_claims(),
    }
}
