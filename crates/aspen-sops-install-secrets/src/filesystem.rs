//! Filesystem operations: mount, write, symlink, prune.
//!
//! Handles ramfs/tmpfs mounting, secret file writing with permissions,
//! atomic symlink updates, and generation pruning.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use tracing::debug;
use tracing::warn;

use crate::manifest::SecretEntry;
use crate::manifest::TemplateEntry;

// ── Mount ──────────────────────────────────────────────────────────────

/// Mount a ramfs or tmpfs at the secrets mount point.
pub fn mount_secrets_fs(mount_point: &str, keys_gid: u32, use_tmpfs: bool) -> Result<()> {
    let path = Path::new(mount_point);

    // Create mount point if it doesn't exist
    if !path.exists() {
        fs::create_dir_all(path).with_context(|| format!("cannot create mount point '{mount_point}'"))?;
    }

    // Check if already mounted
    if is_mounted(mount_point)? {
        debug!("secrets filesystem already mounted at {mount_point}");
        return Ok(());
    }

    let fs_type = if use_tmpfs { "tmpfs" } else { "ramfs" };
    let source = if use_tmpfs { "tmpfs" } else { "ramfs" };

    nix::mount::mount(
        Some(source),
        mount_point,
        Some(fs_type),
        nix::mount::MsFlags::MS_NOSUID | nix::mount::MsFlags::MS_NODEV,
        None::<&str>,
    )
    .with_context(|| format!("cannot mount {fs_type} at '{mount_point}'"))?;

    // Set ownership to root:<keys_gid> with mode 0751
    nix::unistd::chown(path, Some(nix::unistd::Uid::from_raw(0)), Some(nix::unistd::Gid::from_raw(keys_gid)))
        .with_context(|| format!("cannot chown '{mount_point}'"))?;

    fs::set_permissions(path, fs::Permissions::from_mode(0o751))
        .with_context(|| format!("cannot set permissions on '{mount_point}'"))?;

    debug!("mounted {fs_type} at {mount_point}");
    Ok(())
}

/// Check if a path is a mount point by reading /proc/mounts.
fn is_mounted(path: &str) -> Result<bool> {
    let mounts = fs::read_to_string("/proc/mounts").unwrap_or_default();
    Ok(mounts.lines().any(|line| line.split_whitespace().nth(1).is_some_and(|mp| mp == path)))
}

// ── Generation management ──────────────────────────────────────────────

/// Prepare a new generation directory. Returns the path and generation number.
pub fn prepare_generation(mount_point: &str, symlink_path: &str, keys_gid: u32) -> Result<(PathBuf, u64)> {
    // Read current generation from existing symlink
    let current_gen = match fs::read_link(symlink_path) {
        Ok(target) => {
            if let Some(name) = target.file_name().and_then(|n| n.to_str()) {
                name.parse::<u64>().unwrap_or(0)
            } else {
                0
            }
        }
        Err(_) => {
            // Symlink doesn't exist or isn't a symlink — might be a directory from stage-2-init
            if Path::new(symlink_path).exists() {
                fs::remove_dir_all(symlink_path).ok();
            }
            0
        }
    };

    let new_gen = current_gen + 1;
    let gen_dir = PathBuf::from(mount_point).join(new_gen.to_string());

    // Remove stale generation directory if it exists
    if gen_dir.exists() {
        fs::remove_dir_all(&gen_dir).with_context(|| format!("cannot remove existing '{}'", gen_dir.display()))?;
    }

    fs::create_dir(&gen_dir).with_context(|| format!("cannot create '{}'", gen_dir.display()))?;

    nix::unistd::chown(
        gen_dir.as_path(),
        Some(nix::unistd::Uid::from_raw(0)),
        Some(nix::unistd::Gid::from_raw(keys_gid)),
    )
    .with_context(|| format!("cannot chown '{}'", gen_dir.display()))?;

    fs::set_permissions(&gen_dir, fs::Permissions::from_mode(0o751))
        .with_context(|| format!("cannot set permissions on '{}'", gen_dir.display()))?;

    Ok((gen_dir, new_gen))
}

// ── Secret writing ─────────────────────────────────────────────────────

