//! Remote operation handlers for Pijul commands.
//!
//! Repository, channel, change, sync, pull, push, archive, show, blame,
//! and diff operations that communicate with the cluster.

mod archive;
mod blame;
mod change;
mod channel;
mod diff;
mod pull;
mod push;
mod repo;
mod show;
mod sync;

pub(super) use archive::pijul_archive;
pub(super) use blame::pijul_blame;
pub(super) use change::pijul_apply;
pub(super) use change::pijul_checkout;
pub(super) use change::pijul_log;
pub(super) use change::pijul_record;
pub(super) use change::pijul_unrecord;
pub(super) use channel::channel_create;
pub(super) use channel::channel_delete;
pub(super) use channel::channel_fork;
pub(super) use channel::channel_info;
pub(super) use channel::channel_list;
pub(super) use diff::pijul_diff;
pub(super) use pull::pijul_pull;
pub(super) use push::pijul_push;
pub(super) use repo::repo_info;
pub(super) use repo::repo_init;
pub(super) use repo::repo_list;
pub(super) use show::pijul_show;
pub(super) use sync::pijul_sync;
