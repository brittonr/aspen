//! Node daemon unit tests, grouped by lifecycle, reconcile, flow, service, and shutdown behavior.

#[path = "reconcile/acceptance.rs"]
mod acceptance;
#[path = "shutdown/carryover.rs"]
mod carryover;
#[path = "probes/denycases.rs"]
mod denycases;
#[path = "services/dispatches.rs"]
mod dispatches;
#[path = "lifecycle/enqueue.rs"]
mod enqueue;
#[path = "services/fixtures.rs"]
mod fixtures;
#[path = "flows/forgeries.rs"]
mod forgeries;
#[path = "flows/gatedenials.rs"]
mod gatedenials;
#[path = "shutdown/halts.rs"]
mod halts;
#[path = "reconcile/intakes.rs"]
mod intakes;
#[path = "probes/judgements.rs"]
mod judgements;
#[path = "services/loopbacks.rs"]
mod loopbacks;
#[path = "lifecycle/looping.rs"]
mod looping;
#[path = "lifecycle/mailbox.rs"]
mod mailbox;
#[path = "flows/mismatches.rs"]
mod mismatches;
#[path = "lifecycle/origins.rs"]
mod origins;
#[path = "reconcile/peers.rs"]
mod peers;
#[path = "probes/probes.rs"]
mod probes;
#[path = "services/profiling.rs"]
mod profiling;
#[path = "lifecycle/rebuilds.rs"]
mod rebuilds;
#[path = "services/redirects.rs"]
mod redirects;
#[path = "reconcile/refusals.rs"]
mod refusals;
#[path = "reconcile/rejections.rs"]
mod rejections;
#[path = "flows/seeding.rs"]
mod seeding;
#[path = "reconcile/seeds.rs"]
mod seeds;
#[path = "lifecycle/startup.rs"]
mod startup;
#[path = "shutdown/succession.rs"]
mod succession;
#[path = "services/supervisors.rs"]
mod supervisors;
#[path = "flows/transfers.rs"]
mod transfers;
#[path = "reconcile/verdicts.rs"]
mod verdicts;

impl super::storage::NodeStateAuthority for std::path::Path {
    fn acquire_node_state_root(&self) -> crate::error::Result<crate::node_state::DirectoryRoot> {
        crate::node_state::DirectoryRoot::open(self)
    }
}

impl super::storage::NodeStateAuthority for std::path::PathBuf {
    fn acquire_node_state_root(&self) -> crate::error::Result<crate::node_state::DirectoryRoot> {
        crate::node_state::DirectoryRoot::open(self)
    }
}

mod fs {
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub(super) fn remove_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }
}

fn test_live_authority_refs(
    state_root: &std::path::Path,
    peer_id: &str,
    node_id: &str,
    operation: &str,
    policy_refs: &[String],
) -> crate::error::Result<Vec<String>> {
    let operations = vec![operation.to_string()];
    let grant_value = super::issuance::control_authority_grant_value(&super::offers::ControlAuthorityGrantInput {
        peer_id,
        node_id,
        operations: &operations,
        target_scope: "*",
        resource_scope: "*",
        epoch: 1,
        expires_at: None,
        policy_refs,
        revocation_refs: &[],
        evidence_refs: &[],
    })?;
    let grant = super::issuance::import_control_authority_grant(state_root, &grant_value)?;
    Ok(vec![grant.grant_ref])
}

fn test_live_peer_bootstrap_refs(
    state_root: &std::path::Path,
    peer_id: &str,
    topic: &str,
    policy_refs: &[String],
) -> crate::error::Result<Vec<String>> {
    let ticket = super::issuance::export_control_live_ticket(&super::offers::ControlLiveTicketExportInput {
        state_root,
        topic,
        policy_refs,
        evidence_refs: &[],
    })?;
    let admission = super::peering::admit_control_live_peer(&super::offers::ControlLivePeerAdmitInput {
        state_root,
        ticket_value: &ticket.value,
        peer_id,
        sequence: 1,
        expires_at: None,
        policy_refs,
        evidence_refs: &[],
    })?;
    Ok(vec![admission.admission_ref])
}
