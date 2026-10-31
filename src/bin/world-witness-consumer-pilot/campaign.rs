/// A real published release-channel revision that is not the compiled reviewed contract.
const UNREVIEWED_REVISION: &str = "3577429a05acfb442a177582c151dc5557db9ff6";

pub fn run(root: &std::path::Path, revision: &str, program: Option<&std::path::Path>) -> super::Result<()> {
    let path = utf8(root)?;
    child(&["provision", path, UNREVIEWED_REVISION], 1, "differs from compiled reviewed contract")?;
    if root.exists() {
        return Err("rejected provider pin created pilot state".into());
    }
    let mut provision = vec!["provision", path, revision];
    if let Some(program) = program {
        provision.extend(["--provider-program", utf8(program)?]);
    }
    child(&provision, 0, "provisioned")?;
    child(&["advance", path, "1"], 0, "committed generation=1 external_ordinal=1")?;
    child(&["check", path], 0, "current generation=1 external_ordinal=1")?;
    child(&["check", path], 0, "current generation=1 external_ordinal=1")?;
    let snapshot = root.join("consumer-before");
    copy_tree(&root.join("consumer"), &snapshot)?;
    child(&["advance", path, "2"], 0, "committed generation=2 external_ordinal=2")?;
    child(&["check", path], 0, "current generation=2 external_ordinal=2")?;
    supersede(root)?;
    unavailable(root)?;
    // This directory was created exclusively by provision above. Authority,
    // replay/revocations and external index persist independently outside it.
    std::fs::remove_dir_all(root.join("consumer"))?;
    copy_tree(&snapshot, &root.join("consumer"))?;
    child(
        &["check", path],
        2,
        "rollback-detected local_generation=1 local_external_ordinal=1 external_ordinal=4",
    )?;
    println!(
        "pilot-confirmed: real signed authority, reserve/local-CAS/commit, repeated read, unresolved-orphan denial, \
         authenticated resolved-orphan supersession, unavailable-provider denial, complete consumer-root rollback denial"
    );
    println!(
        "nonclaims: private same-host nonproduction trust; no global consensus, provider honesty, or release eligibility"
    );
    Ok(())
}

/// An abandoned external reservation blocks advance until explicitly resolved; the
/// successor then skips exactly the authenticated resolved-orphan high-water.
fn supersede(root: &std::path::Path) -> super::Result<()> {
    let path = utf8(root)?;
    child(&["abandon", path], 0, "abandoned-orphan head_generation=3 external_ordinal=3")?;
    child(&["advance", path, "3"], 1, "unresolved external reservation or orphan blocks advance")?;
    // An unresolved orphan blocks the strong read too; it is not a rollback verdict.
    child(&["check", path], 1, "external high-water rejects stale, staged or orphan claim")?;
    child(&["resolve-orphan", path], 0, "resolved-orphan external_high_water=3 external_committed=2")?;
    child(&["advance", path, "3"], 0, "committed generation=3 external_ordinal=4")?;
    child(&["check", path], 0, "current generation=3 external_ordinal=4")
}

/// Provider absence blocks the strong read; it never downgrades to local currentness.
fn unavailable(root: &std::path::Path) -> super::Result<()> {
    let online = root.join("index");
    let offline = root.join("index-offline");
    std::fs::rename(&online, &offline)?;
    let outcome = child(&["check", utf8(root)?], 1, "Unavailable");
    std::fs::rename(&offline, &online)?;
    outcome
}

fn utf8(path: &std::path::Path) -> super::Result<&str> {
    path.to_str().ok_or_else(|| "pilot path must be UTF-8".into())
}

fn child(arguments: &[&str], expected: i32, marker: &str) -> super::Result<()> {
    println!("step: {}", arguments.join(" "));
    let output = std::process::Command::new(std::env::current_exe()?).args(arguments).output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    print!("{stdout}");
    eprint!("{stderr}");
    if output.status.code() != Some(expected) {
        return Err(
            format!("consumer child {:?} status {:?}, expected {expected}", arguments, output.status.code()).into()
        );
    }
    // Exit status alone never satisfies a step: the exact observed outcome must match.
    if !stdout.contains(marker) && !stderr.contains(marker) {
        return Err(format!("consumer child {arguments:?} lacked expected outcome {marker:?}").into());
    }
    Ok(())
}

/// Bound on snapshot directories and entries; the consumer fixture root is small and fixed.
const MAX_SNAPSHOT_ENTRIES: usize = 4_096;

fn copy_tree(source: &std::path::Path, target: &std::path::Path) -> super::Result<()> {
    let mut pending = vec![(source.to_path_buf(), target.to_path_buf())];
    let mut copied = 0_usize;
    while let Some((source, target)) = pending.pop() {
        std::fs::create_dir(&target)?;
        std::fs::set_permissions(&target, std::fs::metadata(&source)?.permissions())?;
        for entry in std::fs::read_dir(&source)? {
            let entry = entry?;
            copied = copied
                .checked_add(1)
                .filter(|count| *count <= MAX_SNAPSHOT_ENTRIES)
                .ok_or("pilot snapshot exceeds its entry bound")?;
            let kind = entry.file_type()?;
            let target = target.join(entry.file_name());
            if kind.is_dir() {
                if pending.len() >= MAX_SNAPSHOT_ENTRIES {
                    return Err("pilot snapshot exceeds its directory bound".into());
                }
                pending.push((entry.path(), target));
            } else if kind.is_file() {
                std::fs::copy(entry.path(), &target)?;
            } else {
                return Err("pilot snapshot refuses nonregular fixture entry".into());
            }
        }
    }
    Ok(())
}
