#[test]
fn regular_writes_and_database_opens_preserve_file_rules() -> molten_node_host::error::Result<()> {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let namespace = root.control_service()?;
    let leaf = molten_node_host::node_state::RelativePath::parse("nested/value")?;
    namespace.write(&leaf, b"original")?;
    drop(namespace.open_database_file(&leaf)?);
    assert_eq!(namespace.read(&leaf, 8)?, b"original");
    namespace.write(&leaf, b"new")?;
    assert_eq!(namespace.read(&leaf, 3)?, b"new");
    let directory = molten_node_host::node_state::RelativePath::parse("nested")?;
    assert!(namespace.write(&directory, b"bad").is_err());
    assert!(namespace.open_database_file(&directory).is_err());
    assert!(namespace.read(&directory, 8).is_err());
    Ok(())
}

#[test]
fn missing_parents_remain_missing_during_observation() -> molten_node_host::error::Result<()> {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let namespace = root.control_service()?;
    let leaf = molten_node_host::node_state::RelativePath::parse("absent/leaf")?;
    assert!(matches!(namespace.observe_file(&leaf)?, molten_node_host::node_state::FileObservation::Missing));
    assert!(!namespace.try_exists(&leaf)?);
    assert!(!temp.exists("control/service/absent"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_leaves_and_parents_deny_without_changing_target() -> molten_node_host::error::Result<()> {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let namespace = root.control_service()?;
    let target = molten_node_host::node_state::RelativePath::parse("target/value")?;
    namespace.write(&target, b"original")?;
    temp.symlink("target/value", "control/service/link")?;
    temp.symlink("target", "control/service/parent")?;
    for locator in ["link", "parent/value"] {
        let path = molten_node_host::node_state::RelativePath::parse(locator)?;
        assert!(namespace.write(&path, b"bad").is_err());
        assert!(namespace.open_database_file(&path).is_err());
        assert!(namespace.read(&path, 8).is_err());
    }
    assert_eq!(namespace.read(&target, 8)?, b"original");
    Ok(())
}
