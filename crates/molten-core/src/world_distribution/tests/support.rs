use std::collections::BTreeSet;

use super::super::*;
use crate::dag_sync::DagBounds;
use crate::dag_sync::DagEpochRef;
use crate::dag_sync::DagPeerId;
use crate::dag_sync::DagPolicyRef;
use crate::dag_sync::DagSchemaRef;
use crate::dag_sync::DagSyncStrategy;
use crate::world_commit::CompletenessClaim;
use crate::world_commit::SnapshotProfile;
use crate::world_commit::SnapshotProfileKind;
use crate::world_commit::SnapshotProfileRef;
use crate::world_commit::WorldCommitBounds;
use crate::world_commit::WorldCommitCore;
use crate::world_commit::WorldCommitRef;
use crate::world_commit::WorldCommitVersion;
use crate::world_commit::WorldRootRef;
use crate::world_commit::identify_world_commit;
use crate::world_head::MAX_WORLD_HEAD_CONFLICTS;
use crate::world_head::WorldBranchClass;
use crate::world_head::WorldBranchId;
use crate::world_head::WorldCommitHistoryNode;
use crate::world_head::WorldHeadAuthenticationDecisionRef;
use crate::world_head::WorldHeadAuthenticationObservation;
use crate::world_head::WorldHeadAuthorityObservation;
use crate::world_head::WorldHeadAuthorityRef;
use crate::world_head::WorldHeadBounds;
use crate::world_head::WorldHeadClaim;
use crate::world_head::WorldHeadClaimRef;
use crate::world_head::WorldHeadCurrentnessObservation;
use crate::world_head::WorldHeadPolicy;
use crate::world_head::WorldHeadPolicyRef;
use crate::world_head::WorldHeadPurpose;
use crate::world_head::WorldHeadSignerObservation;
use crate::world_head::WorldHeadSignerRole;
use crate::world_head::WorldHeadState;
use crate::world_head::WorldHeadStatementRef;

pub(super) const BASE_BYTES: &[u8] = b"world-distribution-base";
pub(super) const CHILD_BYTES: &[u8] = b"world-distribution-child";
pub(super) const ALTERNATE_BYTES: &[u8] = b"world-distribution-alternate";
pub(super) const ENCODED_ROOT_BYTES: u64 = 64;
pub(super) const CURRENT_GENERATION: u64 = 1;
pub(super) const NEXT_GENERATION: u64 = CURRENT_GENERATION + 1;
pub(super) const DESIRED_REPLICAS: usize = 2;
const MINIMUM_REPLICAS: usize = 1;
const MINIMUM_DOMAINS: usize = 2;
const MAX_ATTEMPTS: u32 = 3;
const TRANSFER_LIMIT: usize = 64;
const QUEUE_LIMIT: usize = 128;
const TIMER_LIMIT: usize = 16;

pub(super) fn fixture_projection() -> WorldDagProjection {
    project_world_dag(&fixture_projection_input()).expect("world DAG projection")
}

pub(super) fn fixture_projection_input() -> WorldDagProjectionInput {
    let roots = logical_roots();
    let base_ref = commit_ref(BASE_BYTES);
    let child_ref = commit_ref(CHILD_BYTES);
    WorldDagProjectionInput {
        requested: child_ref.clone(),
        commits: vec![
            WorldCommitObject {
                commit_ref: base_ref.clone(),
                core: logical_core(Vec::new(), &roots),
                canonical_bytes: BASE_BYTES.to_vec(),
            },
            WorldCommitObject {
                commit_ref: child_ref,
                core: logical_core(vec![base_ref], &roots),
                canonical_bytes: CHILD_BYTES.to_vec(),
            },
        ],
        roots: roots
            .iter()
            .map(|root| WorldRootObject {
                root: root.clone(),
                schema_ref: DagSchemaRef::new(reference(&format!("schema:{}", root.kind().as_str())))
                    .expect("root schema ref"),
                encoded_bytes: ENCODED_ROOT_BYTES,
            })
            .collect(),
        bounds: world_bounds(),
    }
}

pub(super) fn sync_context(inventory: Vec<WorldObjectRef>, strategy: DagSyncStrategy) -> WorldSyncContext {
    WorldSyncContext {
        inventory,
        progress: None,
        peers: vec![DagPeerId::new(reference("dag-peer")).expect("peer ref")],
        epoch_ref: DagEpochRef::new(reference("dag-epoch")).expect("epoch ref"),
        generation: CURRENT_GENERATION,
        policy_ref: DagPolicyRef::new(reference("dag-policy")).expect("policy ref"),
        strategy,
        bounds: DagBounds {
            max_nodes: MAX_WORLD_DISTRIBUTION_OBJECTS,
            max_edges: crate::dag_sync::MAX_DAG_EDGES,
            max_roots: crate::dag_sync::MAX_DAG_ROOTS,
            max_depth: crate::dag_sync::MAX_DAG_DEPTH,
            max_bytes: MAX_WORLD_DISTRIBUTION_BYTES,
            max_steps: MAX_WORLD_DISTRIBUTION_OBJECTS,
            max_peers: crate::dag_sync::MAX_DAG_PEERS,
        },
    }
}

