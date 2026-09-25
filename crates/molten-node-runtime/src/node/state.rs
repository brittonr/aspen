// r[impl molten.node_host.crate_boundary]
// r[impl molten.node_host.facade_compatibility]
// r[impl molten.node_host.bridge_authority]
pub use molten_node_host::node_state::*;

#[cfg(test)]
#[path = "state/tests.rs"]
mod tests;
