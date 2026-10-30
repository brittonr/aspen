/// These pins are deployment authority input, not values copied from a submitted receipt.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trust {
    pub mantle: Tool,
    pub octet: Tool,
    pub chaos: Tool,
    pub bundle_identity: String,
    pub octet_profile_identity: String,
    pub cohort_identity: String,
    pub probe_blake3: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub path: std::path::PathBuf,
    pub blake3: String,
}

pub struct Sources<'a> {
    pub root: &'a cap_std::fs::Dir,
    pub bundle: &'a std::path::Path,
    pub octet: &'a std::path::Path,
    pub module: &'a std::path::Path,
    pub input: &'a std::path::Path,
    pub probe: &'a std::path::Path,
    pub chaos_report: &'a std::path::Path,
}

/// Constructible only by running the pinned external verifiers and binding their outputs.
#[derive(Clone, Debug)]
pub struct Verified {
    artifact_ref: String,
    evidence_ref: String,
    fixture_name: String,
    input_ref: String,
}

impl Verified {
    pub fn artifact_ref(&self) -> &str {
        &self.artifact_ref
    }
    pub fn evidence_ref(&self) -> &str {
        &self.evidence_ref
    }
    pub fn fixture_name(&self) -> &str {
        &self.fixture_name
    }
    pub fn input_ref(&self) -> &str {
        &self.input_ref
    }
}

pub fn verify(trust: &Trust, sources: &Sources<'_>) -> Result<Verified, String> {
    validate_pins(trust)?;
    for path in [
        sources.bundle,
        sources.octet,
        sources.module,
        sources.probe,
        sources.input,
        sources.chaos_report,
    ] {
        relative(path)?;
    }
    let bundle = sources.root.open_dir(sources.bundle).map_err(|error| error.to_string())?;
    let octet_dir = sources.root.open_dir(sources.octet).map_err(|error| error.to_string())?;
    let module_file = sources.root.open(sources.module).map_err(|error| error.to_string())?;
    let probe_file = sources.root.open(sources.probe).map_err(|error| error.to_string())?;
    let input_file = sources.root.open(sources.input).map_err(|error| error.to_string())?;
    let report_file = sources.root.create(sources.chaos_report).map_err(|error| error.to_string())?;
    let bundle_path = command_path(&bundle)?;
    let octet_path = command_path(&octet_dir)?;
    let module_path = command_path(&module_file)?;
    let probe_path = command_path(&probe_file)?;
    let input_path = command_path(&input_file)?;
    let report_path = command_path(&report_file)?;
    run(&trust.mantle, &[std::ffi::OsStr::new("verify"), bundle_path.as_os_str()])?;
    let manifest = read_json(sources.root, &sources.bundle.join("manifest.json"))?;
    equal(&manifest, "/schema", "mantle-spacewasm-reference-bundle-v1")?;
    equal(&manifest, "/profile_id", "nasa-spacewasm-e24cf093-reference-v1")?;
    equal(&manifest, "/bundle_identity_blake3", &trust.bundle_identity)?;
    equal(&manifest, "/cohort_identity_blake3", &trust.cohort_identity)?;
    let module = read_bounded(sources.root, sources.module, 65_536)?;
    let digest = blake3::hash(&module).to_hex().to_string();
    let fixture_name = require_bundle_member(&manifest, &digest)?;
    run(&trust.octet, &[
        std::ffi::OsStr::new("artifact"),
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--artifact-dir"),
        octet_path.as_os_str(),
    ])?;
    let submitted = read_bounded(sources.root, &sources.octet.join("wasm-artifact.bin"), 65_536)?;
    if submitted != module {
        return Err("Octet artifact bytes differ".into());
    }
    let octet = read_json(sources.root, &sources.octet.join("evidence-rail-receipt.json"))?;
    verify_octet(&octet, trust, &digest)?;
    let probe = read_bounded(sources.root, sources.probe, 67_108_864)?;
    if blake3::hash(&probe).to_hex().as_str() != trust.probe_blake3 {
        return Err("differential probe is not authority-pinned".into());
    }
    let input = read_bounded(sources.root, sources.input, 65_536)?;
    crate::codec::parse_canonical_bytes(&input).map_err(|error| error.to_string())?;
    let input_digest = blake3::hash(&input).to_hex().to_string();
    run(&trust.chaos, &[
        probe_path.as_os_str(),
        module_path.as_os_str(),
        input_path.as_os_str(),
        report_path.as_os_str(),
    ])?;
    let chaos = read_json(sources.root, sources.chaos_report)?;
    verify_chaos(&chaos, trust, &digest, &input_digest)?;
    let reports = [manifest, octet, chaos]
        .iter()
        .map(|report| serde_json::to_string(report).map(crate::codec::string).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let value = crate::codec::record("molten-spacewasm-verified-external-evidence-v1", reports);
    Ok(Verified {
        artifact_ref: format!("blake3:{digest}"),
        fixture_name,
        input_ref: format!("blake3:{input_digest}"),
        evidence_ref: crate::codec::canonical_hash(&value).map_err(|error| error.to_string())?,
    })
}

fn validate_pins(trust: &Trust) -> Result<(), String> {
    for pin in [
        &trust.bundle_identity,
        &trust.octet_profile_identity,
        &trust.cohort_identity,
        &trust.probe_blake3,
    ] {
        let digest = pin.strip_prefix("blake3:").or_else(|| pin.strip_prefix("b3:")).unwrap_or(pin);
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("missing authority-pinned external identity".into());
        }
    }
    Ok(())
}

