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
#[cfg(unix)]
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
    let mut executable = std::fs::File::open(&checker.executable).map_err(|e| e.to_string())?;
    if !executable.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("checker-not-file".into());
    }
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut executable, &mut hasher).map_err(|e| e.to_string())?;
    if format!("blake3:{}", hasher.finalize().to_hex()) != checker.executable_blake3 {
        return Err("checker-identity".into());
    }
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

#[cfg(not(unix))]
pub fn run(_: &Admitted, _: &Files<'_>) -> Result<CheckedReport, String> {
    Err("checker-requires-unix-capability-paths".into())
}

#[cfg(unix)]
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
#[cfg(unix)]
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
