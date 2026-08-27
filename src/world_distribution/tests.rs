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

#[derive(Default)]
struct FixtureStore {
    commits: BTreeMap<String, Vec<u8>>,
    roots: BTreeMap<String, Vec<u8>>,
}

impl WorldCommitPublicationPort for FixtureStore {
    fn publish_commit(
        &mut self,
        _commit_ref: &WorldCommitRef,
        _canonical_bytes: &[u8],
    ) -> std::result::Result<PublicationOutcome, WorldCommitPortError> {
        Err(WorldCommitPortError::new("fixture", "publication disabled"))
    }

    fn read_commit(&self, commit_ref: &WorldCommitRef) -> std::result::Result<Vec<u8>, WorldCommitPortError> {
        self.commits
            .get(commit_ref.as_str())
            .cloned()
            .ok_or_else(|| WorldCommitPortError::new("fixture", "missing commit"))
    }
}

impl WorldImmutableObjectPort for FixtureStore {
    fn contains_root(&self, root: &WorldRootRef) -> std::result::Result<bool, WorldCommitPortError> {
        Ok(self.roots.contains_key(root.as_str()))
    }

    fn persist_root(
        &mut self,
        _root: &WorldRootRef,
        _canonical_bytes: &[u8],
    ) -> std::result::Result<(), WorldCommitPortError> {
        Err(WorldCommitPortError::new("fixture", "persistence disabled"))
    }

    fn read_root(&self, root: &WorldRootRef) -> std::result::Result<Vec<u8>, WorldCommitPortError> {
        self.roots
            .get(root.as_str())
            .cloned()
            .ok_or_else(|| WorldCommitPortError::new("fixture", "missing root"))
    }
}

fn fixture_store() -> (FixtureStore, CanonicalWorldCommit) {
    let mut store = FixtureStore::default();
    let roots = SnapshotProfileKind::Logical
        .required_roots()
        .iter()
        .map(|kind| {
            let value =
                crate::preserves_rail::record("world-distribution-root-fixture", vec![crate::preserves_rail::string(
                    kind.as_str(),
                )]);
            let bytes = crate::preserves_rail::canonical_bytes(&value).expect("root bytes");
            let reference = crate::preserves_rail::content_ref_from_bytes(&bytes);
            let root = WorldRootRef::parse(*kind, reference).expect("root ref");
            store.roots.insert(root.as_str().to_string(), bytes);
            root
        })
        .collect::<Vec<_>>();
    let core = WorldCommitCore {
        version: WorldCommitVersion::V1,
        profile: SnapshotProfile {
            kind: SnapshotProfileKind::Logical,
            profile_ref: SnapshotProfileRef::new(reference("logical-profile")).expect("profile ref"),
            cohort_ref: None,
        },
        parents: Vec::new(),
        roots,
        completeness: CompletenessClaim::for_profile(SnapshotProfileKind::Logical),
    };
    let commit = canonical_world_commit(&core, &world_bounds()).expect("canonical commit");
    store.commits.insert(commit.commit_ref.as_str().to_string(), commit.bytes.clone());
    (store, commit)
}

struct ReplicationTransport {
    corrupt: bool,
}

impl TransportPort for ReplicationTransport {
    fn fetch(&mut self, action: &Action) -> Result<TransferOutcome> {
        Ok(TransferOutcome::Received(TransferEnvelope {
            transfer_ref: reference(if self.corrupt { "corrupt-transfer" } else { "transfer" }),
            transport_verification_ref: reference("transport-verification"),
            operation_id: action.operation_id.clone(),
            content_ref: action.content_ref.clone(),
            manifest_ref: reference("transport-manifest"),
            source_peer: action.source_peer.clone().unwrap_or_else(|| reference("source-peer")),
            target_peer: action.target_peer.clone(),
            generation: CURRENT_GENERATION,
            membership_epoch: CURRENT_GENERATION,
            placement_epoch: CURRENT_GENERATION,
            encoded_bytes: action.encoded_bytes,
            protected: action.preserve_protected_form,
        }))
    }
}