fn run(tool: &Tool, args: &[&std::ffi::OsStr]) -> Result<(), String> {
    let resolved = std::fs::canonicalize(&tool.path).map_err(|error| error.to_string())?;
    if !tool.path.is_absolute() || !resolved.starts_with("/nix/store") {
        return Err("verifier must reside in immutable Nix store".into());
    }
    let file = std::fs::File::open(&resolved).map_err(|error| error.to_string())?;
    let mut hasher = blake3::Hasher::new();
    let mut reader = std::io::Read::take(file, 134_217_729);
    let mut bytes = [0_u8; 8192];
    let mut count = 0_u64;
    loop {
        let size = std::io::Read::read(&mut reader, &mut bytes).map_err(|error| error.to_string())?;
        if size == 0 {
            break;
        }
        count += size as u64;
        hasher.update(&bytes[..size]);
    }
    if count > 134_217_728 || hasher.finalize().to_hex().as_str() != tool.blake3 {
        return Err("external verifier executable differs from bounded authority pin".into());
    }
    let mut command = std::process::Command::new(&resolved);
    command
        .args(args)
        .env_clear()
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    super::process::run(command)
}

fn require_bundle_member(manifest: &serde_json::Value, digest: &str) -> Result<String, String> {
    let members = manifest.get("members").and_then(serde_json::Value::as_array).ok_or("bundle members missing")?;
    let member = members
        .iter()
        .find(|member| {
            member.get("digest_blake3").and_then(serde_json::Value::as_str) == Some(digest)
                && member
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|path| path.starts_with("fixtures/wasm/molten-abi-"))
        })
        .ok_or("candidate is not an exact named ABI fixture bundle member")?;
    let path = member.get("path").and_then(serde_json::Value::as_str).ok_or("fixture path missing")?;
    let name = path
        .strip_prefix("fixtures/wasm/")
        .and_then(|name| name.strip_suffix(".wasm"))
        .filter(|name| !name.contains('/'))
        .ok_or("invalid fixture member path")?;
    if !matches!(
        name,
        "molten-abi-roundtrip"
            | "molten-abi-hostcall"
            | "molten-abi-invalid-pointer"
            | "molten-abi-invalid-output"
            | "molten-abi-memory-limit"
            | "molten-abi-stack-limit"
            | "molten-abi-trap"
            | "molten-abi-out-of-fuel"
    ) {
        return Err("unknown named ABI fixture".into());
    }
    Ok(name.into())
}

fn verify_octet(report: &serde_json::Value, trust: &Trust, digest: &str) -> Result<(), String> {
    for (path, expected) in [
        ("/schema", "octet-evidence-rail-receipt/v1"),
        ("/status", "passed"),
        ("/rail", "wasm-artifact"),
        ("/wasm_artifact/profile_id", "spacewasm-molten-abi-pilot"),
        ("/wasm_artifact/profile_identity", trust.octet_profile_identity.as_str()),
        ("/wasm_artifact/target_runtime/source_revision", super::profile::REVISION),
        (
            "/wasm_artifact/target_runtime/cohort_identity",
            "b3:7c537eef7bff66d0a90508715de08de79c9aaac23352e1007a6d47e7fb4674a2",
        ),
    ] {
        equal(report, path, expected)?;
    }
    equal(report, "/wasm_artifact/exact_artifact_identity", &format!("b3:{digest}"))?;
    let findings = report
        .pointer("/wasm_artifact/findings")
        .and_then(serde_json::Value::as_array)
        .ok_or("Octet findings missing")?;
    if findings
        .iter()
        .any(|finding| finding.get("severity").and_then(serde_json::Value::as_str) == Some("error"))
    {
        return Err("Octet static profile has error findings".into());
    }
    Ok(())
}

