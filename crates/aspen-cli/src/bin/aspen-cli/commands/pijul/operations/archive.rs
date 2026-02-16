//! Archive operation handler.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_blob::InMemoryBlobStore;
use aspen_forge::identity::RepoId;
use aspen_pijul::AspenChangeStore;
use aspen_pijul::ChangeDirectory;
use aspen_pijul::PristineManager;

use super::super::ArchiveArgs;
use super::super::output::PijulArchiveOutput;
use super::super::repo_cache_dir;
use crate::output::print_output;

/// Export repository state as archive (directory or tarball).
///
/// This command outputs the pristine state to a directory, then optionally
/// packages it as a tarball. It uses the local pristine cache, so you must
/// run `pijul sync` first if you want the latest cluster state.
pub(in super::super) async fn pijul_archive(args: ArchiveArgs, json: bool) -> Result<()> {
    use std::fs::File;

    use aspen_pijul::WorkingDirOutput;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = match args.data_dir {
        Some(dir) => dir,
        None => repo_cache_dir(&repo_id).context("failed to determine cache directory")?,
    };

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), args.repo_id);
    }

    info!(cache_dir = %cache_dir.display(), "using local cache");

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Determine output format
    let output_path = PathBuf::from(&args.output_path);
    let is_tar_gz = args.output_path.ends_with(".tar.gz") || args.output_path.ends_with(".tgz");
    let is_tar = args.output_path.ends_with(".tar") || is_tar_gz;
    let format = if is_tar_gz {
        "tar.gz"
    } else if is_tar {
        "tar"
    } else {
        "directory"
    };

    // For tarballs, we need a temp directory first
    let (work_dir, cleanup_temp) = if is_tar {
        let temp_dir = std::env::temp_dir().join(format!(
            "aspen-archive-{}-{}",
            repo_id.to_hex(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).context("failed to create temp directory")?;
        (temp_dir, true)
    } else {
        std::fs::create_dir_all(&output_path).context("failed to create output directory")?;
        (output_path.clone(), false)
    };

    // Output to working directory (or temp dir for tarball)
    let outputter = WorkingDirOutput::new(pristine, change_dir, work_dir.clone());

    let result = if let Some(ref prefix) = args.prefix {
        outputter.output_prefix(&args.channel, prefix).context("failed to output repository state")?
    } else {
        outputter.output(&args.channel).context("failed to output repository state")?
    };

    let conflicts = result.conflict_count() as u32;

    // If tarball requested, create it
    let size_bytes = if is_tar {
        // Create the tarball
        let tar_file = File::create(&output_path).context("failed to create archive file")?;

        let size = if is_tar_gz {
            let encoder = GzEncoder::new(tar_file, Compression::default());
            let mut tar_builder = tar::Builder::new(encoder);
            tar_builder.append_dir_all(".", &work_dir).context("failed to add files to archive")?;
            let encoder = tar_builder.into_inner().context("failed to finish tar archive")?;
            let tar_file = encoder.finish().context("failed to finish gzip compression")?;
            tar_file.metadata().map(|m: std::fs::Metadata| m.len()).unwrap_or(0)
        } else {
            let mut tar_builder = tar::Builder::new(tar_file);
            tar_builder.append_dir_all(".", &work_dir).context("failed to add files to archive")?;
            let tar_file = tar_builder.into_inner().context("failed to finish tar archive")?;
            tar_file.metadata().map(|m: std::fs::Metadata| m.len()).unwrap_or(0)
        };

        // Clean up temp directory
        if cleanup_temp && let Err(e) = std::fs::remove_dir_all(&work_dir) {
            tracing::warn!(path = %work_dir.display(), error = %e, "failed to clean up temp dir");
        }

        size
    } else {
        // Calculate directory size
        calculate_dir_size(&output_path).unwrap_or(0)
    };

    info!(
        output = %args.output_path,
        format = format,
        size = size_bytes,
        conflicts = conflicts,
        "archive complete"
    );

    let output = PijulArchiveOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        output_path: args.output_path,
        format: format.to_string(),
        size_bytes,
        conflicts,
    };
    print_output(&output, json);

    if conflicts > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Calculate the total size of a directory recursively.
fn calculate_dir_size(path: &PathBuf) -> Result<u64> {
    let mut total = 0u64;

    if path.is_file() {
        return Ok(std::fs::metadata(path)?.len());
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            total += calculate_dir_size(&path)?;
        } else {
            total += std::fs::metadata(&path)?.len();
        }
    }

    Ok(total)
}