struct ReplicationContent {
    corrupt: bool,
}

impl ContentPort for ReplicationContent {
    fn inventory(&mut self, _manifest: &Manifest) -> Result<Inventory> {
        Ok(Inventory::default())
    }

    fn verify(&mut self, action: &Action, envelope: &TransferEnvelope) -> Result<VerificationObservation> {
        Ok(VerificationObservation {
            verification_ref: reference("content-verification"),
            operation_id: action.operation_id.clone(),
            replica: Replica {
                content_ref: action.content_ref.clone(),
                peer_id: action.target_peer.clone(),
                fault_domain: action.fault_domain.clone(),
                generation: CURRENT_GENERATION,
                membership_epoch: CURRENT_GENERATION,
                placement_epoch: CURRENT_GENERATION,
                present: true,
                identity_verified: !self.corrupt,
                pinned: true,
                protected: envelope.protected,
                manifest_ref: envelope.manifest_ref.clone(),
                cleanup_clearance_ref: None,
            },
            identity_verified: !self.corrupt,
            authorization_admitted: true,
        })
    }

    fn cleanup(&mut self, _action: &Action, _admission: &CleanupObservation) -> Result<String> {
        Ok(reference("cleanup"))
    }
}

struct DagAuthority;

impl DagAuthorityPort for DagAuthority {
    fn observe_authority(&mut self, plan: &DagSyncPlan) -> Result<DagAuthorityObservation> {
        Ok(DagAuthorityObservation {
            authority_ref: reference("dag-authority"),
            plan_ref: plan.plan_ref.clone(),
            epoch_ref: plan.epoch_ref.clone(),
            generation: plan.generation,
            admitted: true,
        })
    }
}

struct DagResources;

impl DagResourcePort for DagResources {
    fn reserve(&mut self, plan: &DagSyncPlan) -> Result<DagResourceObservation> {
        Ok(DagResourceObservation {
            reservation_ref: reference("dag-resources"),
            plan_ref: plan.plan_ref.clone(),
            admitted: true,
        })
    }
}

struct Progress {
    loaded: Option<DagSyncProgress>,
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl DagProgressPort for Progress {
    fn load(&mut self, _epoch_ref: &DagEpochRef) -> Result<Option<DagSyncProgress>> {
        Ok(self.loaded.clone())
    }

    fn store(&mut self, progress: &DagSyncProgress) -> Result<String> {
        self.loaded = Some(progress.clone());
        self.events.borrow_mut().push("progress");
        Ok(reference("durable-progress"))
    }
}

struct Observations;

impl DagObservationPort for Observations {
    fn publish_response(&mut self, _response: &crate::dag_sync::CanonicalDagRecord) -> Result<()> {
        Ok(())
    }

