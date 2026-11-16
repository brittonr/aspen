#![allow(dead_code, reason = "shared helper module serves two integration binaries")]

use std::path::Path;

pub mod command;
pub mod head;
mod material;
pub mod signed;

pub use command::run_promote;
pub use command::run_promote_output;
pub use command::run_success;
pub use head::seed;
pub use material::publish_intent;
pub use material::reference_for_adapter;
pub use molten::world_promotion::LocalWorldPromotionStore;
pub use molten::world_promotion::WorldPromotionBookkeepingPort;
pub use molten_core::world_head::WorldBranchId;
pub use molten_core::world_head::WorldHeadPolicyRef;
pub use molten_node_host::node_state::DirectoryRoot;
pub use molten_node_host::node_state::NamespaceKind;
pub use molten_node_host::node_state::RelativePath;
pub use signed::fixtures;

pub fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

pub fn write(path: &Path, value: serde_json::Value) {
    std::fs::write(path, serde_json::to_vec(&value).expect("json")).expect("write");
}

pub fn read(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).expect("read fixture")).expect("json")
}
