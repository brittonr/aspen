use std::collections::VecDeque;

use molten_core::world_commit::SnapshotCohortRef;
use molten_core::world_commit::SnapshotProfileRef;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_snapshot::*;

use super::super::*;
use crate::error::MoltenError;
use crate::error::Result;

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
    let (parsed, parsed_canonical) =
        parse_canonical_snapshot_descriptor(&first.bytes).expect("parse canonical descriptor");
    assert_eq!(parsed, descriptor);
    assert_eq!(parsed_canonical.artifact_ref, first.artifact_ref);
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
        issues: Vec::new(),
        non_claims: SNAPSHOT_NON_CLAIMS.iter().map(ToString::to_string).collect(),
    };
    let issue = canonical_snapshot_receipt(&receipt).expect_err("denied receipt requires one bounded issue");
    assert!(issue.to_string().contains("snapshot receipt denied"));
}

#[derive(Default)]
struct Materializer {
    unavailable: Option<SnapshotComponentKind>,
}

impl SnapshotMaterializationPort for Materializer {
    fn observe_component(&mut self, component: &SnapshotComponent) -> Result<SnapshotMaterializationObservation> {
        Ok(SnapshotMaterializationObservation {
            component_identity: component.identity.clone(),
            observation_ref: digest('4'),
            available: self.unavailable != Some(component.kind),
            identity_verified: true,
        })
    }
}

struct Authority {
    observations: VecDeque<SnapshotAdmissionObservation>,
}

impl CurrentSnapshotAdmissionPort for Authority {
    fn observe_current(
        &mut self,
        _descriptor: &SnapshotDescriptor,
        _descriptor_ref: &str,
        _destination: &SnapshotCohort,
    ) -> Result<SnapshotAdmissionObservation> {
        self.observations
            .pop_front()
            .ok_or_else(|| MoltenError::invalid_harness("missing authority observation"))
    }
}

#[derive(Default)]
struct Handles {
    calls: usize,
}

impl SnapshotHostHandlePort for Handles {
    fn recreate_handles(&mut self, _descriptor_ref: &str) -> Result<String> {
        self.calls += 1;
        Ok(digest('5'))
    }
}

#[derive(Default)]
struct Runtime {
    restored: Vec<SnapshotRestoreStep>,
    activated: bool,
}

impl LogicalSnapshotRestorePort for Runtime {
    fn restore_component(
        &mut self,
        step: SnapshotRestoreStep,
        _component: &SnapshotComponent,
        _materialization: &SnapshotMaterializationObservation,
    ) -> Result<SnapshotStepObservation> {
        self.restored.push(step);
        Ok(SnapshotStepObservation {
            step,
            observation_ref: digest('6'),
        })
    }

    fn activate(&mut self, _descriptor_ref: &str) -> Result<String> {
        self.activated = true;
        Ok(digest('7'))
    }
}

#[derive(Default)]
struct Receipts {
    published: Vec<String>,
}

impl SnapshotReceiptPort for Receipts {
    fn publish_receipt(&mut self, receipt_ref: &str, canonical_bytes: &[u8]) -> Result<()> {
        assert!(!canonical_bytes.is_empty());
        self.published.push(receipt_ref.to_string());
        Ok(())
    }
}

fn admission(descriptor: &SnapshotDescriptor, descriptor_ref: &str, generation: u64) -> SnapshotAdmissionObservation {
    SnapshotAdmissionObservation {
        admission_ref: digest('8'),
        descriptor_ref: descriptor_ref.to_string(),
        profile_ref: descriptor.profile_ref.as_str().to_string(),
        cohort_ref: descriptor.cohort.cohort_ref.as_str().to_string(),
        generation,
        allowed: true,
    }
}

#[test]
fn logical_restore_rechecks_authority_and_publishes_receipt_last() {
    const INITIAL_GENERATION: u64 = 4;
    const FINAL_GENERATION: u64 = 5;
    const LOGICAL_STATE_RESTORE_COUNT: usize = 7;
    let descriptor = logical_descriptor();
    let descriptor_ref = canonical_snapshot_descriptor(&descriptor).expect("descriptor").artifact_ref;
    let mut materialization = Materializer::default();
    let mut authority = Authority {
        observations: VecDeque::from([
            admission(&descriptor, &descriptor_ref, INITIAL_GENERATION),
            admission(&descriptor, &descriptor_ref, FINAL_GENERATION),
        ]),
    };
    let mut handles = Handles::default();
    let mut runtime = Runtime::default();
    let mut receipts = Receipts::default();
    let outcome = restore_logical_snapshot(&descriptor, &descriptor.cohort, LogicalSnapshotPorts {
        materialization: &mut materialization,
        admission: &mut authority,
        handles: &mut handles,
        runtime: &mut runtime,
        receipts: &mut receipts,
    })
    .expect("logical restore");
    assert_eq!(runtime.restored.len(), LOGICAL_STATE_RESTORE_COUNT);
    assert!(runtime.activated);
    assert_eq!(handles.calls, 1);
    assert_eq!(receipts.published, vec![outcome.receipt.artifact_ref]);
    assert!(authority.observations.is_empty());
}

#[test]
fn stale_recheck_and_unavailable_component_never_activate() {
    const FRESH_GENERATION: u64 = 2;
    const STALE_GENERATION: u64 = 1;
    let descriptor = logical_descriptor();
    let descriptor_ref = canonical_snapshot_descriptor(&descriptor).expect("descriptor").artifact_ref;
    let mut materialization = Materializer::default();
    let mut authority = Authority {
        observations: VecDeque::from([
            admission(&descriptor, &descriptor_ref, FRESH_GENERATION),
            admission(&descriptor, &descriptor_ref, STALE_GENERATION),
        ]),
    };
    let mut handles = Handles::default();
    let mut runtime = Runtime::default();
    let mut receipts = Receipts::default();
    let denied = restore_logical_snapshot(&descriptor, &descriptor.cohort, LogicalSnapshotPorts {
        materialization: &mut materialization,
        admission: &mut authority,
        handles: &mut handles,
        runtime: &mut runtime,
        receipts: &mut receipts,
    });
    assert!(denied.is_err());
    assert!(!runtime.activated);
    assert!(receipts.published.is_empty());

    let mut unavailable = Materializer {
        unavailable: Some(SnapshotComponentKind::Tasks),
    };
    let mut authority = Authority {
        observations: VecDeque::from([admission(&descriptor, &descriptor_ref, FRESH_GENERATION)]),
    };
    let mut handles = Handles::default();
    let mut runtime = Runtime::default();
    let mut receipts = Receipts::default();
    let denied = restore_logical_snapshot(&descriptor, &descriptor.cohort, LogicalSnapshotPorts {
        materialization: &mut unavailable,
        admission: &mut authority,
        handles: &mut handles,
        runtime: &mut runtime,
        receipts: &mut receipts,
    });
    assert!(denied.is_err());
    assert!(runtime.restored.is_empty());
    assert!(!runtime.activated);
    assert_eq!(handles.calls, 0);
    assert!(receipts.published.is_empty());
}
