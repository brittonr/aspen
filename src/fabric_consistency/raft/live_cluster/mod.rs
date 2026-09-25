use super::*;

pub(in crate::fabric_consistency::raft) mod setup;
mod workflow;

const NODE_A_SECRET_BYTE: u8 = 17;
const NODE_B_SECRET_BYTE: u8 = 19;
const NODE_C_SECRET_BYTE: u8 = 23;
const LIVE_TIMEOUT_SECONDS: u64 = 10;
const LIVE_TICK_SECONDS: u64 = 10;
const LIVE_HEARTBEAT_TICKS: u64 = 1;
const LIVE_ELECTION_MIN_TICKS: u64 = 2;
const LIVE_ELECTION_MAX_TICKS: u64 = 3;
const LIVE_FABRIC_BINDING_COUNT: usize = 7;
const LIVE_TERM: u64 = 1;

#[derive(Debug, Default)]
pub(super) struct LiveApplicationHandler {
    pub(super) applied_request_refs: Vec<String>,
    pub(super) restored_application_state_ref: Option<String>,
}

impl CommittedBatchHandler for LiveApplicationHandler {
    fn restore_snapshot(&mut self, snapshot: &ApplicationSnapshotRestore) -> crate::error::Result<String> {
        self.restored_application_state_ref = Some(snapshot.application_state_ref.clone());
        Ok(super::tests::test_ref("live-cluster-snapshot-restore"))
    }

    fn apply_batch(&mut self, commands: &[ApplicationCommand]) -> crate::error::Result<String> {
        self.applied_request_refs.extend(commands.iter().map(|command| command.request_ref.clone()));
        Ok(super::tests::test_ref("live-cluster-application"))
    }
}

type LivePorts = ConcreteReplicaPortBundle<crate::fabric_time::OperatingSystemEntropySource, LiveApplicationHandler>;
type LiveService = ScopedLiveReplicaService<LivePorts>;

pub(super) struct LiveNode {
    pub(super) service: LiveService,
    pub(super) listener: Option<crate::fabric_transport::IrohCrossProcessListener>,
    pub(super) session_ref: String,
    pub(super) recovery_ref: Option<String>,
    _workspace: Option<crate::test_support::ProcessWorkspace>,
    _control_receiver: tokio::sync::mpsc::Receiver<ReplicaControlObservation>,
}

