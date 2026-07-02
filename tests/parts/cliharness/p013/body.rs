
fn publish_envelope(
    root: &std::path::Path,
    envelope: &std::path::Path,
    receipt: &std::path::Path,
    label: &str,
) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "control-ingress-publish", "--state-root"])
            .arg(root)
            .arg(envelope)
            .args(["--receipt-out"])
            .arg(receipt)
            .output()?,
        label,
    );
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-ingress-receipt");
    Ok(())
}

fn deliver_envelope(
    root: &std::path::Path,
    envelope_ref: &str,
    receipt: &std::path::Path,
    label: &str,
) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "control-ingress-deliver", "--state-root"])
            .arg(root)
            .arg(envelope_ref)
            .args(["--receipt-out"])
            .arg(receipt)
            .output()?,
        label,
    );
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-ingress-receipt");
    Ok(())
}

fn run_once(root: &std::path::Path, receipt: &std::path::Path, label: &str) -> CliResult<std::process::Output> {
    let output = molten_cmd()
        .args(["test", "node", "run-loop", "--state-root"])
        .arg(root)
        .args(["--max-requests", "1", "--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, label);
    Ok(output)
}

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cleanup_stale_molten_temp_dirs() {
    static CLEAN_STALE_TEMP_DIRS: std::sync::Once = std::sync::Once::new();
    CLEAN_STALE_TEMP_DIRS.call_once(|| {
        let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
            return;
        };
        for entry_result in entries {
            let Ok(entry) = entry_result else {
                continue;
            };
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                let file_name = entry.file_name();
                let Some(name) = file_name.to_str() else {
                    continue;
                };
                if is_stale_molten_temp_dir(name) {
                    let remove_result = std::fs::remove_dir_all(entry.path());
                    if remove_result.is_err() {
                        continue;
                    }
                }
            }
        }
    });
}

fn is_stale_molten_temp_dir(name: &str) -> bool {
    name.starts_with("molten-") && live_process_token_count(name) == 0
}

fn live_process_token_count(name: &str) -> usize {
    let current_pid = u64::from(std::process::id());
    name.split('-')
        .filter_map(|token| token.parse::<u64>().ok())
        .filter(|pid| *pid == current_pid || std::path::Path::new("/proc").join(pid.to_string()).exists())
        .count()
}

fn write_release_export_test_archive(
    output_dir: &std::path::Path,
    archive_path: &std::path::Path,
    manifest_path: Option<&std::path::Path>,
    member_refs: &[(String, String)],
) -> CliResult<()> {
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let archive_file = std::fs::File::create(archive_path)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, 0)?;
    let mut builder = tar::Builder::new(encoder);
    if let Some(manifest_path) = manifest_path {
        append_release_export_test_bytes(
            &mut builder,
            "release-export-manifest.preserves",
            &std::fs::read(manifest_path)?,
        )?;
    }
    for (name, _) in member_refs {
        append_release_export_test_bytes(&mut builder, name, &std::fs::read(output_dir.join(name))?)?;
    }
    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

struct ExtraArchiveMember<'a> {
    name: &'a str,
    bytes: &'a [u8],
}

fn write_release_export_test_archive_with_extra(
    output_dir: &std::path::Path,
    archive_path: &std::path::Path,
    manifest_path: &std::path::Path,
    member_refs: &[(String, String)],
    extra: ExtraArchiveMember<'_>,
) -> CliResult<()> {
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let archive_file = std::fs::File::create(archive_path)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, 0)?;
    let mut builder = tar::Builder::new(encoder);
    append_release_export_test_bytes(
        &mut builder,
        "release-export-manifest.preserves",
        &std::fs::read(manifest_path)?,
    )?;
    for (name, _) in member_refs {
        append_release_export_test_bytes(&mut builder, name, &std::fs::read(output_dir.join(name))?)?;
    }
    append_release_export_test_bytes(&mut builder, extra.name, extra.bytes)?;
    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn write_release_export_test_archive_with_tamper(
    output_dir: &std::path::Path,
    archive_path: &std::path::Path,
    manifest_path: &std::path::Path,
    member_refs: &[(String, String)],
) -> CliResult<()> {
    let first = member_refs.first().ok_or_else(|| test_error("release export test needs a member"))?;
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let archive_file = std::fs::File::create(archive_path)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, 0)?;
    let mut builder = tar::Builder::new(encoder);
    append_release_export_test_bytes(
        &mut builder,
        "release-export-manifest.preserves",
        &std::fs::read(manifest_path)?,
    )?;
    for (name, _) in member_refs {
        if name == &first.0 {
            append_release_export_test_bytes(&mut builder, name, b"tampered release evidence")?;
        } else {
            append_release_export_test_bytes(&mut builder, name, &std::fs::read(output_dir.join(name))?)?;
        }
    }
    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn write_release_export_test_archive_with_duplicate(
    output_dir: &std::path::Path,
    archive_path: &std::path::Path,
    manifest_path: &std::path::Path,
    member_refs: &[(String, String)],
) -> CliResult<()> {
    let first = member_refs.first().ok_or_else(|| test_error("release export test needs a member"))?;
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let archive_file = std::fs::File::create(archive_path)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, 0)?;
    let mut builder = tar::Builder::new(encoder);
    append_release_export_test_bytes(
        &mut builder,
        "release-export-manifest.preserves",
        &std::fs::read(manifest_path)?,
    )?;
    for (name, _) in member_refs {
        append_release_export_test_bytes(&mut builder, name, &std::fs::read(output_dir.join(name))?)?;
    }
    append_release_export_test_bytes(&mut builder, &first.0, &std::fs::read(output_dir.join(&first.0))?)?;
    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn append_release_export_test_bytes<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    name: &str,
    bytes: &[u8],
) -> CliResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o444);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();
    builder.append_data(&mut header, name, std::io::Cursor::new(bytes))?;
    Ok(())
}

fn temp_dir(label: &str) -> CliResult<std::path::PathBuf> {
    cleanup_stale_molten_temp_dirs();
    let nonce = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{nonce}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn read_preserves(path: &std::path::Path) -> CliResult<preserves::IOValue> {
    Ok(molten::preserves_rail::parse_text(&std::fs::read_to_string(path)?)?)
}

fn assert_success(output: &std::process::Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout(output),
        stderr(output)
    );
}

fn assert_failure(output: &std::process::Output, label: &str) {
    assert!(
        !output.status.success(),
        "{label} unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn test_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::other(message.into()))
}
