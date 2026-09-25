use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::rc::Rc;

use molten_core::content_replication::*;
use molten_core::dag_sync::*;
use molten_core::world_commit::*;
use molten_core::world_distribution::*;
use molten_core::world_head::*;

use super::*;
use crate::content_replication::CleanupObservation;
use crate::content_replication::ContentPort;
use crate::content_replication::TransferEnvelope;
use crate::content_replication::TransferOutcome;
use crate::content_replication::TransportPort;
use crate::content_replication::VerificationObservation;
use crate::dag_sync::DagAuthorityObservation;
use crate::dag_sync::DagAuthorityPort;
use crate::dag_sync::DagObservationPort;
use crate::dag_sync::DagProgressPort;
use crate::dag_sync::DagReceiptPort;
use crate::dag_sync::DagResourceObservation;
use crate::dag_sync::DagResourcePort;
use crate::dag_sync::DagSyncPorts;
use crate::error::Result;
use crate::retention::ACTION_DELETE;
use crate::retention::CLASS_DURABLE_VALUE;
use crate::retention::DestructiveEvidence;
use crate::world_commit::CanonicalWorldCommit;
use crate::world_commit::PublicationOutcome;
use crate::world_commit::WorldCommitPortError;
use crate::world_commit::WorldCommitPublicationPort;
use crate::world_commit::WorldImmutableObjectPort;
use crate::world_commit::canonical_world_commit;

const CURRENT_GENERATION: u64 = 1;
const NEXT_GENERATION: u64 = CURRENT_GENERATION + 1;
const ROOT_BYTES: u64 = 64;
const DESIRED_REPLICAS: usize = 2;
const MINIMUM_REPLICAS: usize = 1;
const MINIMUM_DOMAINS: usize = 2;
const MAX_ATTEMPTS: u32 = 3;
const TRANSFER_LIMIT: usize = 64;
const QUEUE_LIMIT: usize = 128;
const TIMER_LIMIT: usize = 16;

#[test]
fn local_catalog_binds_canonical_commit_and_root_bytes() {
    let (store, commit) = fixture_store();
    let projection =
        load_world_dag_projection(&store, &commit.commit_ref, &world_bounds()).expect("capability-backed projection");
    assert_eq!(projection.requested, commit.commit_ref);
    assert_eq!(projection.objects.len(), SnapshotProfileKind::Logical.required_roots().len() + 1);

    let mut missing = store;
    let root = commit.core.roots.first().expect("root").as_str().to_string();
    missing.roots.remove(&root);
    assert!(load_world_dag_projection(&missing, &commit.commit_ref, &world_bounds()).is_err());
}

// r[verify molten.world_distribution.closure]
#[test]
fn local_catalog_admits_the_closure_object_bound_and_denies_one_past() {
    let (store, commit) = fixture_store();
    let object_count = SnapshotProfileKind::Logical.required_roots().len() + 1;
    let exact = WorldCommitBounds {
        max_closure_objects: object_count,
        ..world_bounds()
    };
    let projection = load_world_dag_projection(&store, &commit.commit_ref, &exact).expect("exact closure bound");
    assert_eq!(projection.objects.len(), object_count);

    let one_short = WorldCommitBounds {
        max_closure_objects: object_count - 1,
        ..world_bounds()
    };
    assert!(load_world_dag_projection(&store, &commit.commit_ref, &one_short).is_err());
}

