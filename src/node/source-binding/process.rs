//! Explicit Nix verifier capability. No ambient executable lookup or inherited environment.

/// Batch size for one `nix hash path` invocation; output lines are checked in argument order.
pub(super) const HASH_BATCH: usize = 512;
const HASH_LINE_BYTES: usize = 128;
const PATH_INFO_SLACK_BYTES: usize = 1_048_576;

pub(super) fn run(trust: &super::Trust, arguments: &[&str]) -> crate::error::Result<Vec<u8>> {
    let stdout = molten_core::startup_binding::closure::MAX_PATHS
        * molten_core::startup_binding::closure::MAX_PATH_JSON_BYTES
        + PATH_INFO_SLACK_BYTES;
    run_paths(trust, arguments, std::slice::from_ref(&trust.output), stdout)
}

/// One verifier process per batch. The daemon store keeps signature and NAR checks client-side
/// against the overridden trusted keys while remaining usable by unprivileged node users.
pub(super) fn run_paths(
    trust: &super::Trust,
    arguments: &[&str],
    paths: &[std::path::PathBuf],
    stdout_max_bytes: usize,
) -> crate::error::Result<Vec<u8>> {
    let policy = bounded_exec::OutcomePolicy::new(vec![0], true, true)
        .map_err(|error| super::invalid(format!("source verifier policy: {error:?}")))?;
    let mut args: Vec<std::ffi::OsString> = vec![
        "--extra-experimental-features".into(),
        "nix-command blake3-hashes".into(),
        "--store".into(),
        "daemon".into(),
        "--offline".into(),
        "--option".into(),
        "trusted-public-keys".into(),
        trust.public_key.clone().into(),
        "--option".into(),
        "extra-trusted-public-keys".into(),
        "".into(),
        "--option".into(),
        "secret-key-files".into(),
        "".into(),
        "--option".into(),
        "substituters".into(),
        "".into(),
        "--option".into(),
        "plugin-files".into(),
        "".into(),
    ];
    args.extend(arguments.iter().map(std::ffi::OsString::from));
    args.extend(paths.iter().map(|path| path.as_os_str().to_owned()));
    let output = bounded_exec::run(bounded_exec::RunRequest {
        command: bounded_exec::CommandSpec {
            program: trust.nix.clone(),
            args,
            current_dir: std::path::PathBuf::from("/"),
            environment_mode: bounded_exec::EnvironmentMode::Clear,
            environment: Vec::new(),
            input: bounded_exec::Input::Null,
        },
        limits: bounded_exec::ExecutionLimits {
            timeout_ms: 600_000,
            // bounded-exec rejects a zero stdin bound; `Input::Null` still supplies no bytes.
            stdin_max_bytes: 1,
            stdout_max_bytes,
            stderr_max_bytes: 65_536,
            poll_interval_ms: 10,
            teardown_timeout_ms: 1000,
        },
        termination_scope: bounded_exec::TerminationScope::ProcessGroup,
        outcome_policy: policy,
    })
    .map_err(|error| super::invalid(format!("source verifier execution: {error:?}")))?;
    if output.disposition != bounded_exec::Disposition::Succeeded {
        return Err(super::invalid(format!("source verifier denied: {:?}", output.disposition)));
    }
    Ok(output.stdout.bytes)
}

pub(super) fn hash_batch(trust: &super::Trust, paths: &[std::path::PathBuf]) -> crate::error::Result<Vec<u8>> {
    run_paths(
        trust,
        &["hash", "path", "--type", "blake3", "--format", "base16"],
        paths,
        paths.len() * HASH_LINE_BYTES,
    )
}

pub(super) fn authenticate(trust: &super::Trust) -> crate::error::Result<()> {
    let bytes = run(trust, &["path-info", "--json"])?;
    let info: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| super::invalid(format!("source path-info: {error}")))?;
    let row = info
        .get(trust.output.to_string_lossy().as_ref())
        .ok_or_else(|| super::invalid("source path-info has no exact output"))?;
    if row.get("ca").is_some_and(|value| !value.is_null() && value.as_str() != Some("")) {
        return Err(super::invalid("content-addressed trust is not producer authentication"));
    }
    let signatures = row
        .get("signatures")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| super::invalid("source output has no real signatures"))?;
    let key_name = trust
        .public_key
        .split_once(':')
        .map(|pair| pair.0)
        .ok_or_else(|| super::invalid("source trust key has no full key material"))?;
    let prefix = format!("{key_name}:");
    if !signatures.iter().any(|signature| signature.as_str().is_some_and(|text| text.starts_with(&prefix))) {
        return Err(super::invalid("source output has no signature by the independently selected key"));
    }
    run(trust, &["store", "verify", "--sigs-needed", "1"])?;
    Ok(())
}
