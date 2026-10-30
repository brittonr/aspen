use std::io::Read;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

pub(super) struct Prepared {
    pub root: molten::node_state::DirectoryRoot,
    pub root_path: std::path::PathBuf,
    pub output: std::path::PathBuf,
    pub validation: molten::node_daemon::SelectedNodeOutputValidation,
    pub callback_ref: String,
    pub artifact_ref: String,
    pub closure_ref: String,
    pub kind_ref: String,
    pub target_ref: String,
    pub materialization_ref: String,
    pub provenance_ref: String,
}

pub(super) fn prepare(args: &super::Arguments) -> molten::error::Result<Prepared> {
    let root = create_root(args)?;
    let trust = read_input(&args.source_trust, 16_384)?;
    let parsed: serde_json::Value =
        serde_json::from_slice(&trust).map_err(|error| super::invalid(error.to_string()))?;
    let selected = parsed
        .get("output")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| super::invalid("source trust must select one actual signed output"))?;
    if std::path::Path::new(selected) != args.signed_output {
        return Err(super::invalid("explicit output differs from independent trust selection"));
    }
    root.write_new(&molten::node_state::RelativePath::parse("source-gate-trust.json")?, &trust)?;
    let validation = molten::node_daemon::verify_selected_node_output_with_root(&root)?;
    root.write_new(
        &molten::node_state::RelativePath::parse("selected-source-validation.preserves")?,
        &molten::preserves_rail::canonical_bytes(&validation.selected_output_validation_value)?,
    )?;
    let output = cap_std::fs::Dir::open_ambient_dir(&args.signed_output, cap_std::ambient_authority())
        .map_err(molten::error::Failure::from)?;
    let (callback_ref, callback_size, header) = publish_elf(&root, &output)?;
    let (kind_ref, target_ref) = elf(&root, &header)?;
    let closure_ref = closure(&output, &validation)?;
    let provenance_ref = record(&root, "signed-nix-native-provenance-v1", vec![
        molten::preserves_rail::string(selected),
        molten::preserves_rail::string(&validation.binding_ref),
        molten::preserves_rail::string(&validation.selected_output_validation_ref),
        molten::preserves_rail::string("bin/molten-native-extension-fixture"),
        molten::preserves_rail::string(&callback_ref),
        molten::preserves_rail::string(&closure_ref),
    ])?;
    let materialization_ref = record(&root, "owned-native-materialization-v1", vec![
        molten::preserves_rail::string(&callback_ref),
        molten::preserves_rail::string("artifacts/native-callback"),
        molten::preserves_rail::u64_value(callback_size),
        molten::preserves_rail::string(&provenance_ref),
    ])?;
    let artifact_ref = record(&root, "experimental-native-artifact-v1", vec![
        molten::preserves_rail::string(&callback_ref),
        molten::preserves_rail::string(&kind_ref),
        molten::preserves_rail::string(&target_ref),
        molten::preserves_rail::string(&closure_ref),
        molten::preserves_rail::string(&provenance_ref),
    ])?;
    Ok(Prepared {
        root,
        root_path: args.output_parent.join(&args.name),
        output: args.signed_output.clone(),
        validation,
        callback_ref,
        artifact_ref,
        closure_ref,
        kind_ref,
        target_ref,
        materialization_ref,
        provenance_ref,
    })
}

fn create_root(args: &super::Arguments) -> molten::error::Result<molten::node_state::DirectoryRoot> {
    if !args.output_parent.is_absolute()
        || args.name.is_empty()
        || args.name.len() > 96
        || matches!(args.name.as_str(), "." | "..")
        || !args.name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
    {
        return Err(super::invalid("output requires an absolute existing parent and one bounded owned name"));
    }
    molten::node_state::DirectoryRoot::open_existing(&args.output_parent)?;
    let owned = args.output_parent.join(&args.name);
    // Exclusive creation: an existing child is never reused or clobbered.
    std::fs::create_dir(&owned).map_err(molten::error::Failure::from)?;
    let root = molten::node_state::DirectoryRoot::open_existing(&owned)?;
    root.create_layout()?;
    root.create_dir_all(&molten::node_state::RelativePath::parse("evidence")?)?;
    root.create_dir_all(&molten::node_state::RelativePath::parse("artifacts")?)?;
    Ok(root)
}

fn closure(
    output: &cap_std::fs::Dir,
    validation: &molten::node_daemon::SelectedNodeOutputValidation,
) -> molten::error::Result<String> {
    let binding_bytes = read_capability(output, "binding.json", 65_536)?;
    if molten::preserves_rail::content_ref_from_bytes(&binding_bytes) != validation.binding_ref {
        return Err(super::invalid("selected immutable source binding changed during authoring"));
    }
    let binding: molten::core_api::startup_binding::Binding = serde_json::from_slice(&binding_bytes)
        .map_err(|_| super::invalid("verified source binding cannot be decoded"))?;
    Ok(binding.dependencies_ref)
}

