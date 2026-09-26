#[test]
fn reads_preserve_zero_exact_and_hard_bounds() -> molten_node_host::error::Result<()> {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let namespace = root.control_service()?;
    let leaf = molten_node_host::node_state::RelativePath::parse("value")?;
    namespace.write(&leaf, b"")?;
    assert_eq!(namespace.read(&leaf, 0)?, b"");
    namespace.write(&leaf, b"data")?;
    assert_eq!(namespace.read(&leaf, 4)?, b"data");
    assert!(namespace.read(&leaf, 3).is_err());
    assert!(namespace.read(&leaf, 0).is_err());
    let excessive_bound = molten_node_host::node_state::MAX_NODE_STATE_FILE_BYTES
        .checked_add(1)
        .ok_or_else(|| molten_node_host::error::Failure::invalid_harness("node state bound cannot be increased"))?;
    assert!(namespace.read(&leaf, excessive_bound).is_err());
    assert_eq!(namespace.read(&leaf, 4)?, b"data");
    Ok(())
}

#[test]
fn growth_after_observation_cannot_bypass_read_bound() -> molten_node_host::error::Result<()> {
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority())?;
    let root = molten_node_host::node_state::Root::from_dir(temp.try_clone()?);
    let namespace = root.control_service()?;
    let leaf = molten_node_host::node_state::RelativePath::parse("value")?;
    namespace.write(&leaf, b"data")?;
    let molten_node_host::node_state::FileObservation::Regular(observed) = namespace.observe_file(&leaf)? else {
        return Err(molten_node_host::error::Failure::invalid_harness("fixture must produce an observed regular file"));
    };
    assert_eq!(observed.size(), 4);
    namespace.write(&leaf, b"larger")?;
    assert!(observed.read_bounded(4).is_err());
    assert_eq!(namespace.read(&leaf, 6)?, b"larger");
    Ok(())
}
