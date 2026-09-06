use molten_node_host::node_state::{NodeStatePath, NodeStateRoot};

#[test]
fn atomic_leaf_replaces_whole_file_and_rejects_other_shapes() {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let ns = root.control_service().unwrap();
    let leaf = NodeStatePath::parse("status.json").unwrap();
    ns.write_atomic_leaf(&leaf, b"old").unwrap();
    ns.write_atomic_leaf(&leaf, b"complete-new-value").unwrap();
    assert_eq!(ns.read(&leaf, 128).unwrap(), b"complete-new-value");
    let directory = NodeStatePath::parse("directory").unwrap();
    ns.create_dir_all(&directory).unwrap();
    assert!(ns.write_atomic_leaf(&directory, b"bad").is_err());
    let nested = NodeStatePath::parse("absent/leaf").unwrap();
    assert!(ns.write_atomic_leaf(&nested, b"bad").is_err());
    assert!(!ns.try_exists(&NodeStatePath::parse("absent").unwrap()).unwrap());
}

#[test]
fn concurrent_readers_observe_only_complete_snapshots() {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let ns = root.control_service().unwrap();
    let leaf = NodeStatePath::parse("status.json").unwrap();
    let a = vec![b'a'; 4096];
    let b = vec![b'b'; 8192];
    ns.write_atomic_leaf(&leaf, &a).unwrap();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let writer = root.control_service().unwrap();
            for index in 0..32 {
                writer.write_atomic_leaf(&leaf, if index % 2 == 0 { &b } else { &a }).unwrap();
            }
        });
        for _ in 0..128 {
            let bytes = ns.read(&leaf, 16384).unwrap();
            assert!(bytes == a || bytes == b);
        }
    });
}
