use artifact_binding_core::RetirementClassification;

use super::super::*;
use super::support::*;

// r[verify molten.world_distribution.retention_roots]
// r[verify molten.world_distribution.gc_boundary]
#[test]
fn complete_retention_inventory_reports_binding_reachability_without_delete_authority() {
    let projection = fixture_projection();
    let requested = WorldObjectRef::Commit(projection.requested.clone());
    let mut classes = complete_retention_classes();
    classes
        .iter_mut()
        .find(|observation| observation.class == WorldRetentionClass::CurrentHead)
        .expect("current head class")
        .roots
        .push(requested);
    let report = project_world_retention(&WorldRetentionProjectionRequest {
        snapshot_ref: reference("retention-snapshot"),
        generation_ref: reference("retention-generation"),
        projection: projection.clone(),
        classes,
        remote_leases: Vec::new(),
        edge_inventory_complete: true,
        attribution_inventory_complete: true,
    })
    .expect("world retention report");
    assert!(report.reference_index_complete);
    assert_eq!(report.shared_classification, RetirementClassification::Live);
    assert!(report.retained_refs.contains(&projection.requested.as_str().to_string()));
    assert!(report.observation_only);
    assert!(!report.retention_authorized);
    assert!(!report.deletion_authorized);
}

#[test]
fn stale_remote_state_and_missing_classes_keep_retention_incomplete() {
    let projection = fixture_projection();
    let requested = WorldObjectRef::Commit(projection.requested.clone());
    let mut classes = complete_retention_classes();
    classes
        .iter_mut()
        .find(|observation| observation.class == WorldRetentionClass::ActiveExecution)
        .expect("execution class")
        .observed = false;
    let lease_ref = reference("uncertain-lease");
    let report = project_world_retention(&WorldRetentionProjectionRequest {
        snapshot_ref: reference("retention-snapshot-uncertain"),
        generation_ref: reference("retention-generation-uncertain"),
        projection,
        classes,
        remote_leases: vec![WorldRemoteLeaseObservation {
            lease_ref: lease_ref.clone(),
            peer_ref: reference("remote-peer"),
            generation: CURRENT_GENERATION,
            validity_basis_ref: reference("lease-basis"),
            roots: vec![requested],
            state: RemoteLeaseState::Unavailable,
            evidence_ref: reference("lease-evidence"),
        }],
        edge_inventory_complete: true,
        attribution_inventory_complete: true,
    })
    .expect("conservative incomplete report");
    assert!(!report.reference_index_complete);
    assert!(report.missing_classes.contains(&WorldRetentionClass::ActiveExecution));
    assert_eq!(report.unresolved_remote, vec![lease_ref]);
    assert_ne!(report.shared_classification, RetirementClassification::Retired);
    assert!(!report.deletion_authorized);
}
