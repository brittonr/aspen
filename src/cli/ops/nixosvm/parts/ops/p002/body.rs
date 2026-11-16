
const STATE_TREE_MAX_ENTRIES: usize = 4_096;
const STATE_TREE_MAX_BYTES: u64 = 256 * 1_024 * 1_024;
const STATE_HASH_BLOCK_BYTES: usize = 16 * 1_024;

struct ObservationSource {
    binding_ref: String,
    config_ref: String,
    startup_ref: String,
    executable_ref: String,
}

fn run_state_observe(input: super::command::StateObserveInput) -> Outcome<()> {
    use cap_std::fs::MetadataExt;
    if !matches!(input.phase.as_str(), "pre" | "post") || input.target != "health-receipt.preserves" {
        return Err(molten::error::Failure::invalid_harness("unsupported VM state observation phase or target"));
    }
    let root_path = std::fs::canonicalize(&input.state_root).map_err(molten::error::Failure::from)?;
    let package_path = std::fs::canonicalize(&input.package_path).map_err(molten::error::Failure::from)?;
    let parent = input.out.parent().ok_or_else(|| molten::error::Failure::invalid_harness("observation output needs a parent"))?;
    let output_parent = std::fs::canonicalize(parent).map_err(molten::error::Failure::from)?;
    if !input.out.is_absolute() || output_parent.starts_with(&root_path) {
        return Err(molten::error::Failure::invalid_harness("observation output must be outside protected root"));
    }
    let root_text = root_path.to_str().ok_or_else(|| molten::error::Failure::invalid_harness("state root is not UTF-8"))?;
    let package_text = package_path.to_str().ok_or_else(|| molten::error::Failure::invalid_harness("package path is not UTF-8"))?;
    let root = cap_std::fs::Dir::open_ambient_dir(&root_path, cap_std::ambient_authority())
        .map_err(molten::error::Failure::from)?;
    let package = cap_std::fs::Dir::open_ambient_dir(&package_path, cap_std::ambient_authority())
        .map_err(molten::error::Failure::from)?;
    let root_meta = root.dir_metadata().map_err(molten::error::Failure::from)?;
    let mut block = [0u8; STATE_HASH_BLOCK_BYTES];
    let source = observation_source(&root, &package, package_text, &mut block)?;
    let is_target_absent = match root.symlink_metadata(&input.target) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Ok(_) => false,
        Err(error) => return Err(molten::error::Failure::from(error)),
    };
    let tree_ref = observe_vm_state_tree(&root, &mut block)?;
    let value = molten::nixos_vm::vm_state_observation_value(&molten::nixos_vm::VmStateObservationInput {
        phase: &input.phase,
        target_node: &input.target_node,
        state_root: root_text,
        target: &input.target,
        tree_ref: &tree_ref,
        root_uid: root_meta.uid(),
        root_gid: root_meta.gid(),
        root_mode: root_meta.mode() & 0o7777,
        target_absent: is_target_absent,
        package_path: package_text,
        executable_ref: &source.executable_ref,
        startup_ref: &source.startup_ref,
        config_ref: &source.config_ref,
        binding_ref: &source.binding_ref,
    })?;
    let name = input.out.file_name().and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| molten::error::Failure::invalid_harness("observation output needs a UTF-8 leaf"))?;
    let output = molten::node_state::DirectoryRoot::open_existing(&output_parent)?;
    output.write_new(&molten::node_state::RelativePath::parse(name)?, molten::preserves_rail::to_text(&value)?.as_bytes())
}

