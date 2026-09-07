#[test]
fn reads_preserve_zero_exact_and_hard_bounds() {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let namespace = root.control_service().unwrap();
    let leaf = molten_node_host::node_state::NodeStatePath::parse("value").unwrap();
    namespace.write(&leaf, b"").unwrap();
    assert_eq!(namespace.read(&leaf, 0).unwrap(), b"");
    namespace.write(&leaf, b"data").unwrap();
    assert_eq!(namespace.read(&leaf, 4).unwrap(), b"data");
    assert!(namespace.read(&leaf, 3).is_err());
    assert!(namespace.read(&leaf, 0).is_err());
    let excessive_bound = molten_node_host::node_state::MAX_NODE_STATE_FILE_BYTES.checked_add(1).unwrap();
    assert!(namespace.read(&leaf, excessive_bound).is_err());
    assert_eq!(namespace.read(&leaf, 4).unwrap(), b"data");
}

#[test]
fn growth_after_observation_cannot_bypass_read_bound() {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let namespace = root.control_service().unwrap();
    let leaf = molten_node_host::node_state::NodeStatePath::parse("value").unwrap();
    namespace.write(&leaf, b"data").unwrap();
    let molten_node_host::node_state::NodeStateFileObservation::Regular(observed) =
        namespace.observe_file(&leaf).unwrap()
    else {
        panic!("fixture must produce an observed regular file");
    };
    assert_eq!(observed.size(), 4);
    namespace.write(&leaf, b"larger").unwrap();
    assert!(observed.read_bounded(4).is_err());
    assert_eq!(namespace.read(&leaf, 6).unwrap(), b"larger");
}
