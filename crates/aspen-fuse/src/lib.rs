//! FUSE filesystem for mounting Aspen KV clusters as POSIX filesystems.
//!
//! This crate provides a FUSE filesystem that maps filesystem paths to
//! Aspen KV keys, allowing any Aspen cluster to be mounted as a directory.
//!
//! # Features
//!
//! - **File Operations**: Read, write, create, delete files
//! - **Directories**: Virtual directories derived from key prefixes
//! - **Symlinks**: Symbolic link support via special key suffixes
//! - **Extended Attributes**: xattr support for file metadata
//!
//! # Key Mapping
//!
//! Filesystem paths map directly to KV keys:
//! - `/myapp/config/db` → KV key `myapp/config/db`
//! - Directories are virtual (derived from key prefixes)
//!
//! # Usage
//!
//! ```bash
//! # Mount Aspen cluster at /mnt/aspen
//! aspen-fuse --mount-point /mnt/aspen --ticket <cluster-ticket>
//!
//! # Run in foreground (for debugging)
//! aspen-fuse --mount-point /mnt/aspen --ticket <cluster-ticket> -f
//! ```
//!
//! # Tiger Style
//!
//! - Explicit resource bounds (see [`constants`])
//! - Fail-fast on configuration errors
//! - Bounded retries with backoff

pub mod client;
pub mod constants;
pub mod fs;
pub mod inode;

#[cfg(feature = "virtiofs")]
pub mod virtiofs;

pub use client::FuseSyncClient;
pub use client::SharedClient;
pub use fs::AspenFs;
pub use fs::SharedInMemoryStore;
pub use inode::EntryType;
pub use inode::InodeManager;
#[cfg(feature = "virtiofs")]
pub use virtiofs::AspenVirtioFsHandler;
#[cfg(feature = "virtiofs")]
pub use virtiofs::VirtioFsDaemonHandle;
#[cfg(feature = "virtiofs")]
pub use virtiofs::run_virtiofs_daemon;
#[cfg(feature = "virtiofs")]
pub use virtiofs::spawn_virtiofs_daemon;