// r[verify molten.world_distribution.closure]
// r[verify molten.world_distribution.partial]
#[test]
fn content_replication_bridge_completes_sync_and_publishes_domain_receipt_last() {
    let (store, commit) = fixture_store();
    let projection = load_world_dag_projection(&store, &commit.commit_ref, &world_bounds()).expect("projection");
    let profile = replication_profile();
    let manifest = world_replication_manifest(&projection, &profile).expect("replication manifest");
    let source_peer = reference("source-peer");
    let request = WorldReplicationPlanRequest {
        profile: profile.clone(),
        inventory: source_inventory(&manifest, &profile, &source_peer),
        peers: vec![target_peer(&profile)],
        history: Vec::new(),
        observed_tick: CURRENT_GENERATION,
    };
    let replication = plan_world_replication(&projection, &request).expect("replication plan");
    let bridge = WorldReplicationBridge::new(&replication).expect("replication bridge");
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut transfer = ReplicationTransport { corrupt: false };
    let mut content = ReplicationContent { corrupt: false };
    let mut dag_transport = bridge.transport(&mut transfer);
    let mut dag_content = bridge.verification(&mut content);
    let mut authority = DagAuthority;
    let mut resources = DagResources;
    let mut progress = Progress {
        loaded: None,
        events: Rc::clone(&events),
    };
    let mut observations = Observations;
    let mut dag_receipts = DagReceipts {
        events: Rc::clone(&events),
    };
    let mut world_receipts = WorldReceipts {
        events: Rc::clone(&events),
        count: 0,
    };
    let outcome = run_world_sync(&projection, &sync_context(), WorldSyncPorts {
        dag: DagSyncPorts {
            authority: &mut authority,
            resources: &mut resources,
            transport: &mut dag_transport,
            content: &mut dag_content,
            progress: &mut progress,
            observations: &mut observations,
            receipts: &mut dag_receipts,
        },
        receipts: &mut world_receipts,
    })
    .expect("world sync");
    assert!(outcome.complete);
    assert!(outcome.missing.is_empty());
    assert!(!outcome.activation_authorized);
    assert_eq!(world_receipts.count, 1);
    assert_eq!(events.borrow().last(), Some(&"world-receipt"));
    let status = world_distribution_status(&outcome, None, None);
    assert!(status.complete);
    assert!(!status.activation_authorized);
    assert!(!status.deletion_authorized);
}

// r[verify molten.world_distribution.verification]
#[test]
fn corruption_fails_before_progress_or_world_receipt() {
    let (store, commit) = fixture_store();
    let projection = load_world_dag_projection(&store, &commit.commit_ref, &world_bounds()).expect("projection");
    let profile = replication_profile();
    let manifest = world_replication_manifest(&projection, &profile).expect("replication manifest");
    let request = WorldReplicationPlanRequest {
        profile: profile.clone(),
        inventory: source_inventory(&manifest, &profile, &reference("source-peer")),
        peers: vec![target_peer(&profile)],
        history: Vec::new(),
        observed_tick: CURRENT_GENERATION,
    };
    let replication = plan_world_replication(&projection, &request).expect("replication plan");
    let bridge = WorldReplicationBridge::new(&replication).expect("bridge");
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut transfer = ReplicationTransport { corrupt: false };
    let mut content = ReplicationContent { corrupt: true };
    let mut dag_transport = bridge.transport(&mut transfer);
    let mut dag_content = bridge.verification(&mut content);
    let mut authority = DagAuthority;
    let mut resources = DagResources;
    let mut progress = Progress {
        loaded: None,
        events: Rc::clone(&events),
    };
    let mut observations = Observations;
    let mut dag_receipts = DagReceipts {
        events: Rc::clone(&events),
    };
    let mut world_receipts = WorldReceipts { events, count: 0 };
    let result = run_world_sync(&projection, &sync_context(), WorldSyncPorts {
        dag: DagSyncPorts {
            authority: &mut authority,
            resources: &mut resources,
            transport: &mut dag_transport,
            content: &mut dag_content,
            progress: &mut progress,
            observations: &mut observations,
            receipts: &mut dag_receipts,
        },
        receipts: &mut world_receipts,
    });
    assert!(result.is_err());
    assert_eq!(world_receipts.count, 0);
}

