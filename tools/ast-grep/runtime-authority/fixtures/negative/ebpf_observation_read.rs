// AST-only control: a supplied scope and a supplied root path both support bounded reads.
fn read_endpoint(root: &molten::node_state::Scope, leaf: &molten::node_state::RelativePath) {
    let _ = root.read(leaf, 4096);
}

fn read_from_supplied_root(path: &std::path::Path, leaf: &molten::node_state::RelativePath) {
    if let Ok(root) = molten::node_state::Scope::open_existing(molten::node_state::NamespaceKind::Ledger, path) {
        read_endpoint(&root, leaf);
    }
}
