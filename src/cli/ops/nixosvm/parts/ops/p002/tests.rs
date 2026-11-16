#[test]
fn opened_root_survives_bootstrap_replacement() {
    let workspace = crate::test_support::process_workspace("vm-observer-root-replacement").expect("owned workspace");
    let bootstrap = workspace.join("root");
    std::fs::create_dir(&bootstrap).expect("root directory");
    let root = cap_std::fs::Dir::open_ambient_dir(&bootstrap, cap_std::ambient_authority()).expect("opened root");
    root.write("value", b"original").expect("initial protected value");
    let mut block = [0u8; STATE_HASH_BLOCK_BYTES];
    let original = observe_vm_state_tree(&root, &mut block).expect("initial protected tree");
    std::fs::rename(&bootstrap, workspace.join("retained")).expect("rename bootstrap path");
    std::fs::create_dir(&bootstrap).expect("replacement directory");
    std::fs::write(bootstrap.join("value"), b"substitution").expect("replacement value");
    assert_eq!(observe_vm_state_tree(&root, &mut block).expect("retained capability"), original);
    let replacement = cap_std::fs::Dir::open_ambient_dir(&bootstrap, cap_std::ambient_authority())
        .expect("replacement capability");
    assert_ne!(observe_vm_state_tree(&replacement, &mut block).expect("replacement tree"), original);
}

#[test]
fn links_and_byte_exhaustion_cannot_enter_protected_snapshot() {
    let workspace = crate::test_support::process_workspace("vm-observer-member-bounds").expect("owned workspace");
    let root = cap_std::fs::Dir::open_ambient_dir(&*workspace, cap_std::ambient_authority()).expect("opened root");
    root.write("payload", b"abc").expect("protected payload");
    let mut block = [0u8; STATE_HASH_BLOCK_BYTES];
    let (digest, _) = hash_observed_member(&root, std::path::Path::new("payload"), 3, &mut block)
        .expect("exact byte bound");
    assert_eq!(digest.to_hex().as_str(), "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85");
    assert!(hash_observed_member(&root, std::path::Path::new("payload"), 2, &mut block).is_err());
    root.symlink("payload", "alias").expect("test-owned symlink");
    assert!(read_observed_member(&root, "alias").is_err());
    assert!(observe_vm_state_tree(&root, &mut block).is_err());
    root.remove_file("alias").expect("remove test-owned symlink");
    root.create("oversize").expect("sparse member").set_len(STATE_TREE_MAX_BYTES + 1)
        .expect("oversize sparse member");
    assert!(observe_vm_state_tree(&root, &mut block).is_err());
}
