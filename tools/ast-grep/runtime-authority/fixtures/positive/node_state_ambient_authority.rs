fn mutate_node_state_ambiently(state_root: &std::path::Path) {
    let request = state_root.join("control/inbox/request.preserves");
    let _ = std::fs::read(&request);
    let _ = std::fs::write(state_root.join("startup-receipt.preserves"), b"receipt");
    let _ = crate::node_state::NodeStateRoot::open(state_root);
}