pub(super) fn replication_profile() -> WorldReplicationProfile {
    WorldReplicationProfile {
        service_id: "world-distribution-test".to_string(),
        generation: CURRENT_GENERATION,
        membership_epoch: CURRENT_GENERATION,
        placement_epoch: CURRENT_GENERATION,
        authority_ref: reference("replication-authority"),
        identity_ref: reference("replication-identity"),
        content_profile_ref: reference("replication-content-profile"),
        transport_profile_ref: reference("replication-transport-profile"),
        retention_policy_ref: reference("replication-retention-policy"),
        evidence_profile_ref: reference("replication-evidence-profile"),
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

pub(super) fn claim_request(is_authority_admitted: bool) -> WorldClaimAdmissionRequest {
    let root = commit_ref(BASE_BYTES);
    let left = commit_ref(CHILD_BYTES);
    let right = commit_ref(ALTERNATE_BYTES);
    let policy_ref = WorldHeadPolicyRef::new(reference("claim-policy")).expect("claim policy");
    let branch = WorldBranchId::new("main").expect("branch");
    let current = WorldHeadState {
        branch_id: branch.clone(),
        branch_class: WorldBranchClass::Local,
        head: root.clone(),
        generation: CURRENT_GENERATION,
        policy_ref: policy_ref.clone(),
    };
    let policy = WorldHeadPolicy {
        policy_ref: policy_ref.clone(),
        allowed_branch_classes: BTreeSet::from([WorldBranchClass::Local]),
        allowed_purposes: BTreeSet::from([WorldHeadPurpose::Advance]),
        allowed_signer_roles: BTreeSet::from([WorldHeadSignerRole::Maintainer]),
        signature_threshold: MINIMUM_REPLICAS,
        max_conflicts: MAX_WORLD_HEAD_CONFLICTS,
        allow_recovery: false,
        require_independent_recovery_currentness: false,
    };
    let history = vec![
        WorldCommitHistoryNode {
            commit: root.clone(),
            parents: Vec::new(),
        },
        WorldCommitHistoryNode {
            commit: left.clone(),
            parents: vec![root.clone()],
        },
        WorldCommitHistoryNode {
            commit: right.clone(),
            parents: vec![root.clone()],
        },
    ];
    let claims = [("left-claim", left), ("right-claim", right)]
        .into_iter()
        .map(|(label, successor)| remote_claim(label, successor, &root, &branch, &policy_ref, is_authority_admitted))
        .collect();
    WorldClaimAdmissionRequest {
        claims,
        current: Some(current),
        history,
        policy,
        bounds: WorldHeadBounds::standard(),
        max_claims: MAX_WORLD_DISTRIBUTION_CLAIMS,
    }
}

pub(super) fn complete_retention_classes() -> Vec<WorldRetentionClassObservation> {
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

pub(super) fn commit_ref(bytes: &[u8]) -> WorldCommitRef {
    identify_world_commit(bytes).expect("commit fixture identity")
}

pub(super) fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn logical_core(parents: Vec<WorldCommitRef>, roots: &[WorldRootRef]) -> WorldCommitCore {
    WorldCommitCore {
        version: WorldCommitVersion::V1,
        profile: SnapshotProfile {
            kind: SnapshotProfileKind::Logical,
            profile_ref: SnapshotProfileRef::new(reference("logical-profile")).expect("profile ref"),
            cohort_ref: None,
        },
        parents,
        roots: roots.to_vec(),
        completeness: CompletenessClaim::for_profile(SnapshotProfileKind::Logical),
    }
}

fn logical_roots() -> Vec<WorldRootRef> {
    SnapshotProfileKind::Logical
        .required_roots()
        .iter()
        .map(|kind| WorldRootRef::parse(*kind, reference(&format!("root:{}", kind.as_str()))).expect("root ref"))
        .collect()
}

fn world_bounds() -> WorldCommitBounds {
    WorldCommitBounds {
        max_parents: crate::world_commit::MAX_WORLD_COMMIT_PARENTS,
        max_roots: crate::world_commit::MAX_WORLD_COMMIT_ROOTS,
        max_revision_fences: crate::world_commit::MAX_WORLD_COMMIT_REVISION_FENCES,
        max_closure_objects: MAX_WORLD_DISTRIBUTION_OBJECTS,
    }
}

fn remote_claim(
    label: &str,
    successor: WorldCommitRef,
    root: &WorldCommitRef,
    branch: &WorldBranchId,
    policy_ref: &WorldHeadPolicyRef,
    is_authority_admitted: bool,
) -> RemoteWorldHeadClaim {
    RemoteWorldHeadClaim {
        peer_ref: reference(&format!("peer:{label}")),
        claim_ref: WorldHeadClaimRef::new(reference(label)).expect("claim ref"),
        claim: WorldHeadClaim {
            branch_id: branch.clone(),
            branch_class: WorldBranchClass::Local,
            expected_head: Some(root.clone()),
            successor_head: successor,
            expected_generation: CURRENT_GENERATION,
            successor_generation: NEXT_GENERATION,
            purpose: WorldHeadPurpose::Advance,
            policy_ref: policy_ref.clone(),
            source_heads: Vec::new(),
        },
        authentication: WorldHeadAuthenticationObservation {
            statement_ref: WorldHeadStatementRef::new(reference(&format!("statement:{label}"))).expect("statement ref"),
            decision_ref: WorldHeadAuthenticationDecisionRef::new(reference(&format!("authn:{label}")))
                .expect("authentication ref"),
            passed: true,
            purpose_matches: true,
            policy_matches: true,
            signers: vec![WorldHeadSignerObservation {
                key_identity_ref: reference(&format!("key:{label}")),
                role: WorldHeadSignerRole::Maintainer,
                authenticated: true,
                current: true,
                revoked: false,
                authority_admitted: true,
            }],
        },
        authority: WorldHeadAuthorityObservation {
            authority_ref: WorldHeadAuthorityRef::new(reference(&format!("authority:{label}"))).expect("authority ref"),
            policy_ref: policy_ref.clone(),
            admitted: is_authority_admitted,
            observed_generation: CURRENT_GENERATION,
        },
        currentness: WorldHeadCurrentnessObservation {
            durable_generation_observed: true,
            independent_ref: None,
        },
        encoded_bytes: ENCODED_ROOT_BYTES,
    }
}
