use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::Duration;

use super::tests::NODE_A;
use super::tests::NODE_B;
use super::tests::NODE_C;
use super::tests::active_group;
use super::tests::started_state;
use super::tests::test_ref;
use super::*;
use crate::error::Result;
use crate::fabric_durability::DurableAdapterKind;
use crate::fabric_durability::RedbDurableStateAdapter;
use crate::fabric_durability::tests::descriptor;
use crate::fabric_durability::tests::profile;
use crate::fabric_time::OperatingSystemEntropySource;
use crate::fabric_time::tests::live_profile;
use crate::fabric_transport::CanonicalCrossProcessEndpoint;
use crate::fabric_transport::IrohCrossProcessListener;
use crate::fabric_transport::ListenerDrainReason;
use crate::fabric_transport::cross_process::tests::client_input;
use crate::fabric_transport::cross_process::tests::listener_with_secret;

mod setup;
mod workflow;

pub(super) use setup::build_node;
pub(super) use setup::close_node;

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
    fn restore_snapshot(&mut self, snapshot: &ApplicationSnapshotRestore) -> Result<String> {
        self.restored_application_state_ref = Some(snapshot.application_state_ref.clone());
        Ok(test_ref("live-cluster-snapshot-restore"))
    }

    fn apply_batch(&mut self, commands: &[ApplicationCommand]) -> Result<String> {
        self.applied_request_refs.extend(commands.iter().map(|command| command.request_ref.clone()));
        Ok(test_ref("live-cluster-application"))
    }
}

type LivePorts = ConcreteReplicaPortBundle<OperatingSystemEntropySource, LiveApplicationHandler>;
type LiveService = ScopedLiveReplicaService<LivePorts>;

pub(super) struct LiveNode {
    pub(super) service: LiveService,
    pub(super) listener: Option<IrohCrossProcessListener>,
    pub(super) session_ref: String,
    _workspace: crate::test_support::ProcessWorkspace,
    _control_receiver: tokio::sync::mpsc::UnboundedReceiver<ReplicaControlObservation>,
}

// r[verify molten.fabric_consistency.live_raft]
#[tokio::test]
async fn three_endpoint_live_services_elect_commit_read_and_catch_up() {
    let group = active_group();
    let listener_a = listener_with_secret(NODE_A_SECRET_BYTE).await;
    let listener_b = listener_with_secret(NODE_B_SECRET_BYTE).await;
    let listener_c = listener_with_secret(NODE_C_SECRET_BYTE).await;
    let endpoints = BTreeMap::from([
        (NODE_A.to_string(), listener_a.handoff().clone()),
        (NODE_B.to_string(), listener_b.handoff().clone()),
        (NODE_C.to_string(), listener_c.handoff().clone()),
    ]);
    assert_ne!(
        endpoints[NODE_A].descriptor.public_endpoint_identity,
        endpoints[NODE_B].descriptor.public_endpoint_identity
    );
    assert_ne!(
        endpoints[NODE_B].descriptor.public_endpoint_identity,
        endpoints[NODE_C].descriptor.public_endpoint_identity
    );

    let mut node_a = setup::build_node(&group, NODE_A, listener_a, &endpoints).await.expect("node A");
    let mut node_b = setup::build_node(&group, NODE_B, listener_b, &endpoints).await.expect("node B");
    let mut node_c = setup::build_node(&group, NODE_C, listener_c, &endpoints).await.expect("node C");
    workflow::elect_node_a(&mut node_a, &mut node_b, &mut node_c).await.expect("live election");
    let request_ref = test_ref("live-cluster-request");
    workflow::replicate_request(&mut node_a, &mut node_b, &mut node_c, &request_ref)
        .await
        .expect("live replication");
    workflow::quorum_read(&mut node_a, &mut node_b, &mut node_c, &test_ref("live-cluster-linearizable-read"))
        .await
        .expect("live quorum read");
    let application_state_ref = test_ref("live-cluster-application-state");
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
    assert!(!node_a.service.ports().durability.adapter().state().durable_log.is_empty());
    assert!(!node_b.service.ports().durability.adapter().state().durable_log.is_empty());
    let node_c_snapshot_ref = &node_c.service.state().snapshot.as_ref().expect("node C snapshot").snapshot_ref;
    assert!(node_c.service.ports().durability.adapter().state().snapshots.contains_key(node_c_snapshot_ref));

    setup::close_node(node_a).await;
    setup::close_node(node_b).await;
    setup::close_node(node_c).await;
}

fn live_timeout() -> Duration {
    Duration::from_secs(LIVE_TIMEOUT_SECONDS)
}
