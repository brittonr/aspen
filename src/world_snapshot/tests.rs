use molten_core::world_commit::SnapshotCohortRef;
use molten_core::world_commit::SnapshotProfileRef;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_snapshot::*;

use super::*;

const DIGEST_HEX_LENGTH: usize = 64;

fn digest(byte: char) -> String {
    format!("blake3:{}", byte.to_string().repeat(DIGEST_HEX_LENGTH))
}

fn logical_descriptor() -> SnapshotDescriptor {
    let facts = LOGICAL_COHORT_FACTS
        .iter()
        .map(|kind| CohortFact {
            kind: *kind,
            identity: digest('c'),
        })
        .collect();
    let components = LOGICAL_COMPONENTS
        .iter()
        .map(|kind| SnapshotComponent {
            kind: *kind,
            identity: digest('d'),
            root: kind.root_kind().map(|root_kind| WorldRootRef::parse(root_kind, digest('d')).expect("valid root")),
            owner: ComponentOwner::Molten,
        })
        .collect();
    SnapshotDescriptor {
        class: SnapshotClass::Logical,
        commit_ref: WorldCommitRef::new(digest('a')).expect("valid commit"),
        profile_ref: SnapshotProfileRef::new(digest('b')).expect("valid profile"),
        cohort: SnapshotCohort {
            cohort_ref: SnapshotCohortRef::new(digest('c')).expect("valid cohort"),
            facts,
        },
        components,
        contains_live_handle: false,
        synchronization: None,
    }
}

#[test]
fn canonical_snapshot_records_are_stable_and_domain_separated() {
    let descriptor = logical_descriptor();
    let first = canonical_snapshot_descriptor(&descriptor).expect("descriptor");
    let repeated = canonical_snapshot_descriptor(&descriptor).expect("repeated descriptor");
    let inventory = canonical_snapshot_inventory(&inventory_for(&descriptor)).expect("inventory");
    assert_eq!(first.artifact_ref, repeated.artifact_ref);
    assert_eq!(first.bytes, repeated.bytes);
    assert_ne!(first.artifact_ref, inventory.artifact_ref);

    let report = validate_snapshot(&descriptor, &descriptor.cohort);
    let compatibility = canonical_snapshot_compatibility(&report).expect("compatibility");
    let plan = plan_restore(&descriptor, &descriptor.cohort, true).expect("restore plan");
    let restore = canonical_snapshot_restore_plan(&plan).expect("canonical restore");
    assert_ne!(compatibility.artifact_ref, restore.artifact_ref);
}

#[test]
fn canonical_descriptor_normalizes_input_order() {
    let first_descriptor = logical_descriptor();
    let mut reordered = first_descriptor.clone();
    reordered.components.reverse();
    reordered.cohort.facts.reverse();
    let first = canonical_snapshot_descriptor(&first_descriptor).expect("first descriptor");
    let second = canonical_snapshot_descriptor(&reordered).expect("reordered descriptor");
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(first.artifact_ref, second.artifact_ref);
}

#[test]
fn denied_receipt_cannot_overclaim_a_restore_plan() {
    let receipt = SnapshotReceipt {
        decision: SnapshotReceiptDecision::Denied,
        descriptor_ref: digest('1'),
        compatibility_ref: digest('2'),
        restore_plan_ref: Some(digest('3')),
        clone_plan_ref: None,
        current_admission_ref: None,
        issues: vec!["current-authority-denied".to_string()],
        non_claims: SNAPSHOT_NON_CLAIMS.iter().map(ToString::to_string).collect(),
    };
    let issue = canonical_snapshot_receipt(&receipt).expect_err("denied receipt cannot bind output plan");
    assert!(issue.to_string().contains("snapshot receipt denied"));
}
