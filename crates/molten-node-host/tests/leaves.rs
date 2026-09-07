#[test]
fn local_store_file_operations_keep_regular_leaf_rules() {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let ledger = root.ledger_store().unwrap();
    let store = ledger.root();
    let path = molten_node_host::local_store::LocalStorePath::parse("state.db").unwrap();
    store.write(&path, b"original").unwrap();
    drop(store.open_database_file(&path).unwrap());
    assert_eq!(store.read(&path).unwrap(), b"original");
    store.write(&path, b"new").unwrap();
    assert_eq!(store.read(&path).unwrap(), b"new");
    let directory = molten_node_host::local_store::LocalStorePath::parse("directory").unwrap();
    store.create_dir_all(&directory).unwrap();
    assert!(store.read(&directory).is_err());
    assert!(store.write(&directory, b"bad").is_err());
    assert!(store.open_database_file(&directory).is_err());
    assert_eq!(store.read(&path).unwrap(), b"new");
}

#[cfg(unix)]
#[test]
fn local_store_leaf_links_deny_without_target_changes() {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let ledger = root.ledger_store().unwrap();
    let store = ledger.root();
    let target = molten_node_host::local_store::LocalStorePath::parse("target").unwrap();
    let link = molten_node_host::local_store::LocalStorePath::parse("link").unwrap();
    store.write(&target, b"original").unwrap();
    temp.symlink("target", "ledger/link").unwrap();
    assert!(store.read(&link).is_err());
    assert!(store.write(&link, b"bad").is_err());
    assert!(store.open_database_file(&link).is_err());
    assert_eq!(store.read(&target).unwrap(), b"original");
    assert!(temp.symlink_metadata("ledger/link").unwrap().is_symlink());
}

#[cfg(unix)]
#[test]
fn namespace_modes_distinguish_missing_regular_and_nonregular_leaves() {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let namespace = root.identity().unwrap();
    let path = molten_node_host::node_state::NodeStatePath::parse("fixture").unwrap();
    assert_eq!(namespace.unix_mode(&path).unwrap(), None);
    namespace.write_restricted(&path, b"fixture", 0o600).unwrap();
    assert_eq!(namespace.unix_mode(&path).unwrap().unwrap() & 0o777, 0o600);
    let directory = molten_node_host::node_state::NodeStatePath::parse("directory").unwrap();
    namespace.create_dir_all(&directory).unwrap();
    assert!(namespace.unix_mode(&directory).is_err());
    temp.symlink("fixture", "identity/link").unwrap();
    let link = molten_node_host::node_state::NodeStatePath::parse("link").unwrap();
    assert!(namespace.unix_mode(&link).is_err());
    assert_eq!(namespace.read(&path, 7).unwrap(), b"fixture");
}
