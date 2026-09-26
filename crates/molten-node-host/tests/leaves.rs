#[test]
fn local_store_file_operations_keep_regular_leaf_rules() -> molten_node_host::error::Result<()> {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let ledger = root.ledger_store()?;
    let store = ledger.root();
    let path = molten_node_host::local_store::RelativeLocator::parse("state.db")?;
    store.write(&path, b"original")?;
    drop(store.open_database_file(&path)?);
    assert_eq!(store.read(&path)?, b"original");
    store.write(&path, b"new")?;
    assert_eq!(store.read(&path)?, b"new");
    let directory = molten_node_host::local_store::RelativeLocator::parse("directory")?;
    store.create_dir_all(&directory)?;
    assert!(store.read(&directory).is_err());
    assert!(store.write(&directory, b"bad").is_err());
    assert!(store.open_database_file(&directory).is_err());
    assert_eq!(store.read(&path)?, b"new");
    Ok(())
}

#[test]
fn local_store_listing_is_sorted_and_excludes_directories() -> molten_node_host::error::Result<()> {
    use molten_node_host::local_store::ObjectKind;
    use molten_node_host::local_store::RelativeLocator;

    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let ledger = root.ledger_store()?;
    let store = ledger.root();
    let directory = RelativeLocator::parse("data")?;
    store.create_dir_all(&RelativeLocator::parse("data/middle")?)?;
    store.write(&RelativeLocator::parse("data/zeta")?, b"z")?;
    store.write(&RelativeLocator::parse("data/alpha")?, b"a")?;

    let entries = store.list_entries(&directory)?;
    let names_and_kinds = entries.iter().map(|entry| (entry.name.as_str(), entry.kind)).collect::<Vec<_>>();
    assert_eq!(names_and_kinds, [
        ("alpha", ObjectKind::File),
        ("middle", ObjectKind::Directory),
        ("zeta", ObjectKind::File)
    ]);
    assert_eq!(store.list_file_names(&directory)?, ["alpha", "zeta"]);
    Ok(())
}

#[cfg(unix)]
#[test]
fn local_store_leaf_links_deny_without_target_changes() -> molten_node_host::error::Result<()> {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let ledger = root.ledger_store()?;
    let store = ledger.root();
    let target = molten_node_host::local_store::RelativeLocator::parse("target")?;
    let link = molten_node_host::local_store::RelativeLocator::parse("link")?;
    store.write(&target, b"original")?;
    temp.symlink("target", "ledger/link")?;
    assert!(store.read(&link).is_err());
    assert!(store.write(&link, b"bad").is_err());
    assert!(store.open_database_file(&link).is_err());
    assert_eq!(store.read(&target)?, b"original");
    assert!(temp.symlink_metadata("ledger/link")?.is_symlink());
    Ok(())
}

#[cfg(unix)]
#[test]
fn namespace_modes_distinguish_missing_regular_and_nonregular_leaves() -> molten_node_host::error::Result<()> {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let namespace = root.identity()?;
    let path = molten_node_host::node_state::RelativePath::parse("fixture")?;
    assert_eq!(namespace.unix_mode(&path)?, None);
    namespace.write_restricted(&path, b"fixture", 0o600)?;
    let mode = namespace
        .unix_mode(&path)?
        .ok_or_else(|| molten_node_host::error::Failure::invalid_harness("fixture mode must exist"))?;
    assert_eq!(mode & 0o777, 0o600);
    let directory = molten_node_host::node_state::RelativePath::parse("directory")?;
    namespace.create_dir_all(&directory)?;
    assert!(namespace.unix_mode(&directory).is_err());
    temp.symlink("fixture", "identity/link")?;
    let link = molten_node_host::node_state::RelativePath::parse("link")?;
    assert!(namespace.unix_mode(&link).is_err());
    assert_eq!(namespace.read(&path, 7)?, b"fixture");
    Ok(())
}
