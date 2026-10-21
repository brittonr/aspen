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
#[path = "state/publication.rs"]
mod publication;

pub use authority::DirectoryRoot;
pub use authority::EntryKind;
pub use authority::FileHandle;
pub use authority::FileObservation;
pub use authority::NamespaceKind;
pub use locator::RelativePath;
pub use namespace::Entry;
pub use namespace::Scope;

const MAX_NODE_STATE_ENTRIES: usize = 100_000;
pub const MAX_NODE_STATE_FILE_BYTES: u64 = 16 * 1_024 * 1_024;
pub const MAX_NODE_SECRET_BYTES: u64 = 1_024 * 1_024;
const MAX_REASONABLE_NODE_STATE_BOUND: usize = 1_000_000;

const _: () = assert!(MAX_NODE_STATE_ENTRIES > 0);
const _: () = assert!(MAX_NODE_STATE_ENTRIES <= MAX_REASONABLE_NODE_STATE_BOUND);
const _: () = assert!(MAX_NODE_SECRET_BYTES <= MAX_NODE_STATE_FILE_BYTES);

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message.into())
}
