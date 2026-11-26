use std::io::Write;

use cap_fs_ext::OpenOptionsFollowExt;

/// Prefix of every publication artifact: `.stage.{leaf byte length}.{leaf}.{kind}`. The decimal
/// length fixes where the leaf ends, so no artifact of one leaf can match another leaf's prefix.
const ARTIFACT_PREFIX: &str = ".stage.";
/// Kind prefix of a payload stage: `stage-{process}-{sequence}`.
const STAGE_KIND: &str = "stage-";
/// Kind of the lock every publication of a leaf holds while it settles.
const PUBLICATION_LOCK_KIND: &str = "lock";
/// Kind of the lock a read-modify-write update of a leaf holds across its read and publication.
pub(crate) const UPDATE_LOCK_KIND: &str = "update-lock";
/// Infix of the earlier `.{leaf}.stage-…` stage names; scans still skip interrupted legacy stages.
const LEGACY_STAGE_INFIX: &str = ".stage-";
#[cfg(unix)]
const LEAF_LOCK_MODE: u32 = 0o600;
/// Attempts to lock a lock file that a settling holder unlinks concurrently before giving up.
const LEAF_LOCK_ATTEMPTS: usize = 64;

static STAGE_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Whether a directory entry is a stage or lock of an in-progress or interrupted publication;
/// scans skip it.
pub(crate) fn is_stage(name: &str) -> bool {
    name.starts_with(ARTIFACT_PREFIX) || (name.starts_with('.') && name.contains(LEGACY_STAGE_INFIX))
}

/// Whether `name` uses the reserved publication-artifact prefix; such names are never admitted as
/// application leaves, so internal stages and locks cannot collide with application data.
pub(crate) fn is_reserved_artifact_name(name: &str) -> bool {
    name.starts_with(ARTIFACT_PREFIX)
}

fn artifact_prefix(leaf_name: &str) -> String {
    format!("{ARTIFACT_PREFIX}{}.{leaf_name}.", leaf_name.len())
}

fn artifact_name(leaf_name: &str, kind: &str) -> String {
    format!("{}{kind}", artifact_prefix(leaf_name))
}

// r[impl molten.audit_f03.recover_publication]
// An error after creation retains the partial member for explicit reconciliation.
pub(super) fn write_new_regular_file(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
    bytes: &[u8],
) -> crate::error::Result<()> {
    let (parent, leaf) = super::filesystem::open_parent(dir, path, false)?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
    let mut file = parent.open_with(leaf, &options).map_err(crate::error::Failure::from)?;
    if !file.metadata().map_err(crate::error::Failure::from)?.is_file() {
        return Err(super::invalid("new node state publication is not a regular file"));
    }
    file.write_all(bytes).map_err(crate::error::Failure::from)?;
    file.sync_all().map_err(crate::error::Failure::from)?;
    sync_directory(&parent)
}

/// How a staged publication treats an existing destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Replacement {
    /// The destination must not exist; an existing file is never overwritten.
    NoReplace,
    /// An existing regular-file destination is replaced in one step.
    Replace,
}

