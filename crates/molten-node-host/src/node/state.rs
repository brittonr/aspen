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
#[path = "state/process.rs"]
mod process;
#[path = "state/publication.rs"]
pub(crate) mod publication;

pub use authority::DirectoryRoot;
pub use authority::EntryKind;
pub use authority::FileHandle;
pub use authority::FileObservation;
pub use authority::NamespaceKind;
pub use locator::RelativePath;
pub use namespace::Entry;
pub use namespace::Scope;
pub use process::Executable;
pub use process::MAX_EXECUTABLE_BYTES;
pub use process::Workspace;
pub use publication::LeafLock;

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

/// Test-only hook at the lock boundary, so a regression can force a contender to the exact point
/// where it is about to block. Compiled only for this crate's tests or the dev-only `test-seams`
/// feature; the hook stays installed on its thread until replaced or cleared and sees each lock
/// name.
#[cfg(any(test, feature = "test-seams"))]
pub mod publication_seam {
    type Hook = Box<dyn FnMut(&str)>;

    thread_local! {
        static BEFORE_LOCK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) };
    }

    /// Installs `hook` for leaf-lock acquisitions on the current thread.
    pub fn set_before_lock(hook: impl FnMut(&str) + 'static) {
        BEFORE_LOCK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
    }

    /// Removes the current thread's hook.
    pub fn clear_before_lock() {
        BEFORE_LOCK.with(|slot| *slot.borrow_mut() = None);
    }

    pub(super) fn before_lock(name: &str) {
        let hook = BEFORE_LOCK.with(|slot| slot.borrow_mut().take());
        if let Some(mut hook) = hook {
            hook(name);
            BEFORE_LOCK.with(|slot| {
                let mut slot = slot.borrow_mut();
                if slot.is_none() {
                    *slot = Some(hook);
                }
            });
        }
    }
}
