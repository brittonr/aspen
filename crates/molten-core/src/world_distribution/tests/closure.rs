use super::super::*;
use super::support::*;
use crate::content_replication::Inventory;
use crate::content_replication::Peer;
use crate::content_replication::Replica;
use crate::dag_sync::DagSyncProgress;
use crate::dag_sync::DagSyncStrategy;

const SOURCE_CAPACITY: u64 = MAX_WORLD_DISTRIBUTION_BYTES;

// r[verify molten.world_distribution.closure]
// r[verify molten.world_distribution.partial]
#[test]
fn world_projection_and_resume_are_deterministic_and_non_authoritative() {
    let projection = fixture_projection();
    let partial = plan_world_closure(&projection, &sync_context(Vec::new(), DagSyncStrategy::Resumable))
        .expect("partial world closure plan");
    assert!(!partial.complete);
    assert_eq!(partial.missing.len(), projection.objects.len());
    assert!(!partial.activation_authorized);

    let first_verified = partial.shared_plan.missing.first().expect("missing object").clone();
    let mut resume_context = sync_context(Vec::new(), DagSyncStrategy::Resumable);
    resume_context.progress = Some(DagSyncProgress {
        epoch_ref: partial.shared_plan.epoch_ref.clone(),
        generation: partial.shared_plan.generation,
        strategy: partial.shared_plan.strategy,
        policy_ref: partial.request.policy_ref.clone(),
        root_refs: partial.shared_plan.roots.clone(),
        schema_refs: partial.shared_plan.schema_refs.clone(),
        peers: partial.shared_plan.peers.clone(),
        verified: vec![first_verified.clone()],
        steps_completed: 1,
    });
    let resumed = plan_world_closure(&projection, &resume_context).expect("resumed world closure plan");
    assert!(!resumed.shared_plan.missing.contains(&first_verified));
    assert_eq!(resumed.shared_plan.missing.len() + 1, partial.shared_plan.missing.len());

    let inventory = projection.objects.iter().map(|object| object.object_ref.clone()).collect();
    let complete = plan_world_closure(&projection, &sync_context(inventory, DagSyncStrategy::Resumable))
        .expect("complete world closure plan");
    assert!(complete.complete);
    assert!(complete.missing.is_empty());
    let repeated_inventory = projection.objects.iter().map(|object| object.object_ref.clone()).collect();
    let repeated = plan_world_closure(&projection, &sync_context(repeated_inventory, DagSyncStrategy::Resumable))
        .expect("repeated world closure plan");
    assert_eq!(complete.shared_plan.plan_ref, repeated.shared_plan.plan_ref);
}

// r[verify molten.world_distribution.verification]
#[test]
fn projection_rejects_identity_substitution_missing_roots_and_cycles() {
    let input = fixture_projection_input();
    let mut substituted = input.clone();
    let wrong = commit_ref(b"wrong-world-commit");
    substituted.requested = wrong.clone();
    substituted.commits[1].commit_ref = wrong;
    assert!(matches!(
        project_world_dag(&substituted),
        Err(issues) if issues.iter().any(|issue| matches!(issue, WorldDistributionIssue::CommitIdentityMismatch(_)))
    ));

    let mut missing = input.clone();
    let removed = missing.roots.pop().expect("root fixture");
    assert!(matches!(
        project_world_dag(&missing),
        Err(issues) if issues.contains(&WorldDistributionIssue::MissingRoot(removed.root.as_str().to_string()))
    ));

    let mut cyclic = input;
    let child = cyclic.commits[1].commit_ref.clone();
    cyclic.commits[0].core.parents = vec![child];
    let projection = project_world_dag(&cyclic).expect("cycle remains a graph planning concern");
    assert!(matches!(
        plan_world_closure(&projection, &sync_context(Vec::new(), DagSyncStrategy::Full)),
        Err(issues) if issues.iter().any(|issue| matches!(issue, WorldDistributionIssue::DagPlanningDenied(message) if message.contains("Cycle")))
    ));
}

// r[verify molten.world_distribution.closure]
#[test]
fn replication_plan_reuses_generic_policy_and_rejects_unsolicited_objects() {
    let projection = fixture_projection();
    let profile = replication_profile();
    let manifest = world_replication_manifest(&projection, &profile).expect("world replication manifest");
    let source_peer = reference("source-peer");
    let inventory = Inventory {
        replicas: manifest
            .contents
            .iter()
            .map(|content| Replica {
                content_ref: content.content_ref.clone(),
                peer_id: source_peer.clone(),
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
    };
    let request = WorldReplicationPlanRequest {
        profile: profile.clone(),
        inventory: inventory.clone(),
        peers: vec![Peer {
            peer_id: reference("target-peer"),
            fault_domain: "target-domain".to_string(),
            membership_epoch: profile.membership_epoch,
            placement_epoch: profile.placement_epoch,
            available: true,
            capacity_bytes: SOURCE_CAPACITY,
        }],
        history: Vec::new(),
        observed_tick: CURRENT_GENERATION,
    };
    let plan = plan_world_replication(&projection, &request).expect("world replication plan");
    assert!(!plan.activation_authorized);
    assert!(plan.shared_plan.actions.iter().all(|action| action.preserve_protected_form));
    assert!(plan.shared_plan.actions.iter().all(|action| action.cleanup_authority_ref.is_none()));

    let mut unsolicited = request;
    unsolicited.inventory.replicas.push(Replica {
        content_ref: reference("unsolicited"),
        peer_id: source_peer,
        fault_domain: "source-domain".to_string(),
        generation: profile.generation,
        membership_epoch: profile.membership_epoch,
        placement_epoch: profile.placement_epoch,
        present: true,
        identity_verified: true,
        pinned: true,
        protected: true,
        manifest_ref: reference("unsolicited-manifest"),
        cleanup_clearance_ref: None,
    });
    assert!(matches!(
        plan_world_replication(&projection, &unsolicited),
        Err(issues) if issues.iter().any(|issue| matches!(issue, WorldDistributionIssue::ReplicationObjectUnsolicited(_)))
    ));
}

#[test]
fn activation_requires_complete_typed_current_admission() {
    let admitted = admit_world_activation(&WorldActivationFacts {
        closure_complete: true,
        domains_verified: true,
        schemas_admitted: true,
        current_policy_admitted: true,
        current_authority_admitted: true,
        claim_admitted: true,
    });
    assert!(admitted.admitted);

    let denied = admit_world_activation(&WorldActivationFacts {
        closure_complete: false,
        domains_verified: true,
        schemas_admitted: true,
        current_policy_admitted: true,
        current_authority_admitted: false,
        claim_admitted: true,
    });
    assert!(!denied.admitted);
    assert_eq!(denied.diagnostics, vec!["closure-incomplete", "current-authority-denied"]);
}