fn verify_chaos(report: &serde_json::Value, trust: &Trust, digest: &str, input_digest: &str) -> Result<(), String> {
    for (path, expected) in [
        ("/schema", "chaoscontrol.spacewasm-molten-abi-differential.v1"),
        ("/evidence_class", "full-molten-abi-runtime-agreement"),
        ("/source_archive_blake3", "0db57636fb83a8c47f0c55601236cab648dd5edfdd2567cae0619aa74b8c4da3"),
        ("/profile", "spacewasm-molten-abi-pilot"),
        ("/wasm_feature_profile", "wasm1"),
        ("/verdict", "match"),
        ("/spacewasm_revision", super::profile::REVISION),
        ("/wasmtime_version", "45.0.0"),
        ("/module_blake3", digest),
        ("/input_blake3", input_digest),
        ("/probe_blake3", trust.probe_blake3.as_str()),
    ] {
        equal(report, path, expected)?;
    }
    let bounds = serde_json::json!({"module_bytes":65536,"input_bytes":64512,"report_bytes":1048576,
        "process_seconds":30,"reference_fuel":100000,"segment_fuel":1,"uninterrupted_slice_fuel":1000,
        "memory_bytes":65536,"hostcalls":64,"reference_stack_bytes":65536,"segments":100008});
    if report.get("bounds") != Some(&bounds) {
        return Err("differential resource cohort mismatch".into());
    }
    let reference = report.get("reference").ok_or("differential reference row missing")?;
    validate_row(reference)?;
    for mode in ["uninterrupted", "segmented", "streamed"] {
        let path = format!("/interpreter/{mode}/row");
        if report.pointer(&path) != Some(reference) {
            return Err("differential normalized row mismatch".into());
        }
    }
    Ok(())
}

fn validate_row(row: &serde_json::Value) -> Result<(), String> {
    let outcome = row.get("outcome").and_then(serde_json::Value::as_str).ok_or("missing normalized outcome")?;
    if !matches!(
        outcome,
        "completed"
            | "invalid-input-pointer"
            | "invalid-output-pointer"
            | "memory-limit"
            | "stack-limit"
            | "fuel-exhausted"
            | "trap"
            | "invalid-output"
    ) {
        return Err("unsupported differential outcome".into());
    }
    let trap = row.get("trap_class").ok_or("missing normalized trap class")?;
    if outcome == "trap" {
        if !matches!(
            trap.as_str(),
            Some(
                "unreachable"
                    | "divide-by-zero"
                    | "integer-overflow"
                    | "invalid-conversion-to-integer"
                    | "memory-out-of-bounds"
                    | "table-out-of-bounds"
                    | "indirect-call-null"
                    | "bad-signature"
            )
        ) {
            return Err("unsupported or generic trap class".into());
        }
    } else if !trap.is_null() {
        return Err("non-trap outcome has trap class".into());
    }
    if matches!(outcome, "completed" | "invalid-output") {
        let hex = row.get("output_hex").and_then(serde_json::Value::as_str).ok_or("missing guest output bytes")?;
        let bytes = decode_hex(hex)?;
        let canonical = crate::codec::parse_canonical_bytes(&bytes);
        if (outcome == "completed") != canonical.is_ok() {
            return Err("differential canonical-output classification mismatch".into());
        }
    }
    if !row.get("hostcalls").is_some_and(serde_json::Value::is_array)
        || !row.get("globals").is_some_and(serde_json::Value::is_array)
        || !row.get("memory_blake3").is_some_and(serde_json::Value::is_string)
    {
        return Err("incomplete normalized state observation".into());
    }
    Ok(())
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if value.len() > 131072 || !value.len().is_multiple_of(2) {
        return Err("invalid bounded output hex".into());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let upper = (pair[0] as char).to_digit(16).ok_or("invalid output hex")?;
            let lower = (pair[1] as char).to_digit(16).ok_or("invalid output hex")?;
            Ok((upper * 16 + lower) as u8)
        })
        .collect()
}

fn read_json(root: &cap_std::fs::Dir, path: &std::path::Path) -> Result<serde_json::Value, String> {
    let bytes = read_bounded(root, path, 4_194_304)?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn read_bounded(root: &cap_std::fs::Dir, path: &std::path::Path, bound: u64) -> Result<Vec<u8>, String> {
    relative(path)?;
    let file = root.open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut std::io::Read::take(file, bound + 1), &mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > bound {
        return Err("evidence member byte bound exceeded".into());
    }
    Ok(bytes)
}

fn relative(path: &std::path::Path) -> Result<(), String> {
    if path.as_os_str().is_empty() || !path.components().all(|part| matches!(part, std::path::Component::Normal(_))) {
        return Err("evidence path must be relative without traversal".into());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn command_path(file: &impl std::os::fd::AsRawFd) -> Result<std::path::PathBuf, String> {
    let descriptor = std::os::fd::AsRawFd::as_raw_fd(file);
    Ok(std::path::PathBuf::from(format!("/proc/{}/fd/{descriptor}", std::process::id())))
}

#[cfg(not(target_os = "linux"))]
fn command_path<T>(_: &T) -> Result<std::path::PathBuf, String> {
    Err("external evidence capability path requires Linux procfs".into())
}

fn equal(value: &serde_json::Value, path: &str, expected: &str) -> Result<(), String> {
    if value.pointer(path).and_then(serde_json::Value::as_str) != Some(expected) {
        return Err(format!("external evidence binding mismatch: {path}"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "evidencetests.rs"]
mod tests;
