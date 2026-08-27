#[path = "../src/test/support.rs"]
mod test_support;

use std::collections::BTreeMap;
use std::path::PathBuf;

use molten::cluster_harness::DEFAULT_DISTINCT_PROCESS_TIMEOUT_MS;
use molten::content_replication::*;
use molten_core::content_replication::*;

const DIGEST_HEX_LENGTH: usize = 64;
const GENERATION: u64 = 1;
const MEMBERSHIP_EPOCH: u64 = 2;
const PLACEMENT_EPOCH: u64 = 3;
const DESIRED_REPLICAS: usize = 2;
const TRANSFER_LIMIT: usize = 4;
const TRANSFER_BYTES: u64 = 4_096;
const QUEUE_LIMIT: usize = 16;
const TIMER_LIMIT: usize = 4;
const DIAGNOSTIC_LIMIT: usize = 16;
const PAYLOAD: &[u8] = b"bounded-multiprocess-replica";

fn digest(byte: char) -> String {
    format!("blake3:{}", byte.to_string().repeat(DIGEST_HEX_LENGTH))
}

fn manifest() -> Manifest {
    let content_ref = molten::preserves_rail::content_ref_from_bytes(PAYLOAD);
    Manifest {
        service_id: "content-replication-multiprocess".to_string(),
        generation: GENERATION,
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        authority_ref: digest('1'),
        identity_ref: digest('2'),
        content_profile_ref: digest('3'),
        transport_profile_ref: digest('4'),
        retention_policy_ref: digest('5'),
        evidence_profile_ref: digest('6'),
        ports: REQUIRED_PORTS.iter().map(ToString::to_string).collect(),
        policy: ReplicaPolicy {
            desired_replicas: DESIRED_REPLICAS,
            minimum_verified_replicas: DESIRED_REPLICAS,
            minimum_fault_domains: DESIRED_REPLICAS,
        },
        repair: RepairPolicy {
            max_attempts: MAX_REPAIR_ATTEMPTS,
            allow_handoff: true,
            cleanup_after_handoff: true,
        },
        resources: ResourceLimits {
            max_concurrent_transfers: TRANSFER_LIMIT,
            max_transfer_bytes: TRANSFER_BYTES,
            max_queue_depth: QUEUE_LIMIT,
            max_timers: TIMER_LIMIT,
            max_diagnostics: DIAGNOSTIC_LIMIT,
        },
        contents: vec![ReplicaRule {
            content_ref,
            manifest_ref: digest('7'),
            encoded_bytes: u64::try_from(PAYLOAD.len()).expect("payload length"),
            protected: true,
            transform_ref: Some(digest('8')),
            cleanup_authority_ref: Some(digest('9')),
        }],
        non_claims: NON_CLAIMS.iter().map(ToString::to_string).collect(),
    }
}

fn peer(id: &str, domain: &str) -> Peer {
    Peer {
        peer_id: id.to_string(),
        fault_domain: domain.to_string(),
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        available: true,
        capacity_bytes: TRANSFER_BYTES,
    }
}

fn replica(manifest: &Manifest, peer_id: &str, domain: &str, verified: bool) -> Replica {
    Replica {
        content_ref: manifest.contents[0].content_ref.clone(),
        peer_id: peer_id.to_string(),
        fault_domain: domain.to_string(),
        generation: GENERATION,
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        present: true,
        identity_verified: verified,
        pinned: true,
        protected: true,
        manifest_ref: manifest.contents[0].manifest_ref.clone(),
        cleanup_clearance_ref: None,
    }
}

fn action(manifest: &Manifest, corrupt_target: bool) -> Action {
    let mut replicas = vec![replica(manifest, "peer-a", "zone-a", true)];
    if corrupt_target {
        replicas.push(replica(manifest, "peer-b", "zone-b", false));
    }
    let plan = molten_core::content_replication::plan(&ReconcileInput {
        manifest: manifest.clone(),
        inventory: Inventory { replicas },
        peers: vec![peer("peer-a", "zone-a"), peer("peer-b", "zone-b")],
        history: Vec::new(),
        observed_tick: 1,
    })
    .expect("multiprocess plan");
    plan.actions
        .into_iter()
        .find(|action| matches!(action.kind, ActionKind::Transfer | ActionKind::Repair))
        .expect("multiprocess action")
}

fn run_action(label: &str, manifest: &Manifest, action: &Action) -> TransferEnvelope {
    let workspace = test_support::process_workspace(label).expect("process workspace");
    let run_root = workspace.join("run");
    let mut adapter = DistinctProcessTransferAdapter::open(
        manifest,
        run_root.clone(),
        PathBuf::from(env!("CARGO_BIN_EXE_molten")),
        DEFAULT_DISTINCT_PROCESS_TIMEOUT_MS,
        BTreeMap::from([(manifest.contents[0].content_ref.clone(), PAYLOAD.to_vec())]),
    )
    .expect("multiprocess adapter");
    let envelope = match adapter.fetch(action).expect("multiprocess transfer") {
        TransferOutcome::Received(envelope) => envelope,
        other => panic!("unexpected transfer outcome: {other:?}"),
    };
    let verification = molten::cluster_harness::verify_distinct_process_transport_run(
        &run_root.join(action.operation_id.strip_prefix("blake3:").expect("operation prefix")),
    )
    .expect("offline multiprocess verification");
    assert_eq!(verification.decision, "pass");
    assert_eq!(verification.parent_ref, envelope.transfer_ref);
    assert_eq!(verification.verification_ref, envelope.transport_verification_ref);
    assert_eq!(adapter.call_count(), 1);
    envelope
}

#[test]
fn multiprocess_replication_moves_exact_content_under_operation_identity() {
    let manifest = manifest();
    let action = action(&manifest, false);
    assert_eq!(action.kind, ActionKind::Transfer);
    let envelope = run_action("content_replication_transfer", &manifest, &action);
    assert_eq!(envelope.operation_id, action.operation_id);
    assert_eq!(envelope.content_ref, manifest.contents[0].content_ref);
    assert_eq!(envelope.encoded_bytes, manifest.contents[0].encoded_bytes);
}

#[test]
fn multiprocess_repair_uses_the_same_receiver_plan_and_transport_contract() {
    let manifest = manifest();
    let action = action(&manifest, true);
    assert_eq!(action.kind, ActionKind::Repair);
    let envelope = run_action("content_replication_repair", &manifest, &action);
    assert_eq!(envelope.target_peer, "peer-b");
    assert!(envelope.protected);
}

#[test]
fn multiprocess_adapter_rejects_wrong_payload_before_child_processes() {
    let manifest = manifest();
    let workspace = test_support::process_workspace("content_replication_wrong_payload").expect("process workspace");
    let result = DistinctProcessTransferAdapter::open(
        &manifest,
        workspace.join("run"),
        PathBuf::from(env!("CARGO_BIN_EXE_molten")),
        DEFAULT_DISTINCT_PROCESS_TIMEOUT_MS,
        BTreeMap::from([(manifest.contents[0].content_ref.clone(), b"wrong".to_vec())]),
    );
    assert!(result.is_err());
    assert!(!workspace.join("run").exists());
}
