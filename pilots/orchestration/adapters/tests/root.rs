// SPDX-License-Identifier: AGPL-3.0-or-later
use molten_orchestrator_adapters::CapabilityRoot;

// r[verify molten.batch_orchestration.repairs]
#[test]
fn capability_publication_syncs_the_held_directory_after_rename() {
    let workspace = tempfile::tempdir().unwrap();
    let original = workspace.path().join("original");
    let relocated = workspace.path().join("relocated");
    std::fs::create_dir(&original).unwrap();
    let root = CapabilityRoot::open(&original).unwrap();
    std::fs::rename(&original, &relocated).unwrap();
    std::fs::create_dir(&original).unwrap();

    root.create_new("state", b"initial").unwrap();
    assert_eq!(root.read("state", 16).unwrap(), b"initial");
    root.replace("state", b"committed").unwrap();
    assert_eq!(std::fs::read(relocated.join("state")).unwrap(), b"committed");
    assert!(!original.join("state").exists());
    assert!(root.create_new("state", b"must-not-replace").is_err());
    assert_eq!(root.read("state", 16).unwrap(), b"committed");
}
