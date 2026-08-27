use molten_core::dag_sync::*;

use super::DagSyncOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagSyncStatusView {
    pub plan_ref: String,
    pub epoch_ref: String,
    pub generation: u64,
    pub strategy: String,
    pub roots: Vec<String>,
    pub requested: Vec<String>,
    pub verified: Vec<String>,
    pub missing: Vec<String>,
    pub peers: Vec<String>,
    pub resources: Vec<String>,
    pub failures: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub non_claims: Vec<String>,
}

pub fn dag_sync_status(outcome: &DagSyncOutcome) -> DagSyncStatusView {
    let plan = &outcome.plan;
    let progress = &outcome.progress;
    let receipt = &outcome.receipt;
    let mut peers = plan
        .requests
        .iter()
        .filter_map(|request| request.assigned_peer.as_ref().map(DagPeerId::as_str))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    peers.sort();
    peers.dedup();
    DagSyncStatusView {
        plan_ref: plan.plan_ref.as_str().to_string(),
        epoch_ref: plan.epoch_ref.as_str().to_string(),
        generation: plan.generation,
        strategy: plan.strategy.as_str().to_string(),
        roots: plan.roots.iter().map(DagRootRef::as_str).map(ToString::to_string).collect(),
        requested: plan.requests.iter().map(|request| request.object_ref.as_str().to_string()).collect(),
        verified: progress.verified.iter().map(DagObjectRef::as_str).map(ToString::to_string).collect(),
        missing: receipt.missing.iter().map(DagObjectRef::as_str).map(ToString::to_string).collect(),
        peers,
        resources: vec![outcome.resource_ref.clone()],
        failures: receipt.issues.iter().map(|issue| issue.as_str().to_string()).collect(),
        evidence_refs: outcome.evidence_refs.clone(),
        non_claims: receipt.non_claims.clone(),
    }
}