/// Write a decrypted secret to the generation directory.
pub fn write_secret(
    gen_dir: &Path,
    secret: &SecretEntry,
    value: &[u8],
    keys_gid: u32,
    ignore_passwd: bool,
) -> Result<()> {
    let file_path = gen_dir.join(&secret.name);

    // Create parent directories
    if let Some(parent) = Path::new(&secret.name).parent() {
        if parent != Path::new("") {
            let full_parent = gen_dir.join(parent);
            create_parent_dirs(&full_parent, keys_gid)?;
        }
    }

    fs::write(&file_path, value).with_context(|| format!("cannot write '{}'", file_path.display()))?;

    // Parse mode
    let mode = u32::from_str_radix(&secret.mode, 8)
        .with_context(|| format!("invalid mode '{}' for secret '{}'", secret.mode, secret.name))?;

    fs::set_permissions(&file_path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("cannot set mode on '{}'", file_path.display()))?;

    // Set ownership
    if !ignore_passwd {
        let (uid, gid) = resolve_ownership(secret.owner.as_deref(), secret.uid, secret.group.as_deref(), secret.gid)?;

        nix::unistd::chown(
            file_path.as_path(),
            Some(nix::unistd::Uid::from_raw(uid)),
            Some(nix::unistd::Gid::from_raw(gid)),
        )
        .with_context(|| format!("cannot chown '{}'", file_path.display()))?;
    }

    Ok(())
}

/// Write a rendered template to the generation directory.
pub fn write_template(
    gen_dir: &Path,
    template: &TemplateEntry,
    rendered: &str,
    keys_gid: u32,
    ignore_passwd: bool,
) -> Result<()> {
    let rendered_dir = gen_dir.join("rendered");
    create_parent_dirs(&rendered_dir, keys_gid)?;

    let file_path = rendered_dir.join(&template.name);

    // Create parent directories for nested template names
    if let Some(parent) = Path::new(&template.name).parent() {
        if parent != Path::new("") {
            let full_parent = rendered_dir.join(parent);
            create_parent_dirs(&full_parent, keys_gid)?;
        }
    }

    // Write via tempfile + rename for atomicity
    let dir = file_path.parent().unwrap_or(&rendered_dir);
    let tmp = tempfile::NamedTempFile::new_in(dir)
        .with_context(|| format!("cannot create temp file in '{}'", dir.display()))?;

    fs::write(tmp.path(), rendered.as_bytes()).with_context(|| format!("cannot write template '{}'", template.name))?;

    let mode = u32::from_str_radix(&template.mode, 8)
        .with_context(|| format!("invalid mode '{}' for template '{}'", template.mode, template.name))?;

    fs::set_permissions(tmp.path(), fs::Permissions::from_mode(mode))?;

    if !ignore_passwd {
        let (uid, gid) =
            resolve_ownership(template.owner.as_deref(), template.uid, template.group.as_deref(), template.gid)?;

        nix::unistd::chown(tmp.path(), Some(nix::unistd::Uid::from_raw(uid)), Some(nix::unistd::Gid::from_raw(gid)))?;
    }

    tmp.persist(&file_path)
        .with_context(|| format!("cannot rename temp file to '{}'", file_path.display()))?;

    Ok(())
}

fn create_parent_dirs(path: &Path, keys_gid: u32) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).with_context(|| format!("cannot create directory '{}'", path.display()))?;

        nix::unistd::chown(path, Some(nix::unistd::Uid::from_raw(0)), Some(nix::unistd::Gid::from_raw(keys_gid))).ok(); // Best effort
    }
    Ok(())
}

// ── Symlinks ───────────────────────────────────────────────────────────

/// Atomically update the secrets symlink to point to the new generation.
pub fn atomic_symlink(target: &Path, link_path: &str) -> Result<()> {
    let link = Path::new(link_path);

    // Create temp dir in the same parent for atomic rename
    let parent = link.parent().unwrap_or(Path::new("/"));
    let tmp_dir =
        tempfile::tempdir_in(parent).with_context(|| format!("cannot create temp dir in '{}'", parent.display()))?;

    let tmp_link = tmp_dir.path().join("tmp.symlink");
    std::os::unix::fs::symlink(target, &tmp_link)
        .with_context(|| format!("cannot create symlink at '{}'", tmp_link.display()))?;

    fs::rename(&tmp_link, link).with_context(|| format!("cannot rename symlink to '{link_path}'"))?;

    // tmp_dir cleanup happens on drop
    Ok(())
}

