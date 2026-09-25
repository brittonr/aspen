
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