/// Publishes `bytes` so a crash leaves the destination absent, unchanged, or holding all of `bytes`
/// with `unix_mode`, never a truncated or partially written file. The payload goes to a stage in
/// the destination directory and is synchronized, then linked (no replacement) or renamed
/// (replacement) onto the leaf, and the directory is synchronized. A failure after the leaf changed
/// is reported as an uncertain outcome.
// r[impl molten.crypto_identity.atomic_key_publication]
pub(crate) fn publish_regular_file(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
    bytes: &[u8],
    unix_mode: Option<u32>,
    replacement: Replacement,
) -> crate::error::Result<()> {
    let (parent, leaf) = super::filesystem::open_parent(dir, path, true)?;
    let leaf_name = leaf
        .to_str()
        .ok_or_else(|| super::invalid(format!("node state publication leaf {} is not UTF-8", path.display())))?;
    // Every publication of this leaf, in any process, holds the leaf's lock until it settles, so a
    // stage present while the lock is held here belongs to an interrupted writer, never a live one.
    if is_reserved_artifact_name(leaf_name) {
        return Err(super::invalid(format!("node state publication leaf {leaf_name} uses a reserved name")));
    }
    let _publication = lock_leaf(&parent, leaf_name, PUBLICATION_LOCK_KIND)?;
    match parent.symlink_metadata(leaf) {
        Ok(_) if replacement == Replacement::NoReplace => {
            return Err(super::invalid(format!("node state publication {} already exists", path.display())));
        }
        Ok(metadata) => {
            if super::enumeration::entry_kind(&metadata.file_type()) != super::authority::EntryKind::RegularFile {
                return Err(super::invalid(format!("node state write leaf {} must be a regular file", path.display())));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(crate::error::Failure::from(error)),
    }
    remove_stale_stages(&parent, leaf_name)?;
    let stage = artifact_name(
        leaf_name,
        &format!(
            "{STAGE_KIND}{}-{}",
            std::process::id(),
            STAGE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ),
    );
    if let Err(error) = write_stage(&parent, &stage, bytes, unix_mode) {
        return Err(discard_stage(&parent, &stage, error));
    }
    let committed = match replacement {
        Replacement::NoReplace => parent.hard_link(&stage, &parent, leaf),
        Replacement::Replace => parent.rename(&stage, &parent, leaf),
    };
    if let Err(error) = committed {
        return Err(discard_stage(&parent, &stage, crate::error::Failure::from(error)));
    }
    let settled = match replacement {
        Replacement::NoReplace => parent.remove_file(&stage).map_err(crate::error::Failure::from),
        Replacement::Replace => Ok(()),
    };
    settled.and_then(|()| sync_directory(&parent)).map_err(|error| {
        crate::error::Failure::classified(
            crate::error::FailureClass::UncertainExternalOutcome,
            format!("node state publication {} is visible but not settled: {error}", path.display()),
        )
    })
}

fn write_stage(
    parent: &cap_std::fs::Dir,
    stage: &str,
    bytes: &[u8],
    unix_mode: Option<u32>,
) -> crate::error::Result<()> {
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
    #[cfg(unix)]
    if let Some(mode) = unix_mode {
        use cap_std::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    let _unused_mode = unix_mode;
    let mut file = parent.open_with(stage, &options).map_err(crate::error::Failure::from)?;
    file.write_all(bytes).map_err(crate::error::Failure::from)?;
    file.sync_all().map_err(crate::error::Failure::from)
}

/// Removes this publication's stage after a failure and returns the original error, which takes
/// precedence.
fn discard_stage(parent: &cap_std::fs::Dir, stage: &str, error: crate::error::Failure) -> crate::error::Failure {
    match parent.remove_file(stage) {
        Ok(()) => error,
        Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => error,
        Err(cleanup) => super::invalid(format!("{error}; stage {stage} was not removed: {cleanup}")),
    }
}

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
    let name = artifact_name(leaf_name, kind);
    for _ in 0..LEAF_LOCK_ATTEMPTS {
        let Some((file, is_created_by_holder)) = open_lock_file(parent, &name)? else {
            continue;
        };
        let locked = file.metadata().map_err(crate::error::Failure::from)?;
        if !locked.is_file() {
            return Err(super::invalid("node state leaf lock is not a regular file"));
        }
        if !is_created_by_holder {
            require_safe_existing_lock(parent, &name, &locked)?;
        }
        #[cfg(any(test, feature = "test-seams"))]
        seam::before_lock(&name);
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
    Err(super::invalid(format!("node state leaf lock {name} stayed contended")))
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
                == effective_owner(parent, STAGE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed))?;
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
fn effective_owner(parent: &cap_std::fs::Dir, first_sequence: u64) -> crate::error::Result<u32> {
    use std::os::unix::fs::MetadataExt;
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
    {
        use cap_std::fs::OpenOptionsExt;
        options.mode(LEAF_LOCK_MODE);
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
    Err(super::invalid("node state owner probe stayed contended"))
}

#[cfg(unix)]
fn owner_probe_name(sequence: u64) -> String {
    artifact_name("owner-probe", &format!("probe-{}-{sequence}", std::process::id()))
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
        create.mode(LEAF_LOCK_MODE);
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

/// Removes stages that an interrupted publication of the same leaf left behind; they are never
/// read. Callers hold the leaf's publication lock, so no live writer owns any of these stages, and
/// the length-delimited prefix never matches another leaf's stages or any lock. Only names of the
/// exact form this module creates (`…stage-{process}-{sequence}`, both decimal) are removed; any
/// other file under the reserved prefix, however it got there, is left untouched.
fn remove_stale_stages(parent: &cap_std::fs::Dir, leaf_name: &str) -> crate::error::Result<()> {
    let prefix = artifact_name(leaf_name, STAGE_KIND);
    for entry in parent.read_dir(".").map_err(crate::error::Failure::from)? {
        let entry = entry.map_err(crate::error::Failure::from)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let is_well_formed_stage = name.strip_prefix(&prefix).is_some_and(is_stage_suffix);
        if is_well_formed_stage && entry.file_type().map_err(crate::error::Failure::from)?.is_file() {
            match parent.remove_file(name) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(crate::error::Failure::from(error)),
            }
        }
    }
    Ok(())
}

/// Whether `suffix` is exactly `{process}-{sequence}` with nonempty decimal parts.
fn is_stage_suffix(suffix: &str) -> bool {
    let is_decimal = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    suffix
        .split_once('-')
        .is_some_and(|(process, sequence)| is_decimal(process) && is_decimal(sequence))
}

/// Test-only hook at the lock boundary, so a regression can force a contender to the exact point
/// where it is about to block. Compiled only for this crate's tests or the dev-only `test-seams`
/// feature; the hook stays installed on its thread until replaced or cleared and sees each lock
/// name.
#[cfg(any(test, feature = "test-seams"))]
pub mod seam {
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

// Directory capabilities may hold O_PATH descriptors, which cannot be synchronized.
fn sync_directory(parent: &cap_std::fs::Dir) -> crate::error::Result<()> {
    parent
        .open(".")
        .map_err(crate::error::Failure::from)?
        .sync_all()
        .map_err(crate::error::Failure::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET_MODE: u32 = 0o600;

    fn tempdir() -> cap_tempfile::TempDir {
        cap_tempfile::tempdir(cap_tempfile::ambient_authority()).expect("temporary directory")
    }

    fn mode(dir: &cap_std::fs::Dir, leaf: &str) -> u32 {
        use cap_std::fs::PermissionsExt;
        dir.metadata(leaf).expect("metadata").permissions().mode() & 0o777
    }

    fn stages(dir: &cap_std::fs::Dir) -> Vec<String> {
        dir.entries()
            .expect("entries")
            .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
            .filter(|name| is_stage(name))
            .collect()
    }

    // r[verify molten.crypto_identity.atomic_key_publication]
    #[test]
    fn no_replace_creates_once_and_never_overwrites() {
        let dir = tempdir();
        let path = std::path::Path::new("keys/transport.key");
        publish_regular_file(&dir, path, b"first", Some(SECRET_MODE), Replacement::NoReplace).expect("first key");
        let keys = dir.open_dir("keys").expect("key directory");
        assert_eq!(keys.read("transport.key").expect("key bytes"), b"first");
        assert_eq!(mode(&keys, "transport.key"), SECRET_MODE);

        let second = publish_regular_file(&dir, path, b"second", Some(SECRET_MODE), Replacement::NoReplace);
        assert!(second.is_err(), "an existing key must not be replaced");
        assert_eq!(keys.read("transport.key").expect("key bytes"), b"first");
        assert!(stages(&keys).is_empty());
    }

    // r[verify molten.crypto_identity.atomic_key_publication]
    #[test]
    fn replacement_swaps_whole_content_and_restricts_a_loose_existing_file() {
        let dir = tempdir();
        dir.write("transport.key", b"old generation").expect("existing key");
        {
            use cap_std::fs::PermissionsExt;
            dir.set_permissions("transport.key", cap_std::fs::Permissions::from_mode(0o644))
                .expect("loose mode");
        }
        let path = std::path::Path::new("transport.key");
        publish_regular_file(&dir, path, b"new", Some(SECRET_MODE), Replacement::Replace).expect("rotation");
        assert_eq!(dir.read("transport.key").expect("key bytes"), b"new");
        assert_eq!(mode(&dir, "transport.key"), SECRET_MODE);
        assert!(stages(&dir).is_empty());
    }

    // r[verify molten.crypto_identity.atomic_key_publication]
    #[test]
    fn interrupted_stages_are_removed_and_never_become_the_key() {
        let dir = tempdir();
        dir.write(artifact_name("transport.key", "stage-1-0"), b"half written").expect("stale stage");
        dir.write(artifact_name("other.key", "stage-1-0"), b"other leaf").expect("other stage");
        let path = std::path::Path::new("transport.key");
        publish_regular_file(&dir, path, b"whole", Some(SECRET_MODE), Replacement::NoReplace).expect("publication");
        assert_eq!(dir.read("transport.key").expect("key bytes"), b"whole");
        assert_eq!(stages(&dir), vec![artifact_name("other.key", "stage-1-0")]);
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[test]
    fn a_concurrent_writers_active_stage_is_never_unlinked() {
        let dir = tempdir();
        // A live writer: it holds the leaf's publication lock while its stage exists.
        let live_lock = lock_leaf(&dir, "transport.key", PUBLICATION_LOCK_KIND).expect("live writer lock");
        let live_stage = artifact_name("transport.key", "stage-4242-0");
        dir.write(&live_stage, b"live writer bytes").expect("live writer stage");
        let (reached_tx, reached_rx) = std::sync::mpsc::channel();
        std::thread::scope(|threads| {
            let second = threads.spawn(|| {
                seam::set_before_lock(move |_name| reached_tx.send(()).expect("report lock boundary"));
                let path = std::path::Path::new("transport.key");
                publish_regular_file(&dir, path, b"second", Some(SECRET_MODE), Replacement::Replace)
            });
            // The second publication is now at the lock boundary; it cannot clean up before the live
            // writer releases, so the live stage survives regardless of scheduling.
            reached_rx.recv().expect("second publication reaches the lock");
            assert_eq!(dir.read(&live_stage).expect("live stage survives"), b"live writer bytes");
            dir.rename(&live_stage, &dir, "transport.key").expect("live writer commit");
            drop(live_lock);
            second.join().expect("second publication thread").expect("second publication");
        });
        assert_eq!(dir.read("transport.key").expect("key bytes"), b"second");
        assert!(stages(&dir).is_empty());
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[test]
    fn cleanup_never_matches_another_leafs_stage_or_lock() {
        let dir = tempdir();
        let neighbours = ["a.stage-lock", "a.", "1.a", "a.stage-1-0", "aa"];
        let mut kept = Vec::new();
        let mut live_locks = Vec::new();
        for leaf in neighbours {
            let stage = artifact_name(leaf, "stage-7-0");
            dir.write(&stage, b"neighbour stage").expect("neighbour stage");
            kept.push(stage);
            live_locks.push(lock_leaf(&dir, leaf, PUBLICATION_LOCK_KIND).expect("neighbour publication lock"));
            live_locks.push(lock_leaf(&dir, leaf, UPDATE_LOCK_KIND).expect("neighbour update lock"));
            kept.push(artifact_name(leaf, PUBLICATION_LOCK_KIND));
            kept.push(artifact_name(leaf, UPDATE_LOCK_KIND));
        }
        dir.write(artifact_name("a", "stage-7-0"), b"stale").expect("own stale stage");
        publish_regular_file(&dir, std::path::Path::new("a"), b"a", Some(SECRET_MODE), Replacement::Replace)
            .expect("publish a");
        let mut remaining = stages(&dir);
        remaining.sort();
        kept.sort();
        assert_eq!(remaining, kept, "only leaf a's own stale stage may be removed");
        drop(live_locks);
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[test]
    fn cleanup_removes_only_well_formed_stages_of_its_own_leaf() {
        let dir = tempdir();
        let foreign = [
            artifact_name("a", "stage-"),
            artifact_name("a", "stage-junk"),
            artifact_name("a", "stage-1-x"),
            artifact_name("a", "stage-1-2-3"),
            artifact_name("a", "stage--2"),
            artifact_name("a", "stage-1-"),
        ];
        for name in &foreign {
            dir.write(name, b"created outside the publication API").expect("foreign reserved-prefix file");
        }
        let stale = artifact_name("a", "stage-12-34");
        dir.write(&stale, b"interrupted stage").expect("well-formed stale stage");
        publish_regular_file(&dir, std::path::Path::new("a"), b"a", Some(SECRET_MODE), Replacement::Replace)
            .expect("publish a");
        assert!(dir.read(&stale).is_err(), "the well-formed stale stage is removed");
        for name in &foreign {
            assert_eq!(
                dir.read(name).expect("foreign file survives"),
                b"created outside the publication API",
                "{name} must never be deleted"
            );
        }
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[test]
    fn a_pre_existing_file_at_a_lock_name_is_reused_but_never_deleted_or_rewritten() {
        use cap_std::fs::PermissionsExt;

        let dir = tempdir();
        let lock_name = artifact_name("a", PUBLICATION_LOCK_KIND);
        dir.write(&lock_name, b"left by a crashed holder").expect("crash-left lock file");
        dir.set_permissions(&lock_name, cap_std::fs::Permissions::from_mode(0o600))
            .expect("owner-only lock");
        publish_regular_file(&dir, std::path::Path::new("a"), b"first", Some(SECRET_MODE), Replacement::Replace)
            .expect("first publication");
        publish_regular_file(&dir, std::path::Path::new("a"), b"second", Some(SECRET_MODE), Replacement::Replace)
            .expect("second publication");
        assert_eq!(dir.read("a").expect("leaf"), b"second");
        assert_eq!(dir.read(&lock_name).expect("existing lock survives"), b"left by a crashed holder");
        // A lock file this publication created itself is still unlinked when it settles.
        let fresh = tempdir();
        publish_regular_file(&fresh, std::path::Path::new("a"), b"a", Some(SECRET_MODE), Replacement::Replace)
            .expect("fresh publication");
        assert!(stages(&fresh).is_empty());
    }

    /// The uid this test process creates files as, read from a file it creates in its own temp dir.
    #[cfg(unix)]
    fn writer_uid() -> u32 {
        use std::os::unix::fs::MetadataExt;
        let dir = tempdir();
        dir.write("uid-witness", b"").expect("uid witness");
        dir.open("uid-witness").expect("witness").into_std().metadata().expect("witness metadata").uid()
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[cfg(unix)]
    #[test]
    fn the_owner_probe_reclaims_crash_left_probes_and_leaves_nothing_behind() {
        let dir = tempdir();
        const FIRST: u64 = 9_000;
        // Probes left by a crash at the next three names this call would use.
        for sequence in FIRST..FIRST + 3 {
            dir.write(owner_probe_name(sequence), b"crash-left probe").expect("stale probe");
        }
        assert_eq!(effective_owner(&dir, FIRST).expect("probe past stale names"), writer_uid());
        assert!(stages(&dir).is_empty(), "collided and fresh probes are all removed: {:?}", stages(&dir));
        // A clean directory needs no reclamation and also leaves nothing behind.
        let clean = tempdir();
        assert_eq!(effective_owner(&clean, FIRST).expect("probe"), writer_uid());
        assert!(stages(&clean).is_empty());
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[cfg(unix)]
    #[test]
    fn a_crash_left_lock_of_a_writer_that_does_not_own_the_directory_is_reusable() {
        use std::os::unix::fs::MetadataExt;

        const UNPRIVILEGED_OWNER: u32 = 65_534;
        let effective = writer_uid();
        let sequence = STAGE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // A directory owned by another account: as root, a private temp dir handed to `nobody`; as an
        // ordinary user, the root-owned sticky `/tmp`. Either way the writer is not the owner.
        let (shared, owned_fixture) = if effective == 0 {
            let path =
                std::env::temp_dir().join(format!("molten-node-host-foreign-owner-{}-{sequence}", std::process::id()));
            std::fs::create_dir(&path).expect("fixture directory");
            std::os::unix::fs::chown(&path, Some(UNPRIVILEGED_OWNER), Some(UNPRIVILEGED_OWNER))
                .expect("hand the fixture directory to another account");
            (path.clone(), Some(path))
        } else {
            (std::path::PathBuf::from("/tmp"), None)
        };
        let directory_owner = std::fs::metadata(&shared).expect("shared directory").uid();
        assert_ne!(directory_owner, effective, "fixture: the writer must not own the directory");
        let dir = cap_std::fs::Dir::open_ambient_dir(&shared, cap_std::ambient_authority()).expect("shared dir");
        let leaf = format!("molten-node-host-lock-reuse-{}-{sequence}", std::process::id());
        let lock_name = artifact_name(&leaf, PUBLICATION_LOCK_KIND);
        // A lock this writer created and then crashed without unlinking: owner-only, owned by the writer.
        let mut options = cap_std::fs::OpenOptions::new();
        options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
        {
            use cap_std::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock_owner = dir
            .open_with(&lock_name, &options)
            .expect("crash-left lock")
            .into_std()
            .metadata()
            .expect("meta")
            .uid();
        let published =
            publish_regular_file(&dir, std::path::Path::new(&leaf), b"reused", Some(SECRET_MODE), Replacement::Replace);
        let leaf_bytes = dir.read(&leaf);
        let _cleanup = (dir.remove_file(&leaf), dir.remove_file(&lock_name));
        if let Some(path) = owned_fixture {
            let _removed = std::fs::remove_dir_all(path);
        }
        assert_eq!(lock_owner, effective);
        published.expect("a crash-left owner-only lock of this writer is reusable");
        assert_eq!(leaf_bytes.expect("published leaf"), b"reused");
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[cfg(unix)]
    #[test]
    fn a_permissive_pre_existing_lock_is_refused_before_blocking() {
        use cap_std::fs::PermissionsExt;

        let dir = tempdir();
        let lock_name = artifact_name("a", PUBLICATION_LOCK_KIND);
        dir.write(&lock_name, b"planted").expect("permissive lock file");
        dir.set_permissions(&lock_name, cap_std::fs::Permissions::from_mode(0o666))
            .expect("permissive mode");
        let refused =
            publish_regular_file(&dir, std::path::Path::new("a"), b"a", Some(SECRET_MODE), Replacement::Replace)
                .expect_err("a permissive lock must be refused");
        assert_eq!(refused.class(), Some(crate::error::FailureClass::CapabilityUnavailable), "{refused}");
        assert_eq!(dir.read(&lock_name).expect("planted file untouched"), b"planted");
        assert!(dir.read("a").is_err(), "nothing is published under a refused lock");
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[test]
    fn local_store_mutations_cannot_name_a_reserved_artifact() {
        for reserved in [
            artifact_name("a", PUBLICATION_LOCK_KIND),
            artifact_name("a", UPDATE_LOCK_KIND),
            artifact_name("a", "stage-1-0"),
            format!("dir/{}", artifact_name("a", PUBLICATION_LOCK_KIND)),
        ] {
            // Every local-store write, publish, create, rename and remove takes a `Locator`, so a
            // reserved name is refused before any filesystem effect.
            let refused = crate::local_store::Locator::parse(&reserved).expect_err("reserved locator");
            assert!(refused.to_string().contains("reserved publication-artifact prefix"), "{refused}");
            assert!(super::super::locator::RelativePath::parse(&reserved).is_err());
        }
        crate::local_store::Locator::parse(".a.stage-lock").expect("ordinary hidden leaf stays admissible");
    }

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[test]
    fn an_application_leaf_named_like_an_internal_artifact_is_never_admitted_or_touched() {
        let lock_like = artifact_name("a", PUBLICATION_LOCK_KIND);
        assert!(super::super::locator::RelativePath::parse(&lock_like).is_err());
        assert!(super::super::locator::RelativePath::parse(&format!("dir/{lock_like}")).is_err());
        let dir = tempdir();
        assert!(
            publish_regular_file(
                &dir,
                std::path::Path::new(&lock_like),
                b"app",
                Some(SECRET_MODE),
                Replacement::Replace
            )
            .is_err()
        );
        // Ordinary hidden and stage-like application names remain admissible.
        super::super::locator::RelativePath::parse(".a.stage-lock").expect("ordinary hidden leaf");
    }

    // r[verify molten.crypto_identity.atomic_key_publication]
    #[cfg(unix)]
    #[test]
    fn a_symlinked_or_non_regular_destination_is_refused() {
        let dir = tempdir();
        dir.create_dir("elsewhere").expect("directory leaf");
        let refused = publish_regular_file(
            &dir,
            std::path::Path::new("elsewhere"),
            b"key",
            Some(SECRET_MODE),
            Replacement::Replace,
        );
        assert!(refused.is_err(), "a directory leaf must not be replaced");

        cap_fs_ext::DirExt::symlink(&*dir, "target.key", "linked.key").expect("symlink leaf");
        let through_link = publish_regular_file(
            &dir,
            std::path::Path::new("linked.key"),
            b"key",
            Some(SECRET_MODE),
            Replacement::Replace,
        );
        assert!(through_link.is_err(), "a symlink leaf must not be followed or replaced");
        assert!(dir.symlink_metadata("target.key").is_err(), "nothing is written through the link");
        assert!(stages(&dir).is_empty());
    }
}