/// Create per-secret symlinks at their configured paths.
pub fn create_secret_symlinks(
    gen_dir: &Path,
    _symlink_path: &str,
    secrets: &[SecretEntry],
    ignore_passwd: bool,
) -> Result<()> {
    for secret in secrets {
        let target_file = gen_dir.join(&secret.name);

        // If the target == path, no symlink needed
        let target_str = target_file.to_string_lossy();
        if target_str == secret.path {
            continue;
        }

        let link_path = Path::new(&secret.path);

        // Create parent directory
        if let Some(parent) = link_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).with_context(|| format!("cannot create parent of '{}'", secret.path))?;
            }
        }

        // Remove existing link/file
        if link_path.exists() || link_path.symlink_metadata().is_ok() {
            fs::remove_file(link_path).ok();
        }

        // Create the symlink via secure method (temp dir + chown + rename)
        if !ignore_passwd {
            let (uid, gid) =
                resolve_ownership(secret.owner.as_deref(), secret.uid, secret.group.as_deref(), secret.gid)?;

            secure_symlink_chown(&target_file, link_path, uid, gid)?;
        } else {
            std::os::unix::fs::symlink(&target_file, link_path)
                .with_context(|| format!("cannot create symlink '{}'", secret.path))?;
        }
    }

    Ok(())
}

/// Create symlink with specific ownership (matching Go's SecureSymlinkChown).
fn secure_symlink_chown(target: &Path, link_path: &Path, uid: u32, gid: u32) -> Result<()> {
    let parent = link_path.parent().unwrap_or(Path::new("/tmp"));
    let tmp_dir = tempfile::tempdir_in(parent).with_context(|| format!("cannot create temp dir for symlink chown"))?;

    let tmp_link = tmp_dir.path().join(link_path.file_name().unwrap_or_default());
    std::os::unix::fs::symlink(target, &tmp_link)?;

    // lchown the symlink (best effort — may fail for non-root)
    nix::unistd::chown(
        tmp_link.as_path(),
        Some(nix::unistd::Uid::from_raw(uid)),
        Some(nix::unistd::Gid::from_raw(gid)),
    )
    .ok();

    fs::rename(&tmp_link, link_path).with_context(|| format!("cannot move symlink to '{}'", link_path.display()))?;

    Ok(())
}

/// Create per-template symlinks at their configured paths.
pub fn create_template_symlinks(gen_dir: &Path, templates: &[TemplateEntry], ignore_passwd: bool) -> Result<()> {
    for template in templates {
        let target_file = gen_dir.join("rendered").join(&template.name);

        if target_file.to_string_lossy() == template.path {
            continue;
        }

        let link_path = Path::new(&template.path);

        if let Some(parent) = link_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        if link_path.exists() || link_path.symlink_metadata().is_ok() {
            fs::remove_file(link_path).ok();
        }

        if !ignore_passwd {
            let (uid, gid) =
                resolve_ownership(template.owner.as_deref(), template.uid, template.group.as_deref(), template.gid)?;
            secure_symlink_chown(&target_file, link_path, uid, gid)?;
        } else {
            std::os::unix::fs::symlink(&target_file, link_path)?;
        }
    }

    Ok(())
}

// ── Generation pruning ─────────────────────────────────────────────────

/// Remove old generation directories, keeping at most `keep` generations.
pub fn prune_generations(mount_point: &str, current_gen: u64, keep: u32) -> Result<()> {
    if keep == 0 {
        return Ok(());
    }

    let entries = fs::read_dir(mount_point).with_context(|| format!("cannot read '{mount_point}'"))?;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        let Ok(gen_num) = name_str.parse::<u64>() else {
            continue;
        };

        // Never prune current generation
        if gen_num == current_gen {
            continue;
        }

        if current_gen.saturating_sub(keep as u64) >= gen_num {
            fs::remove_dir_all(entry.path()).ok();
            debug!("pruned generation {gen_num}");
        }
    }

    Ok(())
}

