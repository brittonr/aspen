use ed25519_dalek::SigningKey;
use molten_core::world_commit::SnapshotCohortRef;
use molten_core::world_commit::SnapshotProfileRef;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_snapshot::*;
use molten_node_host::node_state::DirectoryRoot;
use molten_node_host::node_state::NamespaceKind;
use molten_node_host::node_state::RelativePath;

use super::super::*;

fn digest(byte: char) -> String {
    format!("blake3:{}", byte.to_string().repeat(64))
}

fn root() -> (tempfile::TempDir, DirectoryRoot) {
    let temporary = tempfile::tempdir().expect("temporary root");
    let root = DirectoryRoot::open(temporary.path()).expect("state root");
    root.create_layout().expect("layout");
    (temporary, root)
}

fn populate_bundle(root: &DirectoryRoot) {
    let storage = root.namespace(NamespaceKind::Storage).expect("storage");
    for kind in LOGICAL_COMPONENTS {
        let directory = RelativePath::parse(&format!("world-snapshot/bundle/{}", kind.as_str())).expect("dir");
        storage.create_dir_all(&directory).expect("create dir");
        storage.write(&directory.join("state.bin").expect("file"), kind.as_str().as_bytes()).expect("write");
    }
}

fn descriptor(adapter: &LocalLogicalSnapshot) -> SnapshotDescriptor {
    let components = LOGICAL_COMPONENTS
        .iter()
        .map(|kind| {
            let identity = adapter.component_identity(*kind).expect("identity");
            SnapshotComponent {
                kind: *kind,
                root: kind.root_kind().map(|root_kind| WorldRootRef::parse(root_kind, identity.clone()).expect("root")),
                identity,
                owner: ComponentOwner::Molten,
            }
        })
        .collect();
    let facts = LOGICAL_COHORT_FACTS
        .iter()
        .map(|kind| CohortFact {
            kind: *kind,
            identity: digest('c'),
        })
        .collect();
    SnapshotDescriptor {
        class: SnapshotClass::Logical,
        commit_ref: WorldCommitRef::new(digest('a')).expect("commit"),
        profile_ref: SnapshotProfileRef::new(digest('b')).expect("profile"),
        cohort: SnapshotCohort {
            cohort_ref: SnapshotCohortRef::new(digest('c')).expect("cohort"),
            facts,
        },
        components,
        contains_live_handle: false,
        synchronization: None,
    }
}

fn write_admission(root: &DirectoryRoot, bytes: &[u8]) {
    let storage = root.namespace(NamespaceKind::Storage).expect("storage");
    storage.create_dir_all(&RelativePath::parse("world-snapshot").expect("dir")).expect("create");
    storage
        .write(&RelativePath::parse("world-snapshot/admission.json").expect("admission"), bytes)
        .expect("write admission");
}

#[test]
fn local_logical_restore_materializes_and_activates_a_verified_bundle() {
    let (temporary, root) = root();
    populate_bundle(&root);
    let signer = SigningKey::from_bytes(&[7_u8; 32]);
    let adapter = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let descriptor = descriptor(&adapter);
    let descriptor_ref = canonical_snapshot_descriptor(&descriptor).expect("descriptor").artifact_ref;
    write_admission(
        &root,
        &sign_local_admission(
            &signer,
            &descriptor_ref,
            descriptor.profile_ref.as_str(),
            descriptor.cohort.cohort_ref.as_str(),
            1,
        )
        .expect("admission"),
    );

    let mut materialization = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut admission = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut handles = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut runtime = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut receipts = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let outcome = restore_logical_snapshot(&descriptor, &descriptor.cohort, LogicalSnapshotPorts {
        materialization: &mut materialization,
        admission: &mut admission,
        handles: &mut handles,
        runtime: &mut runtime,
        receipts: &mut receipts,
    })
    .expect("restore");
    assert!(!outcome.observations.is_empty());
    let storage = root.namespace(NamespaceKind::Storage).expect("storage");
    let restored = storage
        .read(&RelativePath::parse("world-snapshot/restored/durable-state/state.bin").expect("restored"), 64)
        .expect("restored bytes");
    assert_eq!(restored, b"durable-state");
    assert!(
        storage
            .try_exists(&RelativePath::parse("world-snapshot/active.json").expect("active"))
            .expect("exists")
    );
    let receipt = RelativePath::parse(&format!(
        "world-snapshot/receipts/{}.preserves",
        outcome.receipt.artifact_ref.trim_start_matches("blake3:")
    ))
    .expect("receipt");
    assert!(storage.try_exists(&receipt).expect("exists"));
}

#[test]
fn local_logical_restore_denies_tampered_bundle_and_untrusted_admission() {
    let (temporary, root) = root();
    populate_bundle(&root);
    let signer = SigningKey::from_bytes(&[7_u8; 32]);
    let adapter = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let descriptor = descriptor(&adapter);
    let descriptor_ref = canonical_snapshot_descriptor(&descriptor).expect("descriptor").artifact_ref;
    let storage = root.namespace(NamespaceKind::Storage).expect("storage");
    storage
        .write(&RelativePath::parse("world-snapshot/bundle/durable-state/state.bin").expect("file"), b"tampered")
        .expect("tamper");
    write_admission(
        &root,
        &sign_local_admission(
            &signer,
            &descriptor_ref,
            descriptor.profile_ref.as_str(),
            descriptor.cohort.cohort_ref.as_str(),
            1,
        )
        .expect("admission"),
    );
    let mut materialization = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut admission = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut handles = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut runtime = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut receipts = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    assert!(
        restore_logical_snapshot(&descriptor, &descriptor.cohort, LogicalSnapshotPorts {
            materialization: &mut materialization,
            admission: &mut admission,
            handles: &mut handles,
            runtime: &mut runtime,
            receipts: &mut receipts,
        })
        .is_err()
    );

    let wrong = SigningKey::from_bytes(&[9_u8; 32]);
    write_admission(
        &root,
        &sign_local_admission(
            &wrong,
            &descriptor_ref,
            descriptor.profile_ref.as_str(),
            descriptor.cohort.cohort_ref.as_str(),
            2,
        )
        .expect("admission"),
    );
    let mut materialization = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut admission = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut handles = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut runtime = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    let mut receipts = LocalLogicalSnapshot::open(temporary.path(), signer.verifying_key()).expect("adapter");
    assert!(
        restore_logical_snapshot(&descriptor, &descriptor.cohort, LogicalSnapshotPorts {
            materialization: &mut materialization,
            admission: &mut admission,
            handles: &mut handles,
            runtime: &mut runtime,
            receipts: &mut receipts,
        })
        .is_err()
    );
}