// r[verify molten.world_distribution.head_claims]
#[test]
fn claim_exchange_rechecks_local_facts_and_never_mutates_a_head() {
    let carrier = claim_carrier();
    let mut transport = ClaimTransport {
        carriers: vec![carrier],
    };
    let mut authentication = ClaimAuthentication;
    let mut authority = ClaimAuthority { admitted: true };
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut receipts = WorldReceipts { events, count: 0 };
    let outcome = run_world_claim_exchange(&claim_context(), WorldClaimPorts {
        transport: &mut transport,
        authentication: &mut authentication,
        authority: &mut authority,
        receipts: &mut receipts,
    })
    .expect("claim exchange");
    assert_eq!(outcome.admission.admitted.len(), 1);
    assert!(outcome.admission.selected_claim.is_none());
    assert!(!outcome.admission.head_mutation_authorized);
    assert_eq!(receipts.count, 1);

    let mut transport = ClaimTransport {
        carriers: vec![claim_carrier()],
    };
    let mut authority = ClaimAuthority { admitted: false };
    let mut denied_receipts = WorldReceipts {
        events: Rc::new(RefCell::new(Vec::new())),
        count: 0,
    };
    let denied = run_world_claim_exchange(&claim_context(), WorldClaimPorts {
        transport: &mut transport,
        authentication: &mut authentication,
        authority: &mut authority,
        receipts: &mut denied_receipts,
    })
    .expect("denied claim exchange remains inspectable");
    assert!(denied.admission.admitted.is_empty());
    assert_eq!(denied.admission.denied.len(), 1);
}

// r[verify molten.world_distribution.retention_roots]
// r[verify molten.world_distribution.gc_boundary]
#[test]
fn retention_handoff_preserves_existing_destructive_gates() {
    let (store, commit) = fixture_store();
    let projection = load_world_dag_projection(&store, &commit.commit_ref, &world_bounds()).expect("projection");
    let mut classes = complete_retention_classes();
    classes
        .iter_mut()
        .find(|observation| observation.class == WorldRetentionClass::LegalHold)
        .expect("legal hold")
        .roots
        .push(WorldObjectRef::Commit(commit.commit_ref.clone()));
    let report = project_world_retention(&WorldRetentionProjectionRequest {
        snapshot_ref: reference("retention-snapshot"),
        generation_ref: reference("retention-generation"),
        projection,
        classes,
        remote_leases: Vec::new(),
        edge_inventory_complete: true,
        attribution_inventory_complete: true,
    })
    .expect("retention report");
    let temp = cap_tempfile::tempdir(cap_tempfile::ambient_authority()).expect("retention tempdir");
    let descriptor_path = PathBuf::from(format!("/proc/self/fd/{}", temp.as_raw_fd()));
    let retention_root = std::fs::read_link(descriptor_path).expect("retention host path");
    let existing = DestructiveEvidence {
        requester_ref: Some(reference("retention-requester")),
        policy_refs: vec![reference("retention-policy")],
        authority_refs: vec![reference("deletion-authority")],
        evidence_refs: vec![reference("retention-evidence")],
        retained_refs: Vec::new(),
        remote_peer_refs: Vec::new(),
        remote_refs: Vec::new(),
        reference_index_refs: vec![reference("existing-index")],
        remote_gc_refs: Vec::new(),
        remote_clearance_refs: Vec::new(),
        is_reference_index_complete: true,
    };
    let handoff = handoff_world_retention(WorldRetentionHandoffInput {
        retention_root: &retention_root,
        report: &report,
        object_ref: commit.commit_ref.as_str(),
        object_kind: "world-commit",
        retention_class: CLASS_DURABLE_VALUE,
        action: ACTION_DELETE,
        existing_evidence: &existing,
    })
    .expect("retention handoff");
    assert_eq!(handoff.plan.decision, "deny");
    assert!(!handoff.report_granted_deletion_authority);
    assert!(handoff.plan.evidence.retained_refs.contains(&commit.commit_ref.as_str().to_string()));
}
