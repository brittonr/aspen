const MARKER_BYTES: &[u8] = b"marker";
const MARKER_BYTE_COUNT: u64 = MARKER_BYTES.len() as u64;

fn accepts_new_path_type(_: &molten_node_host::node_state::NodeStatePath) {}
fn accepts_old_path_type(_: &molten::node_state::NodeStatePath) {}

#[test]
fn root_facades_preserve_exact_node_host_types_and_behavior() -> Result<(), Box<dyn std::error::Error>> {
    // r[verify molten.node_host.crate_boundary]
    // r[verify molten.node_host.facade_compatibility]
    // r[verify molten.node_host.bridge_authority]
    let path = molten_node_host::node_state::NodeStatePath::parse("receipts/marker.bin")?;
    accepts_new_path_type(&path);
    accepts_old_path_type(&path);

    let directory = cap_tempfile::tempdir(cap_tempfile::ambient_authority())?;
    let root = molten::node_state::NodeStateRoot::from_dir(directory.try_clone()?);
    root.create_layout()?;
    root.write(&path, MARKER_BYTES)?;
    assert_eq!(root.read(&path, MARKER_BYTE_COUNT)?, MARKER_BYTES);
    Ok(())
}

#[test]
fn old_and_new_paths_deny_the_same_invalid_locator_class() -> Result<(), Box<dyn std::error::Error>> {
    // r[verify molten.node_host.facade_compatibility]
    let new_error = match molten_node_host::node_state::NodeStatePath::parse("../escape") {
        Ok(_) => return Err(std::io::Error::other("new path admitted parent traversal").into()),
        Err(error) => error,
    };
    let old_error = match molten::node_state::NodeStatePath::parse("../escape") {
        Ok(_) => return Err(std::io::Error::other("old path admitted parent traversal").into()),
        Err(error) => error,
    };
    assert_eq!(new_error, old_error);
    assert!(new_error.to_string().contains("parent traversal"));
    Ok(())
}
