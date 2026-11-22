//! Effect shell for the immutable semantic-history-contract executable.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Admitted {
    pub executable: std::path::PathBuf,
    pub executable_blake3: String,
    pub checker_ref: String,
    pub timeout_ms: u64,
    pub max_bytes: usize,
}

pub struct Files<'a> {
    pub root: &'a cap_std::fs::Dir,
    pub history: &'a serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct CheckedReport {
    pub(crate) history_ref: String,
    pub(crate) report_ref: String,
    pub(crate) checker_ref: String,
    pub(crate) report: serde_json::Value,
}

impl CheckedReport {
    pub fn report(&self) -> &serde_json::Value {
        &self.report
    }
    pub fn history_ref(&self) -> &str {
        &self.history_ref
    }
    pub fn report_ref(&self) -> &str {
        &self.report_ref
    }
    pub fn checker_ref(&self) -> &str {
        &self.checker_ref
    }
}

// r[impl molten.consensus.live_linearizability]
#[cfg(target_os = "linux")]
pub fn run(checker: &Admitted, files: &Files<'_>) -> Result<CheckedReport, String> {
    use std::os::fd::AsRawFd;
    if !checker.executable.is_absolute()
        || !reference(&checker.checker_ref)
        || !reference(&checker.executable_blake3)
        || !(1..=120_000).contains(&checker.timeout_ms)
        || !(1..=8_388_608).contains(&checker.max_bytes)
    {
        return Err("checker-admission".into());
    }
    let executable = pinned_executable(checker)?;
    let bytes = serde_json::to_vec(files.history).map_err(|e| e.to_string())?;
    if bytes.len() > checker.max_bytes {
        return Err("history-bound".into());
    }
    // Caller supplies a dedicated admitted directory; no ambient path reconstruction.
    files.root.write("history.json", bytes).map_err(|e| e.to_string())?;
    match files.root.remove_file("report.json") {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    let process = std::process::id();
    let binary = format!("/proc/{process}/fd/{}", executable.as_raw_fd());
    let directory = format!("/proc/{process}/fd/{}", files.root.as_raw_fd());
    for action in ["check", "validate"] {
        execute(&binary, action, &directory, checker.timeout_ms)?;
    }
    let file = files.root.open("report.json").map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut std::io::Read::take(file, checker.max_bytes as u64 + 1), &mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > checker.max_bytes {
        return Err("report-bound".into());
    }
    let report = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    Ok(CheckedReport {
        history_ref: canonical_ref(files.history)?,
        report_ref: canonical_ref(&report)?,
        checker_ref: checker.checker_ref.clone(),
        report,
    })
}

#[cfg(target_os = "linux")]
fn pinned_executable(checker: &Admitted) -> Result<std::fs::File, String> {
    use std::os::fd::AsRawFd;

    use rustix::fs::Access;
    use rustix::fs::AtFlags;
    use rustix::fs::MemfdFlags;
    use rustix::fs::Mode;
    use rustix::fs::SealFlags;

    const MAX_EXECUTABLE_BYTES: u64 = 128 * 1024 * 1024;

    let source = rustix::fs::open(
        &checker.executable,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC | rustix::fs::OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map(std::fs::File::from)
    .map_err(|e| e.to_string())?;
    let metadata = source.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("checker-not-file".into());
    }
    if !(1..=MAX_EXECUTABLE_BYTES).contains(&metadata.len()) {
        return Err("checker-executable-bound".into());
    }
    let source_fd_path = format!("/proc/self/fd/{}", source.as_raw_fd());
    rustix::fs::accessat(rustix::fs::CWD, source_fd_path, Access::EXEC_OK, AtFlags::EACCESS)
        .map_err(|e| format!("checker-executable-access:{e}"))?;

    // An open file pins its inode, not its contents. Hash the sealed image that both child
    // operations will execute, rather than measuring a file that can be rewritten in place.
    let fd = rustix::fs::memfd_create(
        "reliability-checker",
        MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING | MemfdFlags::EXEC,
    )
    .map_err(|e| e.to_string())?;
    let mut image = std::fs::File::from(fd);
    let copied =
        std::io::copy(&mut std::io::Read::take(source, metadata.len()), &mut image).map_err(|e| e.to_string())?;
    if copied != metadata.len() {
        return Err("checker-identity".into());
    }
    rustix::fs::fchmod(&image, Mode::RUSR | Mode::XUSR).map_err(|e| e.to_string())?;
    rustix::fs::fcntl_add_seals(&image, SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL)
        .map_err(|e| e.to_string())?;
    let mut executable =
        std::fs::File::open(format!("/proc/self/fd/{}", image.as_raw_fd())).map_err(|e| e.to_string())?;
    drop(image);
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut executable, &mut hasher).map_err(|e| e.to_string())?;
    if format!("blake3:{}", hasher.finalize().to_hex()) != checker.executable_blake3 {
        return Err("checker-identity".into());
    }
    Ok(executable)
}

#[cfg(not(target_os = "linux"))]
pub fn run(_: &Admitted, _: &Files<'_>) -> Result<CheckedReport, String> {
    Err("checker-requires-linux-capability-paths".into())
}

