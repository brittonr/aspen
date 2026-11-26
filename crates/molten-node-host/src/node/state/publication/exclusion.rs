//! Exclusive per-leaf publication and update locks.

use cap_fs_ext::OpenOptionsFollowExt;

/// An exclusive lock on one leaf. Dropping it unlinks the lock file while still holding the lock,
/// but only when this holder created that file; a file that already existed under the lock name
/// (left by a crashed holder or created outside the API) is reused and never deleted. A process
/// that dies releases the lock without unlinking.
pub struct LeafLock {
    parent: cap_std::fs::Dir,
    name: String,
    is_created_by_holder: bool,
    _file: std::fs::File,
}

impl std::fmt::Debug for LeafLock {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("NodeStateLeafLock").field("name", &self.name).finish_non_exhaustive()
    }
}

impl Drop for LeafLock {
    fn drop(&mut self) {
        if self.is_created_by_holder {
            // Best effort: a lock file left behind is reused by the next holder.
            let _unlinked = self.parent.remove_file(&self.name);
        }
    }
}

/// Opens and exclusively locks `kind`'s lock for `leaf_name` without following a symlink. A lock
/// taken on a file that its creating holder already unlinked is retried, so at most one holder of a
/// leaf's lock kind holds a linked lock at a time. The lock file's bytes are never read or written.
// r[impl molten.runtime_spine.admission_and_capability_hardening]
pub(crate) fn lock_leaf(parent: &cap_std::fs::Dir, leaf_name: &str, kind: &str) -> crate::error::Result<LeafLock> {
    let name = super::artifact_name(leaf_name, kind);
    for _ in 0..super::LEAF_LOCK_ATTEMPTS {
        let Some((file, is_created_by_holder)) = open_lock_file(parent, &name)? else {
            continue;
        };
        let locked = file.metadata().map_err(crate::error::Failure::from)?;
        if !locked.is_file() {
            return Err(super::super::invalid("node state leaf lock is not a regular file"));
        }
        if !is_created_by_holder {
            require_safe_existing_lock(parent, &name, &locked)?;
        }
        #[cfg(any(test, feature = "test-seams"))]
        super::super::publication_seam::before_lock(&name);
        file.lock().map_err(crate::error::Failure::from)?;
        if is_still_linked(parent, &name, &locked)? {
            return Ok(LeafLock {
                parent: parent.try_clone().map_err(crate::error::Failure::from)?,
                name,
                is_created_by_holder,
                _file: file,
            });
        }
    }
    Err(super::super::invalid(format!("node state leaf lock {name} stayed contended")))
}

/// Before blocking on a lock file this holder did not create, require that the opened handle grants
/// no group or other access and is owned by an account allowed to create it: the node-state
/// directory's owner or this process's effective owner (the account whose own new locks carry its
/// uid). Otherwise a foreign account could pre-create the lock and stall every publication of the
/// leaf, while a lock left by a crashed writer of either kind stays reusable.
fn require_safe_existing_lock(
    parent: &cap_std::fs::Dir,
    name: &str,
    locked: &std::fs::Metadata,
) -> crate::error::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let directory_owner = parent
            .open(".")
            .map_err(crate::error::Failure::from)?
            .into_std()
            .metadata()
            .map_err(crate::error::Failure::from)?
            .uid();
        let is_owner_only = locked.mode() & 0o077 == 0;
        let is_allowed_owner = locked.uid() == directory_owner
            || locked.uid()
                == effective_owner(parent, super::STAGE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed))?;
        if !(is_owner_only && is_allowed_owner) {
            return Err(crate::error::Failure::classified(
                crate::error::FailureClass::CapabilityUnavailable,
                format!(
                    "node state leaf lock {name} must be mode 0600 or stricter and owned by the node-state \
                     directory owner or this process"
                ),
            ));
        }
    }
    #[cfg(not(unix))]
    let _unchecked = (parent, name, locked);
    Ok(())
}

/// The uid this process creates files as. molten-node-host's crate boundary admits no system-call
/// crate (`r[molten.node_host.crate_boundary]`), so the uid is observed on a private owner-only
/// probe created inside the capability under the reserved prefix and removed at once. A probe name
/// that already exists (left by a crash, or another process's probe) is reclaimed and the next
/// sequence tried, so a stale probe never blocks lock reuse; any probe file is safe to remove
/// because its creator reads the uid from its own open handle.
#[cfg(unix)]
pub(super) fn effective_owner(parent: &cap_std::fs::Dir, first_sequence: u64) -> crate::error::Result<u32> {
    use std::os::unix::fs::MetadataExt;
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
    {
        use cap_std::fs::OpenOptionsExt;
        options.mode(super::LEAF_LOCK_MODE);
    }
    const PROBE_ATTEMPTS: u64 = 64;
    for attempt in 0..PROBE_ATTEMPTS {
        let probe = owner_probe_name(first_sequence.wrapping_add(attempt));
        match parent.open_with(&probe, &options) {
            Ok(file) => {
                let owner = file.into_std().metadata().map_err(crate::error::Failure::from).map(|meta| meta.uid());
                remove_if_present(parent, &probe)?;
                return owner;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => remove_if_present(parent, &probe)?,
            Err(error) => return Err(crate::error::Failure::from(error)),
        }
    }
    Err(super::super::invalid("node state owner probe stayed contended"))
}

#[cfg(unix)]
pub(super) fn owner_probe_name(sequence: u64) -> String {
    super::artifact_name("owner-probe", &format!("probe-{}-{sequence}", std::process::id()))
}

#[cfg(unix)]
fn remove_if_present(parent: &cap_std::fs::Dir, name: &str) -> crate::error::Result<()> {
    match parent.remove_file(name) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(crate::error::Failure::from(error)),
    }
}

/// Creates the lock file exclusively, or opens an existing one without truncating it; `None` when
/// the existing file vanished between the two attempts. The flag reports whether this call created
/// it.
fn open_lock_file(parent: &cap_std::fs::Dir, name: &str) -> crate::error::Result<Option<(std::fs::File, bool)>> {
    let mut create = cap_std::fs::OpenOptions::new();
    create.read(true).write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        create.mode(super::LEAF_LOCK_MODE);
    }
    match parent.open_with(name, &create) {
        Ok(file) => return Ok(Some((file.into_std(), true))),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(crate::error::Failure::from(error)),
    }
    let mut existing = cap_std::fs::OpenOptions::new();
    existing.read(true).write(true).follow(cap_fs_ext::FollowSymlinks::No);
    match parent.open_with(name, &existing) {
        Ok(file) => Ok(Some((file.into_std(), false))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(crate::error::Failure::from(error)),
    }
}

/// Whether the lock path still names the locked file rather than a successor or nothing.
fn is_still_linked(parent: &cap_std::fs::Dir, name: &str, locked: &std::fs::Metadata) -> crate::error::Result<bool> {
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
    let current = match parent.open_with(name, &options) {
        Ok(current) => current.into_std(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(crate::error::Failure::from(error)),
    };
    let current = current.metadata().map_err(crate::error::Failure::from)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(current.dev() == locked.dev() && current.ino() == locked.ino())
    }
    #[cfg(not(unix))]
    {
        Ok(current.is_file() && locked.is_file())
    }
}
