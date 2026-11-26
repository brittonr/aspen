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
    let live_lock = exclusion::lock_leaf(&dir, "transport.key", PUBLICATION_LOCK_KIND).expect("live writer lock");
    let live_stage = artifact_name("transport.key", "stage-4242-0");
    dir.write(&live_stage, b"live writer bytes").expect("live writer stage");
    // The publication takes one lock, which reports at most once per attempt, so sends never block.
    let (reached_tx, reached_rx) = std::sync::mpsc::sync_channel(LEAF_LOCK_ATTEMPTS);
    std::thread::scope(|threads| {
        let second = threads.spawn(|| {
            super::super::publication_seam::set_before_lock(move |_name| {
                reached_tx.send(()).expect("report lock boundary")
            });
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
    let mut kept = Vec::with_capacity(neighbours.len() * 3);
    let mut live_locks = Vec::with_capacity(neighbours.len() * 2);
    for leaf in neighbours {
        let stage = artifact_name(leaf, "stage-7-0");
        dir.write(&stage, b"neighbour stage").expect("neighbour stage");
        kept.push(stage);
        live_locks.push(exclusion::lock_leaf(&dir, leaf, PUBLICATION_LOCK_KIND).expect("neighbour publication lock"));
        live_locks.push(exclusion::lock_leaf(&dir, leaf, UPDATE_LOCK_KIND).expect("neighbour update lock"));
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

#[path = "tests/ownership.rs"]
mod ownership;

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
        publish_regular_file(&dir, std::path::Path::new(&lock_like), b"app", Some(SECRET_MODE), Replacement::Replace)
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
    let refused =
        publish_regular_file(&dir, std::path::Path::new("elsewhere"), b"key", Some(SECRET_MODE), Replacement::Replace);
    assert!(refused.is_err(), "a directory leaf must not be replaced");

    cap_fs_ext::DirExt::symlink(&*dir, "target.key", "linked.key").expect("symlink leaf");
    let through_link =
        publish_regular_file(&dir, std::path::Path::new("linked.key"), b"key", Some(SECRET_MODE), Replacement::Replace);
    assert!(through_link.is_err(), "a symlink leaf must not be followed or replaced");
    assert!(dir.symlink_metadata("target.key").is_err(), "nothing is written through the link");
    assert!(stages(&dir).is_empty());
}
