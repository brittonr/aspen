#[path = "state/authority.rs"]
mod authority;
#[path = "state/enumeration.rs"]
mod enumeration;
#[path = "state/filesystem.rs"]
mod filesystem;
#[path = "state/locator.rs"]
mod locator;
#[path = "state/namespace.rs"]
mod namespace;

pub use authority::NodeStateEntryKind;
pub use authority::NodeStateFile;
pub use authority::NodeStateFileObservation;
pub use authority::NodeStateNamespaceKind;
pub use authority::NodeStateRoot;
pub use locator::NodeStatePath;
pub use namespace::NodeStateEntry;
pub use namespace::NodeStateNamespace;

const MAX_NODE_STATE_ENTRIES: usize = 100_000;
pub const MAX_NODE_STATE_FILE_BYTES: u64 = 16 * 1_024 * 1_024;
pub const MAX_NODE_SECRET_BYTES: u64 = 1_024 * 1_024;
const MAX_REASONABLE_NODE_STATE_BOUND: usize = 1_000_000;

const _: () = assert!(MAX_NODE_STATE_ENTRIES > 0);
const _: () = assert!(MAX_NODE_STATE_ENTRIES <= MAX_REASONABLE_NODE_STATE_BOUND);
const _: () = assert!(MAX_NODE_SECRET_BYTES <= MAX_NODE_STATE_FILE_BYTES);

fn invalid(message: impl Into<String>) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(message.into())
}