#[cfg(target_os = "linux")]
fn execute(binary: &str, action: &str, directory: &str, timeout_ms: u64) -> Result<(), String> {
    let mut child = std::process::Command::new(binary)
        .arg(action)
        .arg(format!("{directory}/history.json"))
        .arg(format!("{directory}/report.json"))
        .env_clear()
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let timeout = std::time::Duration::from_millis(timeout_ms);
    let mut deadline = match crate::fabric_time::SupervisionDeadline::after(timeout) {
        Ok(deadline) => deadline,
        Err(e) => return Err(terminate(&mut child, e.to_string())),
    };
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) => return Err(format!("checker-{action}-failed")),
            Err(e) => return Err(terminate(&mut child, e.to_string())),
            Ok(None) => {}
        }
        match deadline.is_expired() {
            Ok(false) => {}
            Ok(true) => return Err(terminate(&mut child, "checker-time-bound".into())),
            Err(e) => return Err(terminate(&mut child, e.to_string())),
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// Stops and reaps the checker; a failed stop is reported with the original diagnostic, never
/// hidden.
#[cfg(target_os = "linux")]
fn terminate(child: &mut std::process::Child, diagnostic: String) -> String {
    match child.kill().and_then(|()| child.wait()) {
        Ok(_) => diagnostic,
        Err(e) => format!("{diagnostic};checker-termination-uncertain:{e}"),
    }
}

pub fn reference(value: &str) -> bool {
    value
        .strip_prefix("blake3:")
        .is_some_and(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
}

/// Molten references are BLAKE3 over canonical packed Preserves, not JSON bytes.
pub fn canonical_ref(value: &serde_json::Value) -> Result<String, String> {
    crate::preserves_rail::canonical_hash(&canonical_value(value)?).map_err(|e| e.to_string())
}

pub fn canonical_value(value: &serde_json::Value) -> Result<preserves::IOValue, String> {
    let mut remaining = 100_000;
    project(value, 0, &mut remaining)
}

fn project(value: &serde_json::Value, depth: usize, remaining: &mut usize) -> Result<preserves::IOValue, String> {
    if depth > 32 || *remaining == 0 {
        return Err("canonical-value-bound".into());
    }
    *remaining -= 1;
    let fields = match value {
        serde_json::Value::Null => return Ok(crate::preserves_rail::record("null", vec![])),
        serde_json::Value::Bool(v) => return Ok(crate::preserves_rail::bool_value(*v)),
        serde_json::Value::Number(v) => {
            return Ok(crate::preserves_rail::record("integer", vec![crate::preserves_rail::string(v.to_string())]));
        }
        serde_json::Value::String(v) if v.len() <= 1_048_576 => return Ok(crate::preserves_rail::string(v)),
        serde_json::Value::String(_) => return Err("canonical-string-bound".into()),
        serde_json::Value::Array(values) => {
            let values = values.iter().map(|v| project(v, depth + 1, remaining)).collect::<Result<Vec<_>, _>>()?;
            return Ok(crate::preserves_rail::sequence(values));
        }
        serde_json::Value::Object(values) => {
            let ordered: std::collections::BTreeMap<_, _> = values.iter().collect();
            ordered
                .into_iter()
                .map(|(k, v)| {
                    Ok(crate::preserves_rail::record("entry", vec![
                        crate::preserves_rail::string(k),
                        project(v, depth + 1, remaining)?,
                    ]))
                })
                .collect::<Result<Vec<_>, String>>()?
        }
    };
    Ok(crate::preserves_rail::record("map", fields))
}

/// Producer-owned identity protocol, distinct from Molten canonical receipt references.
pub fn semantic_history_id(history: &crate::live_reliability::history::Document) -> Result<String, String> {
    let bytes = serde_json::to_vec(history).map_err(|e| e.to_string())?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"chaoscontrol.semantic-history.v2");
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(&bytes);
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    // r[verify molten.consensus.live_linearizability]
    #[test]
    fn executes_only_the_pinned_image_for_both_actions() {
        use std::os::unix::fs::PermissionsExt;

        // A shell fixture exercises execution and substitution, not register semantics.
        let script = r#"#!/bin/sh
set -eu
case "$1" in
    check)
        printf '%s\n' '{"verdict":"valid"}' > "$3"
        if [ -f "${3%/*}/rewrite" ]; then
            printf '#!/bin/sh\nexit 0\n' > "${3%/*}/checker"
        fi
        ;;
    validate)
        [ ! -f "${3%/*}/rewrite" ]
        ;;
    *) exit 2 ;;
esac
"#;
        let workspace = crate::test_support::process_workspace("live-checker-image").expect("owned workspace");
        let root: &std::path::Path = workspace.as_ref();
        let directory =
            cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority()).expect("open work directory");
        let executable = root.join("checker");
        std::fs::write(&executable, script).expect("write checker");
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).expect("make checker executable");
        let config = super::Admitted {
            executable,
            executable_blake3: format!("blake3:{}", blake3::hash(script.as_bytes()).to_hex()),
            checker_ref: format!("blake3:{}", blake3::hash(b"reference-fixture").to_hex()),
            timeout_ms: 5_000,
            max_bytes: 1024,
        };
        let history = serde_json::json!({"input":"synthetic"});
        let files = super::Files {
            root: &directory,
            history: &history,
        };
        let checked = super::run(&config, &files).expect("execute check then validate with the original image");
        assert_eq!(checked.report(), &serde_json::json!({"verdict":"valid"}));

        directory.write("rewrite", b"").expect("enable substitution during check");
        assert_eq!(super::run(&config, &files).unwrap_err(), "checker-validate-failed");
        assert_eq!(directory.read("checker").expect("rewritten inode").as_slice(), b"#!/bin/sh\nexit 0\n");

        directory.remove_file("history.json").expect("clear work history");
        assert_eq!(super::run(&config, &files).unwrap_err(), "checker-identity");
        assert!(!directory.exists("history.json"));
    }
}
