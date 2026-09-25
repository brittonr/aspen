#[test]
fn direct_node_host_path_opens_capability_state_and_denies_invalid_locators() -> Result<(), Box<dyn std::error::Error>>
{
    // r[verify molten.node_host.crate_boundary]
    // r[verify molten.node_host.facade_compatibility]
    const MARKER_BYTES: &[u8] = b"marker";
    const MARKER_BYTE_COUNT: u64 = MARKER_BYTES.len() as u64;

    let directory = cap_tempfile::tempdir(cap_tempfile::ambient_authority())?;
    let root = molten_node_host::node_state::Root::from_dir(directory.try_clone()?);
    // r[verify molten.node_host.bridge_authority]
    root.create_layout()?;
    let marker = molten_node_host::node_state::RelativePath::parse("receipts/marker.bin")?;
    root.write(&marker, MARKER_BYTES)?;
    assert_eq!(root.read(&marker, MARKER_BYTE_COUNT)?, MARKER_BYTES);
    assert!(molten_node_host::node_state::RelativePath::parse("../escape").is_err());
    Ok(())
}