// r[verify molten.fabric_consistency.live_raft]
#[tokio::test]
async fn three_endpoint_live_services_elect_commit_read_and_catch_up() {
    let group = super::tests::active_group();
    let listener_a = crate::fabric_transport::cross_process::tests::listener_with_secret(NODE_A_SECRET_BYTE).await;
    let listener_b = crate::fabric_transport::cross_process::tests::listener_with_secret(NODE_B_SECRET_BYTE).await;
    let listener_c = crate::fabric_transport::cross_process::tests::listener_with_secret(NODE_C_SECRET_BYTE).await;
    let endpoints = std::collections::BTreeMap::from([
        (super::tests::NODE_A.to_string(), listener_a.handoff().clone()),
        (super::tests::NODE_B.to_string(), listener_b.handoff().clone()),
        (super::tests::NODE_C.to_string(), listener_c.handoff().clone()),
    ]);
    assert_ne!(
        endpoints[super::tests::NODE_A].descriptor.public_endpoint_identity,
        endpoints[super::tests::NODE_B].descriptor.public_endpoint_identity
    );
    assert_ne!(
        endpoints[super::tests::NODE_B].descriptor.public_endpoint_identity,
        endpoints[super::tests::NODE_C].descriptor.public_endpoint_identity
    );

    let mut node_a = setup::build_node(&group, super::tests::NODE_A, listener_a, &endpoints).await.expect("node A");
    let mut node_b = setup::build_node(&group, super::tests::NODE_B, listener_b, &endpoints).await.expect("node B");
    let mut node_c = setup::build_node(&group, super::tests::NODE_C, listener_c, &endpoints).await.expect("node C");
    workflow::elect_node_a(&mut node_a, &mut node_b, &mut node_c).await.expect("live election");
    let request_ref = super::tests::test_ref("live-cluster-request");
    workflow::replicate_request(&mut node_a, &mut node_b, &mut node_c, &request_ref)
        .await
        .expect("live replication");
    workflow::quorum_read(
        &mut node_a,
        &mut node_b,
        &mut node_c,
        &super::tests::test_ref("live-cluster-linearizable-read"),
    )
    .await
    .expect("live quorum read");
    let application_state_ref = super::tests::test_ref("live-cluster-application-state");
    workflow::snapshot_catch_up(&mut node_a, &mut node_b, &mut node_c, &application_state_ref)
        .await
        .expect("live snapshot catch-up");

    assert_eq!(node_a.service.state().role, ReplicaRole::Leader);
    assert_eq!(node_a.service.state().current_term, LIVE_TERM);
    assert_eq!(node_a.service.state().commit_index, INITIAL_LOG_INDEX);
    assert_eq!(node_b.service.state().commit_index, INITIAL_LOG_INDEX);
    assert!(node_a.service.state().pending_reads.is_empty());
    assert_eq!(node_a.service.state().quorum_confirmed_term, Some(LIVE_TERM));
    assert_eq!(node_a.service.state().completed_requests.get(&request_ref), Some(&INITIAL_LOG_INDEX));
    assert_eq!(node_b.service.state().completed_requests.get(&request_ref), Some(&INITIAL_LOG_INDEX));
    assert_eq!(node_c.service.state().commit_index, INITIAL_LOG_INDEX);
    assert_eq!(node_c.service.state().last_applied, INITIAL_LOG_INDEX);
    assert_eq!(node_c.service.state().completed_requests.get(&request_ref), Some(&INITIAL_LOG_INDEX));
    assert_eq!(
        node_c.service.ports().application.handler().restored_application_state_ref,
        Some(application_state_ref)
    );
    assert_eq!(node_a.service.ports().application.handler().applied_request_refs, vec![request_ref.clone()]);
    assert_eq!(node_b.service.ports().application.handler().applied_request_refs, vec![request_ref]);
    assert_selected_evidence_and_readback(&group, &node_a, &node_b, &node_c);

    setup::close_node(node_a).await;
    setup::close_node(node_b).await;
    setup::close_node(node_c).await;
}

/// The leader records the selected milestone evidence and a healthy, unadmitted readback, and every
/// replica keeps its durable log or snapshot.
fn assert_selected_evidence_and_readback(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
    node_a: &LiveNode,
    node_b: &LiveNode,
    node_c: &LiveNode,
) {
    let selected_evidence = node_a
        .service
        .evidence()
        .records()
        .iter()
        .map(|record| record.kind)
        .collect::<std::collections::BTreeSet<_>>();
    for kind in [
        ReplicaEvidenceKind::GroupAdmission,
        ReplicaEvidenceKind::Configuration,
        ReplicaEvidenceKind::Commit,
        ReplicaEvidenceKind::ReadCurrentness,
        ReplicaEvidenceKind::Snapshot,
    ] {
        assert!(selected_evidence.contains(&kind));
    }
    assert!(node_a.service.evidence().suppressed_heartbeat_count() > 0);
    let health = node_a.service.aggregate_health_evidence().expect("aggregate health evidence");
    assert_eq!(health.status, "healthy");
    assert!(!health.production_admitted);
    crate::preserves_rail::validate_content_ref(&health.evidence_ref).expect("aggregate health ref");
    let readback = crate::fabric_consistency::live_replica_operator_readback(group, &node_a.service)
        .expect("live operator readback");
    assert_eq!(readback.commit_index, INITIAL_LOG_INDEX);
    assert!(!readback.production_admitted);
    assert!(readback.selected_evidence_refs.len() <= crate::fabric_consistency::MAX_OPERATOR_EVIDENCE_REFS);
    crate::preserves_rail::validate_content_ref(&readback.readback_ref).expect("live operator readback ref");
    assert!(!node_a.service.ports().durability.adapter().state().durable_log.is_empty());
    assert!(!node_b.service.ports().durability.adapter().state().durable_log.is_empty());
    let node_c_snapshot_ref = &node_c.service.state().snapshot.as_ref().expect("node C snapshot").snapshot_ref;
    assert!(node_c.service.ports().durability.adapter().state().snapshots.contains_key(node_c_snapshot_ref));
}

fn live_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(LIVE_TIMEOUT_SECONDS)
}