// ── Change detection ───────────────────────────────────────────────────

/// Detect changes between old and new generation and collect affected units.
pub fn detect_changes(
    symlink_path: &str,
    gen_dir: &Path,
    secrets: &[SecretEntry],
    templates: &[TemplateEntry],
    is_dry: bool,
    log_changes: bool,
) -> Result<()> {
    // If the symlink doesn't exist yet, we're in stage-2-init — no restarts needed
    if !Path::new(symlink_path).exists() {
        return Ok(());
    }

    let mut restart_units: Vec<String> = Vec::new();
    let mut reload_units: Vec<String> = Vec::new();

    // Check secrets
    for secret in secrets {
        let old_path = PathBuf::from(symlink_path).join(&secret.name);
        let new_path = gen_dir.join(&secret.name);

        let changed = match (fs::read(&old_path), fs::read(&new_path)) {
            (Ok(old), Ok(new)) => old != new,
            (Err(_), Ok(_)) => true, // new secret
            _ => false,
        };

        if changed {
            restart_units.extend(secret.restart_units.iter().cloned());
            reload_units.extend(secret.reload_units.iter().cloned());
            if log_changes {
                eprintln!("sops-install-secrets: secret '{}' changed", secret.name);
            }
        }
    }

    // Check templates
    for template in templates {
        let old_path = PathBuf::from(symlink_path).join("rendered").join(&template.name);
        let new_path = gen_dir.join("rendered").join(&template.name);

        let changed = match (fs::read(&old_path), fs::read(&new_path)) {
            (Ok(old), Ok(new)) => old != new,
            (Err(_), Ok(_)) => true,
            _ => false,
        };

        if changed {
            restart_units.extend(template.restart_units.iter().cloned());
            reload_units.extend(template.reload_units.iter().cloned());
        }
    }

    // Write unit lists
    let prefix = if is_dry {
        "/run/nixos/dry-activation"
    } else {
        "/run/nixos/activation"
    };

    write_unit_list(&format!("{prefix}-restart-list"), &restart_units)?;
    write_unit_list(&format!("{prefix}-reload-list"), &reload_units)?;

    Ok(())
}

fn write_unit_list(path: &str, units: &[String]) -> Result<()> {
    if units.is_empty() {
        return Ok(());
    }

    let parent = Path::new(path).parent().unwrap_or(Path::new("/"));
    if !parent.exists() {
        return Ok(());
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("cannot open '{path}'"))?;

    use std::io::Write;
    for unit in units {
        writeln!(file, "{unit}")?;
    }

    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Resolve owner/group to numeric UID/GID.
fn resolve_ownership(owner: Option<&str>, uid: u32, group: Option<&str>, gid: u32) -> Result<(u32, u32)> {
    let resolved_uid = if let Some(name) = owner {
        match nix::unistd::User::from_name(name) {
            Ok(Some(u)) => u.uid.as_raw(),
            Ok(None) => bail!("user '{name}' not found"),
            Err(e) => bail!("failed to lookup user '{name}': {e}"),
        }
    } else {
        uid
    };

    let resolved_gid = if let Some(name) = group {
        match nix::unistd::Group::from_name(name) {
            Ok(Some(g)) => g.gid.as_raw(),
            Ok(None) => bail!("group '{name}' not found"),
            Err(e) => bail!("failed to lookup group '{name}': {e}"),
        }
    } else {
        gid
    };

    Ok((resolved_uid, resolved_gid))
}

/// Look up the "keys" group GID (falling back to "nogroup").
pub fn lookup_keys_gid() -> Result<u32> {
    if let Ok(Some(g)) = nix::unistd::Group::from_name("keys") {
        return Ok(g.gid.as_raw());
    }
    if let Ok(Some(g)) = nix::unistd::Group::from_name("nogroup") {
        return Ok(g.gid.as_raw());
    }
    bail!("can't find group 'keys' nor 'nogroup'");
}