    fn publish_progress(&mut self, _progress: &crate::dag_sync::CanonicalDagRecord) -> Result<()> {
        Ok(())
    }
}

struct DagReceipts {
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl DagReceiptPort for DagReceipts {
    fn publish_receipt(&mut self, _receipt: &crate::dag_sync::CanonicalDagRecord) -> Result<()> {
        self.events.borrow_mut().push("dag-receipt");
        Ok(())
    }
}

struct WorldReceipts {
    events: Rc<RefCell<Vec<&'static str>>>,
    count: usize,
}

impl WorldDistributionReceiptPort for WorldReceipts {
    fn publish_world_distribution_receipt(&mut self, _receipt: &CanonicalWorldDistributionRecord) -> Result<()> {
        self.events.borrow_mut().push("world-receipt");
        self.count = self.count.saturating_add(1);
        Ok(())
    }
}

struct ClaimTransport {
    carriers: Vec<WorldClaimCarrier>,
}

impl WorldClaimTransportPort for ClaimTransport {
    fn receive_claims(&mut self, _maximum: usize) -> Result<Vec<WorldClaimCarrier>> {
        Ok(self.carriers.clone())
    }
}

struct ClaimAuthentication;

impl WorldClaimAuthenticationPort for ClaimAuthentication {
    fn authenticate_claim(&mut self, carrier: &WorldClaimCarrier) -> Result<WorldHeadAuthenticationObservation> {
        Ok(WorldHeadAuthenticationObservation {
            statement_ref: WorldHeadStatementRef::new(reference("claim-statement")).expect("statement ref"),
            decision_ref: WorldHeadAuthenticationDecisionRef::new(reference("claim-authentication"))
                .expect("authentication ref"),
            passed: true,
            purpose_matches: true,
            policy_matches: true,
            signers: vec![WorldHeadSignerObservation {
                key_identity_ref: reference("claim-key"),
                role: WorldHeadSignerRole::Maintainer,
                authenticated: true,
                current: true,
                revoked: false,
                authority_admitted: carrier.claim.successor_generation == NEXT_GENERATION,
            }],
        })
    }
}

struct ClaimAuthority {
    admitted: bool,
}

impl WorldClaimAuthorityPort for ClaimAuthority {
    fn observe_claim_authority(&mut self, _carrier: &WorldClaimCarrier) -> Result<WorldClaimAuthorityFacts> {
        Ok(WorldClaimAuthorityFacts {
            authority: WorldHeadAuthorityObservation {
                authority_ref: WorldHeadAuthorityRef::new(reference("claim-authority")).expect("authority ref"),
                policy_ref: claim_policy_ref(),
                admitted: self.admitted,
                observed_generation: CURRENT_GENERATION,
            },
            currentness: WorldHeadCurrentnessObservation {
                durable_generation_observed: true,
                independent_ref: None,
            },
            evidence_ref: reference("claim-authority-evidence"),
        })
    }
}

fn claim_carrier() -> WorldClaimCarrier {
    WorldClaimCarrier {
        peer_ref: reference("claim-peer"),
        claim_ref: WorldHeadClaimRef::new(reference("claim-ref")).expect("claim ref"),
        claim: WorldHeadClaim {
            branch_id: branch(),
            branch_class: WorldBranchClass::Local,
            expected_head: Some(commit_ref("claim-root")),
            successor_head: commit_ref("claim-successor"),
            expected_generation: CURRENT_GENERATION,
            successor_generation: NEXT_GENERATION,
            purpose: WorldHeadPurpose::Advance,
            policy_ref: claim_policy_ref(),
            source_heads: Vec::new(),
        },
        encoded_bytes: ROOT_BYTES,
    }
}

fn claim_context() -> WorldClaimAdmissionContext {
    let root = commit_ref("claim-root");
    let successor = commit_ref("claim-successor");
    WorldClaimAdmissionContext {
        current: Some(WorldHeadState {
            branch_id: branch(),
            branch_class: WorldBranchClass::Local,
            head: root.clone(),
            generation: CURRENT_GENERATION,
            policy_ref: claim_policy_ref(),
        }),
        history: vec![
            WorldCommitHistoryNode {
                commit: root.clone(),
                parents: Vec::new(),
            },
            WorldCommitHistoryNode {
                commit: successor,
                parents: vec![root],
            },
        ],
        policy: WorldHeadPolicy {
            policy_ref: claim_policy_ref(),
            allowed_branch_classes: BTreeSet::from([WorldBranchClass::Local]),
            allowed_purposes: BTreeSet::from([WorldHeadPurpose::Advance]),
            allowed_signer_roles: BTreeSet::from([WorldHeadSignerRole::Maintainer]),
            signature_threshold: MINIMUM_REPLICAS,
            max_conflicts: MAX_WORLD_HEAD_CONFLICTS,
            allow_recovery: false,
            require_independent_recovery_currentness: false,
        },
        bounds: WorldHeadBounds::standard(),
        max_claims: MAX_WORLD_DISTRIBUTION_CLAIMS,
    }
}

fn source_inventory(manifest: &Manifest, profile: &WorldReplicationProfile, source_peer: &str) -> Inventory {
    Inventory {
        replicas: manifest
            .contents
            .iter()
            .map(|content| Replica {
                content_ref: content.content_ref.clone(),
                peer_id: source_peer.to_string(),
                fault_domain: "source-domain".to_string(),
                generation: profile.generation,
                membership_epoch: profile.membership_epoch,
                placement_epoch: profile.placement_epoch,
                present: true,
                identity_verified: true,
                pinned: true,
                protected: true,
                manifest_ref: content.manifest_ref.clone(),
                cleanup_clearance_ref: None,
            })
            .collect(),
    }
}

fn target_peer(profile: &WorldReplicationProfile) -> Peer {
    Peer {
        peer_id: reference("target-peer"),
        fault_domain: "target-domain".to_string(),
        membership_epoch: profile.membership_epoch,
        placement_epoch: profile.placement_epoch,
        available: true,
        capacity_bytes: MAX_WORLD_DISTRIBUTION_BYTES,
    }
}

fn replication_profile() -> WorldReplicationProfile {
    WorldReplicationProfile {
        service_id: "world-distribution-shell-test".to_string(),
        generation: CURRENT_GENERATION,
        membership_epoch: CURRENT_GENERATION,
        placement_epoch: CURRENT_GENERATION,
        authority_ref: reference("replication-authority"),
        identity_ref: reference("replication-identity"),
        content_profile_ref: reference("content-profile"),
        transport_profile_ref: reference("transport-profile"),
        retention_policy_ref: reference("retention-policy"),
        evidence_profile_ref: reference("evidence-profile"),
        desired_replicas: DESIRED_REPLICAS,
        minimum_verified_replicas: MINIMUM_REPLICAS,
        minimum_fault_domains: MINIMUM_DOMAINS,
        max_attempts: MAX_ATTEMPTS,
        max_concurrent_transfers: TRANSFER_LIMIT,
        max_transfer_bytes: MAX_WORLD_DISTRIBUTION_BYTES,
        max_queue_depth: QUEUE_LIMIT,
        max_timers: TIMER_LIMIT,
    }
}

fn sync_context() -> WorldSyncContext {
    WorldSyncContext {
        inventory: Vec::new(),
        progress: None,
        peers: Vec::new(),
        epoch_ref: DagEpochRef::new(reference("world-sync-epoch")).expect("epoch ref"),
        generation: CURRENT_GENERATION,
        policy_ref: DagPolicyRef::new(reference("world-sync-policy")).expect("policy ref"),
        strategy: DagSyncStrategy::Full,
        bounds: DagBounds {
            max_nodes: MAX_WORLD_DISTRIBUTION_OBJECTS,
            max_edges: MAX_DAG_EDGES,
            max_roots: MAX_DAG_ROOTS,
            max_depth: MAX_DAG_DEPTH,
            max_bytes: MAX_WORLD_DISTRIBUTION_BYTES,
            max_steps: MAX_WORLD_DISTRIBUTION_OBJECTS,
            max_peers: MAX_DAG_PEERS,
        },
    }
}

fn complete_retention_classes() -> Vec<WorldRetentionClassObservation> {
    WorldRetentionClass::all()
        .into_iter()
        .map(|class| WorldRetentionClassObservation {
            class,
            owner_ref: reference(&format!("owner:{}", class.as_str())),
            roots: Vec::new(),
            observed: true,
            evidence_refs: vec![reference(&format!("evidence:{}", class.as_str()))],
        })
        .collect()
}

fn world_bounds() -> WorldCommitBounds {
    WorldCommitBounds {
        max_parents: MAX_WORLD_COMMIT_PARENTS,
        max_roots: MAX_WORLD_COMMIT_ROOTS,
        max_revision_fences: MAX_WORLD_COMMIT_REVISION_FENCES,
        max_closure_objects: MAX_WORLD_DISTRIBUTION_OBJECTS,
    }
}

fn branch() -> WorldBranchId {
    WorldBranchId::new("main").expect("branch")
}

fn claim_policy_ref() -> WorldHeadPolicyRef {
    WorldHeadPolicyRef::new(reference("claim-policy")).expect("claim policy ref")
}

fn commit_ref(label: &str) -> WorldCommitRef {
    WorldCommitRef::new(reference(label)).expect("commit ref")
}

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}
