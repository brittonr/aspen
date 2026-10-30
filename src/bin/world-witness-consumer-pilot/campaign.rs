pub fn run(root: &std::path::Path, revision: &str) -> super::Result<()> {
    child(&["provision", utf8(root)?, revision], 0)?;
    child(&["advance", utf8(root)?, "1"], 0)?;
    child(&["check", utf8(root)?], 0)?;
    child(&["check", utf8(root)?], 0)?;
    let snapshot = root.join("consumer-before");
    copy_tree(&root.join("consumer"), &snapshot)?;
    child(&["advance", utf8(root)?, "2"], 0)?;
    child(&["check", utf8(root)?], 0)?;
    // This directory was created exclusively by provision above. Authority,
    // replay/revocations and external index persist independently outside it.
    std::fs::remove_dir_all(root.join("consumer"))?;
    copy_tree(&snapshot, &root.join("consumer"))?;
    child(&["check", utf8(root)?], 2)?;
    println!(
        "pilot-confirmed: real signed authority, reserve/local-CAS/commit, repeated read, complete consumer-root rollback denial"
    );
    println!(
        "nonclaims: private same-host nonproduction trust; no global consensus, provider honesty, or release eligibility"
    );
    Ok(())
}

fn utf8(path: &std::path::Path) -> super::Result<&str> {
    path.to_str().ok_or_else(|| "pilot path must be UTF-8".into())
}

fn child(arguments: &[&str], expected: i32) -> super::Result<()> {
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
    if expected == 2 && !stdout.contains("rollback-detected local_generation=1 external_generation=2") {
        return Err("child failure was not authenticated whole-store rollback detection".into());
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