pub(super) fn record(
    root: &molten::node_state::DirectoryRoot,
    tag: &str,
    fields: Vec<preserves::IOValue>,
) -> molten::error::Result<String> {
    retain(root, &preserves::IOValue::record(preserves::IOValue::symbol(tag.to_owned()), fields))
}

pub(super) fn retain(
    root: &molten::node_state::DirectoryRoot,
    value: &preserves::IOValue,
) -> molten::error::Result<String> {
    let bytes = molten::preserves_rail::canonical_bytes(value)?;
    let reference = molten::preserves_rail::content_ref_from_bytes(&bytes);
    let path = molten::node_state::RelativePath::parse(&format!("evidence/{}.preserves", &reference[7..]))?;
    if root.entry_kind(&path)?.is_some() {
        if root.read(&path, 1_048_576)? != bytes {
            return Err(super::invalid("owned evidence identity collision"));
        }
    } else {
        root.write_new(&path, &bytes)?;
    }
    Ok(reference)
}

pub(super) fn read_input(path: &std::path::Path, maximum: u64) -> molten::error::Result<Vec<u8>> {
    let file = std::fs::File::open(path).map_err(molten::error::Failure::from)?;
    if !file.metadata().map_err(molten::error::Failure::from)?.is_file() {
        return Err(super::invalid("authoring input must be a regular file"));
    }
    read_file(file, maximum)
}

fn read_capability(root: &cap_std::fs::Dir, path: &str, maximum: u64) -> molten::error::Result<Vec<u8>> {
    let file = root.open(path).map_err(molten::error::Failure::from)?;
    if !file.metadata().map_err(molten::error::Failure::from)?.is_file() {
        return Err(super::invalid("signed producer member is not a regular file"));
    }
    read_file(file, maximum)
}

fn read_file(file: impl Read, maximum: u64) -> molten::error::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes).map_err(molten::error::Failure::from)?;
    if bytes.len() as u64 > maximum {
        return Err(super::invalid("authoring input exceeds its byte bound"));
    }
    Ok(bytes)
}

fn publish_elf(
    root: &molten::node_state::DirectoryRoot,
    output: &cap_std::fs::Dir,
) -> molten::error::Result<(String, u64, [u8; 20])> {
    let mut source = output.open("bin/molten-native-extension-fixture").map_err(molten::error::Failure::from)?;
    if !source.metadata().map_err(molten::error::Failure::from)?.is_file() {
        return Err(super::invalid("signed callback artifact must be a regular file"));
    }
    let mut target = root.open_database_file(&molten::node_state::RelativePath::parse("artifacts/native-callback")?)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 65_536];
    let mut header = [0_u8; 20];
    let mut header_len = 0;
    let mut total = 0_u64;
    loop {
        let count = source.read(&mut buffer).map_err(molten::error::Failure::from)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or_else(|| super::invalid("ELF size overflow"))?;
        if total > 536_870_912 {
            return Err(super::invalid("actual callback ELF exceeds its authoring bound"));
        }
        let copied = (header.len() - header_len).min(count);
        header[header_len..header_len + copied].copy_from_slice(&buffer[..copied]);
        header_len += copied;
        hasher.update(&buffer[..count]);
        target.write_all(&buffer[..count]).map_err(molten::error::Failure::from)?;
    }
    if header_len != header.len() {
        return Err(super::invalid("actual callback ELF is truncated"));
    }
    target
        .set_permissions(std::fs::Permissions::from_mode(0o500))
        .map_err(molten::error::Failure::from)?;
    target.sync_all().map_err(molten::error::Failure::from)?;
    Ok((format!("blake3:{}", hasher.finalize().to_hex()), total, header))
}

fn elf(root: &molten::node_state::DirectoryRoot, bytes: &[u8]) -> molten::error::Result<(String, String)> {
    if bytes.len() < 20 || &bytes[..4] != b"\x7fELF" || bytes[4] != 2 || bytes[5] != 1 {
        return Err(super::invalid("experimental callback cohort requires an actual ELF64 little-endian artifact"));
    }
    let machine = u16::from_le_bytes([bytes[18], bytes[19]]);
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) || machine != 62 {
        return Err(super::invalid("experimental native cohort is scoped to Linux x86-64 process groups"));
    }
    let kind = record(root, "observed-native-artifact-kind-v1", vec![molten::preserves_rail::string(
        "ELF64-little-endian",
    )])?;
    let target = record(root, "observed-native-target-v1", vec![
        molten::preserves_rail::u64_value(u64::from(machine)),
        molten::preserves_rail::u64_value(u64::from(bytes[7])),
    ])?;
    Ok((kind, target))
}
