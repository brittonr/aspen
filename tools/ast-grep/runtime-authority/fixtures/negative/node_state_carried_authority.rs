fn persist_received_envelope(
    root: &crate::node_state::Root,
    envelope_ref: &str,
    bytes: &[u8],
) -> crate::error::Result<()> {
    let ingress = root.control_ingress()?;
    let locator = crate::node_state::RelativePath::parse(envelope_ref)?;
    ingress.write(&locator, bytes)
}
