use molten_node_host::node_state::RelativePath;
use molten_node_host::node_state::Root;

#[test]
fn atomic_leaf_replaces_whole_file_and_rejects_other_shapes() -> molten_node_host::error::Result<()> {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = Root::from_dir(temp.try_clone()?);
    let ns = root.control_service()?;
    let leaf = RelativePath::parse("status.json")?;
    ns.write_atomic_leaf(&leaf, b"old")?;
    ns.write_atomic_leaf(&leaf, b"complete-new-value")?;
    assert_eq!(ns.read(&leaf, 128)?, b"complete-new-value");
    let directory = RelativePath::parse("directory")?;
    ns.create_dir_all(&directory)?;
    assert!(ns.write_atomic_leaf(&directory, b"bad").is_err());
    let nested = RelativePath::parse("absent/leaf")?;
    assert!(ns.write_atomic_leaf(&nested, b"bad").is_err());
    assert!(!ns.try_exists(&RelativePath::parse("absent")?)?);
    Ok(())
}

#[test]
fn concurrent_readers_observe_only_complete_snapshots() -> molten_node_host::error::Result<()> {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = Root::from_dir(temp.try_clone()?);
    let ns = root.control_service()?;
    let leaf = RelativePath::parse("status.json")?;
    let a = vec![b'a'; 4096];
    let b = vec![b'b'; 8192];
    ns.write_atomic_leaf(&leaf, &a)?;
    std::thread::scope(|scope| -> molten_node_host::error::Result<()> {
        let writer_thread = scope.spawn(|| -> molten_node_host::error::Result<()> {
            let writer = root.control_service()?;
            for index in 0..32 {
                writer.write_atomic_leaf(&leaf, if index % 2 == 0 { &b } else { &a })?;
            }
            Ok(())
        });
        for _ in 0..128 {
            let bytes = ns.read(&leaf, 16384)?;
            assert!(bytes == a || bytes == b);
        }
        writer_thread
            .join()
            .map_err(|_| molten_node_host::error::Failure::invalid_harness("atomic writer thread panicked"))??;
        Ok(())
    })?;
    Ok(())
}
