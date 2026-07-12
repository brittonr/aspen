fn bootstrap_node_state(state_root: &std::path::Path) -> crate::error::Result<()> {
    let root = crate::node_state::NodeStateRoot::open(state_root)?;
    run_node_with_root(&root)
}

fn run_node_with_root(root: &crate::node_state::NodeStateRoot) -> crate::error::Result<()> {
    let locator = crate::node_state::NodeStatePath::parse("startup-receipt.preserves")?;
    root.write(&locator, b"receipt")
}
