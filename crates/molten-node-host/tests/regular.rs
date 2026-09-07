#[test]
fn regular_writes_and_database_opens_preserve_file_rules() {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let namespace = root.control_service().unwrap();
    let leaf = molten_node_host::node_state::NodeStatePath::parse("nested/value").unwrap();
    namespace.write(&leaf, b"original").unwrap();
    drop(namespace.open_database_file(&leaf).unwrap());
    assert_eq!(namespace.read(&leaf, 8).unwrap(), b"original");
    namespace.write(&leaf, b"new").unwrap();
    assert_eq!(namespace.read(&leaf, 3).unwrap(), b"new");
    let directory = molten_node_host::node_state::NodeStatePath::parse("nested").unwrap();
    assert!(namespace.write(&directory, b"bad").is_err());
    assert!(namespace.open_database_file(&directory).is_err());
    assert!(namespace.read(&directory, 8).is_err());
}

#[test]
fn missing_parents_remain_missing_during_observation() {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let namespace = root.control_service().unwrap();
    let leaf = molten_node_host::node_state::NodeStatePath::parse("absent/leaf").unwrap();
    assert!(matches!(
        namespace.observe_file(&leaf).unwrap(),
        molten_node_host::node_state::NodeStateFileObservation::Missing
    ));
    assert!(!namespace.try_exists(&leaf).unwrap());
    assert!(!temp.exists("control/service/absent"));
}

#[cfg(unix)]
#[test]
fn symlink_leaves_and_parents_deny_without_changing_target() {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let namespace = root.control_service().unwrap();
    let target = molten_node_host::node_state::NodeStatePath::parse("target/value").unwrap();
    namespace.write(&target, b"original").unwrap();
    temp.symlink("target/value", "control/service/link").unwrap();
    temp.symlink("target", "control/service/parent").unwrap();
    for locator in ["link", "parent/value"] {
        let path = molten_node_host::node_state::NodeStatePath::parse(locator).unwrap();
        assert!(namespace.write(&path, b"bad").is_err());
        assert!(namespace.open_database_file(&path).is_err());
        assert!(namespace.read(&path, 8).is_err());
    }
    assert_eq!(namespace.read(&target, 8).unwrap(), b"original");
}