fn observe_vm_state_tree(root: &cap_std::fs::Dir, block: &mut [u8]) -> Outcome<String> {
    use cap_std::fs::MetadataExt;
    let mut hasher = blake3::Hasher::new_derive_key("molten.nixos-vm.protected-state-tree.v1");
    let mut entries = vec![std::path::PathBuf::new()];
    let mut total_bytes = 0u64;
    let mut entry_count = 0usize;
    while let Some(relative) = entries.pop() {
        entry_count += 1;
        let path = if relative.as_os_str().is_empty() { std::path::Path::new(".") } else { &relative };
        let meta = root.symlink_metadata(path).map_err(molten::error::Failure::from)?;
        let name = relative.to_str().ok_or_else(|| molten::error::Failure::invalid_harness("non-UTF8 state path"))?;
        let length = u64::try_from(name.len()).map_err(|_| molten::error::Failure::invalid_harness("state path length overflow"))?;
        hasher.update(&length.to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update(&meta.mode().to_le_bytes());
        hasher.update(&meta.uid().to_le_bytes());
        hasher.update(&meta.gid().to_le_bytes());
        if meta.is_dir() {
            hasher.update(b"d");
            let remaining = STATE_TREE_MAX_ENTRIES - entry_count - entries.len();
            let mut children = Vec::new();
            for entry in root.read_dir(path).map_err(molten::error::Failure::from)? {
                if children.len() >= remaining {
                    return Err(molten::error::Failure::invalid_harness("VM state tree entry bound exceeded"));
                }
                children.push(relative.join(entry.map_err(molten::error::Failure::from)?.file_name()));
            }
            let after = root.symlink_metadata(path).map_err(molten::error::Failure::from)?;
            if !has_same_observed_metadata(&meta, &after) {
                return Err(molten::error::Failure::invalid_harness("state directory changed during observation"));
            }
            children.sort_unstable();
            entries.extend(children.into_iter().rev());
        } else if meta.is_file() {
            let (digest, observed) = hash_observed_member(root, path, STATE_TREE_MAX_BYTES - total_bytes, block)?;
            if !has_same_observed_metadata(&meta, &observed) {
                return Err(molten::error::Failure::invalid_harness("state file changed before observation"));
            }
            total_bytes += observed.len();
            hasher.update(b"f");
            hasher.update(b"blake3:");
            hasher.update(digest.to_hex().as_bytes());
        } else {
            return Err(molten::error::Failure::invalid_harness("unsupported protected state entry"));
        }
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn observation_source(
    root: &cap_std::fs::Dir,
    package: &cap_std::fs::Dir,
    selected_output: &str,
    block: &mut [u8],
) -> Outcome<ObservationSource> {
    let trust: serde_json::Value = serde_json::from_slice(&read_observed_member(root, "source-gate-trust.json")?)
        .map_err(|error| molten::error::Failure::invalid_harness(format!("observed source trust: {error}")))?;
    if trust.get("output").and_then(serde_json::Value::as_str) != Some(selected_output) {
        return Err(molten::error::Failure::invalid_harness("observed source trust does not select package"));
    }
    let binding = read_observed_member(package, "binding.json")?;
    let binding_ref = molten::preserves_rail::content_ref_from_bytes(&binding);
    let config = read_observed_preserves(root, "config.preserves")?;
    let config_ref = molten::preserves_rail::canonical_hash(&config)?;
    let startup = molten::node_runtime::parse_node_startup_receipt(&read_observed_preserves(root, "startup-receipt.preserves")?)?;
    if startup.config_ref != config_ref || startup.version_refs.as_slice() != [binding_ref.as_str()] {
        return Err(molten::error::Failure::invalid_harness("observed startup is not bound to selected package/config"));
    }
    let (digest, _) = hash_observed_member(package, std::path::Path::new("bin/molten"), molten::node_state::MAX_EXECUTABLE_BYTES, block)?;
    let executable_ref = format!("blake3:{}", digest.to_hex());
    let binding_json: serde_json::Value = serde_json::from_slice(&binding)
        .map_err(|error| molten::error::Failure::invalid_harness(format!("observed binding: {error}")))?;
    if binding_json.get("executable_ref").and_then(serde_json::Value::as_str) != Some(executable_ref.as_str()) {
        return Err(molten::error::Failure::invalid_harness("observed executable differs from source binding"));
    }
    Ok(ObservationSource { binding_ref, config_ref, startup_ref: startup.receipt_ref, executable_ref })
}

fn open_observed_member(root: &cap_std::fs::Dir, path: &std::path::Path) -> Outcome<cap_std::fs::File> {
    use cap_fs_ext::OpenOptionsFollowExt;
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
    let file = root.open_with(path, &options).map_err(molten::error::Failure::from)?;
    if !file.metadata().map_err(molten::error::Failure::from)?.is_file() {
        return Err(molten::error::Failure::invalid_harness("observed member is not a regular file"));
    }
    Ok(file)
}

fn read_observed_member(root: &cap_std::fs::Dir, path: &str) -> Outcome<Vec<u8>> {
    use std::io::Read;
    let mut file = open_observed_member(root, std::path::Path::new(path))?;
    let before = file.metadata().map_err(molten::error::Failure::from)?;
    if before.len() > molten::node_state::MAX_NODE_STATE_FILE_BYTES {
        return Err(molten::error::Failure::invalid_harness("observed member byte bound exceeded"));
    }
    let capacity = usize::try_from(before.len() + 1)
        .map_err(|_| molten::error::Failure::invalid_harness("observed member size overflow"))?;
    let mut bytes = Vec::with_capacity(capacity);
    (&mut file).take(before.len() + 1).read_to_end(&mut bytes).map_err(molten::error::Failure::from)?;
    let after = file.metadata().map_err(molten::error::Failure::from)?;
    if u64::try_from(bytes.len()).ok() != Some(before.len()) || !has_same_observed_metadata(&before, &after) {
        return Err(molten::error::Failure::invalid_harness("member changed during observation"));
    }
    Ok(bytes)
}

fn read_observed_preserves(root: &cap_std::fs::Dir, path: &str) -> Outcome<preserves::IOValue> {
    let text = String::from_utf8(read_observed_member(root, path)?)
        .map_err(|_| molten::error::Failure::invalid_harness("observed Preserves is not UTF-8"))?;
    molten::preserves_rail::parse_text(&text)
}

fn hash_observed_member(
    root: &cap_std::fs::Dir,
    path: &std::path::Path,
    maximum: u64,
    block: &mut [u8],
) -> Outcome<(blake3::Hash, cap_std::fs::Metadata)> {
    use std::io::Read;
    let mut file = open_observed_member(root, path)?;
    let before = file.metadata().map_err(molten::error::Failure::from)?;
    if before.len() > maximum {
        return Err(molten::error::Failure::invalid_harness("observed file byte bound exceeded"));
    }
    let mut hasher = blake3::Hasher::new();
    let mut reader = (&mut file).take(before.len() + 1);
    let mut total = 0u64;
    loop {
        let read = reader.read(block).map_err(molten::error::Failure::from)?;
        if read == 0 { break; }
        total += u64::try_from(read).map_err(|_| molten::error::Failure::invalid_harness("read size overflow"))?;
        if total > before.len() {
            return Err(molten::error::Failure::invalid_harness("observed file grew during observation"));
        }
        hasher.update(&block[..read]);
    }
    let after = file.metadata().map_err(molten::error::Failure::from)?;
    let named = root.symlink_metadata(path).map_err(molten::error::Failure::from)?;
    if total != before.len() || !has_same_observed_metadata(&before, &after) || !has_same_observed_metadata(&after, &named) {
        return Err(molten::error::Failure::invalid_harness("observed file changed during readback"));
    }
    Ok((hasher.finalize(), after))
}

fn has_same_observed_metadata(left: &cap_std::fs::Metadata, right: &cap_std::fs::Metadata) -> bool {
    use cap_std::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino() && left.mode() == right.mode()
        && left.uid() == right.uid() && left.gid() == right.gid() && left.len() == right.len()
        && left.mtime() == right.mtime() && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime() && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(test)]
mod member_checks {
    use super::*;
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/ops/nixosvm/parts/ops/p002/tests.rs"));
}
