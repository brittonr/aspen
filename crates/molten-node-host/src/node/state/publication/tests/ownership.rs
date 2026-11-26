use super::*;

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
        dir.write(exclusion::owner_probe_name(sequence), b"crash-left probe").expect("stale probe");
    }
    assert_eq!(exclusion::effective_owner(&dir, FIRST).expect("probe past stale names"), writer_uid());
    assert!(stages(&dir).is_empty(), "collided and fresh probes are all removed: {:?}", stages(&dir));
    // A clean directory needs no reclamation and also leaves nothing behind.
    let clean = tempdir();
    assert_eq!(exclusion::effective_owner(&clean, FIRST).expect("probe"), writer_uid());
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
    let refused = publish_regular_file(&dir, std::path::Path::new("a"), b"a", Some(SECRET_MODE), Replacement::Replace)
        .expect_err("a permissive lock must be refused");
    assert_eq!(refused.class(), Some(crate::error::FailureClass::CapabilityUnavailable), "{refused}");
    assert_eq!(dir.read(&lock_name).expect("planted file untouched"), b"planted");
    assert!(dir.read("a").is_err(), "nothing is published under a refused lock");
}
