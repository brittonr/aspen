//! Three independently source-admitted node processes hosting the admitted collaboration service.

pub mod flow;
pub mod operations;
pub mod process;
mod scenarios;
pub mod scope;
pub mod setup;
pub mod status;

pub const READY: &str = "node-service-host-ready-v1";
/// Bound listener ownership by the experiment's 12-hour operator trust interval.
const LIVE_EVENT_TIMEOUT_MS: &str = "43200000";
/// Every signed start re-authenticates the whole producer output before readiness, which takes many
/// minutes per node on a loaded host; the supervisor bound must outlast that verification.
const READINESS_SECONDS: u64 = 3_600;
const SOURCE_TRUST_ENV: &str = "MOLTEN_TEST_SOURCE_TRUST";
/// Optional comma-separated labels from `scenarios::Case::label` for targeted re-execution after a
/// host interruption. Acceptance requires the default (unset) full sequence of baseline and cases.
const CASE_SELECTION_ENV: &str = "MOLTEN_TEST_NATIVE_CASES";

pub struct Node {
    pub index: u64,
    pub state_root: std::path::PathBuf,
    pub signed_main: std::path::PathBuf,
    pub issuer: crate::authority::Issuer,
    pub child: std::process::Child,
    pub log: std::path::PathBuf,
    pub control: Option<process::Control>,
}

impl Drop for Node {
    fn drop(&mut self) {
        if let Some(control) = self.control.as_mut() {
            control.close();
        }
        let _terminated = self.child.kill();
        let _reaped = self.child.wait();
        if let Some(control) = self.control.as_mut() {
            control.join();
        }
    }
}

/// EOF must close the actual retained owner and listener, not merely kill the process.
pub(super) fn finish(node: &mut Node) -> crate::producer::Result<()> {
    node.control.as_mut().ok_or("retained control missing")?.close();
    let mut deadline = molten::fabric_time::SupervisionDeadline::after(std::time::Duration::from_secs(120))
        .map_err(|error| format!("{error:?}"))?;
    loop {
        if let Some(status) = node.child.try_wait()? {
            if !status.success() {
                return Err(format!("node {} failed during owned shutdown: {status}", node.index).into());
            }
            break;
        }
        if deadline.is_expired().map_err(|error| format!("{error:?}"))? {
            return Err(format!("node {} did not finish owned shutdown", node.index).into());
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    node.control.as_mut().ok_or("retained control missing")?.join();
    Ok(())
}

// r[verify molten.system_extension.native_effect_lifetime]
#[test]
fn exited_child_cannot_supply_readiness() -> crate::producer::Result<()> {
    let workspace = crate::test_support::process_workspace("exited-host-readiness")?;
    let executable = std::env::current_exe()?;
    let mut child = std::process::Command::new(&executable)
        .arg("--list")
        .current_dir(&*workspace)
        .stdout(std::process::Stdio::null())
        .spawn()?;
    child.wait()?;
    let log = workspace.join("exited.log");
    std::fs::write(&log, format!("<{READY}>\n"))?;
    let mut child = Node {
        index: 0,
        state_root: workspace.to_path_buf(),
        signed_main: executable,
        issuer: crate::authority::Issuer::new([0x6b; 32])?,
        child,
        log,
        control: None,
    };
    assert!(await_ready(&mut child).is_err());
    Ok(())
}

// r[verify molten.system_extension.native_effect_installation]
// r[verify molten.system_extension.native_effect_authority]
// r[verify molten.system_extension.native_effect_completion]
// r[verify molten.system_extension.native_effect_uncertainty]
// r[verify molten.system_extension.native_effect_lifetime]
// r[verify molten.system_extension.native_effect_consumer]
#[test]
#[ignore = "requires independently provisioned MOLTEN_TEST_SOURCE_TRUST signed producer output"]
fn installed_native_effect_commits_through_raft_and_returns_a_completion() -> crate::producer::Result<()> {
    let trust = std::path::PathBuf::from(std::env::var_os(SOURCE_TRUST_ENV).ok_or("signed source trust")?);
    let output = output_of(&trust)?;
    let evidence = std::env::var_os("MOLTEN_TEST_EVIDENCE_ROOT")
        .map(std::path::PathBuf::from)
        .map(|path| molten::node_state::DirectoryRoot::open(&path))
        .transpose()?;
    let selection = match std::env::var_os(CASE_SELECTION_ENV) {
        None => None,
        Some(raw) => {
            let raw = raw.into_string().map_err(|_| "native case selection is not UTF-8")?;
            Some(parse_case_selection(&raw)?)
        }
    };
    if selection.is_none() {
        let workspace = crate::test_support::process_workspace("collaboration-native-effect")?;
        let mut cohort = crate::producer::prepare(
            workspace.as_ref(),
            &output.join("bin/molten"),
            &output.join("bin/molten-collaboration-scope"),
            &trust,
            None,
        )?;
        cohort.release_endpoints();
        let observation = operations::drive(&cohort)?;
        eprintln!("baseline observations={}", scenarios::retain(&observation, evidence.as_ref())?);
    }
    for case in selection.unwrap_or_else(|| scenarios::Case::all().to_vec()) {
        let label = case.label();
        let workspace = crate::test_support::process_workspace(label)?;
        let mut cohort = crate::producer::prepare(
            workspace.as_ref(),
            &output.join("bin/molten"),
            &output.join("bin/molten-collaboration-scope"),
            &trust,
            case.frame_budget()?,
        )?;
        cohort.release_endpoints();
        let observation = case.run(&cohort).map_err(|error| format!("{label}: {error}"))?;
        eprintln!("{label} observations={}", scenarios::retain(&observation, evidence.as_ref())?);
    }
    Ok(())
}

/// Parse a bounded, duplicate-free selection of known case labels.
fn parse_case_selection(raw: &str) -> crate::producer::Result<Vec<scenarios::Case>> {
    let mut selected: Vec<scenarios::Case> = Vec::with_capacity(scenarios::Case::all().len());
    for label in raw.split(',').map(str::trim).filter(|label| !label.is_empty()) {
        if selected.len() >= scenarios::Case::all().len() {
            return Err("native case selection exceeds the admitted case count".into());
        }
        let case = scenarios::Case::all()
            .into_iter()
            .find(|case| case.label() == label)
            .ok_or_else(|| format!("unknown native case selection: {label}"))?;
        if selected.iter().any(|existing| existing.label() == label) {
            return Err(format!("duplicate native case selection: {label}").into());
        }
        selected.push(case);
    }
    if selected.is_empty() {
        return Err("native case selection is empty".into());
    }
    Ok(selected)
}

pub fn output_of(trust: &std::path::Path) -> crate::producer::Result<std::path::PathBuf> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(trust)?.take(65_536).read_to_end(&mut bytes)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    Ok(std::path::PathBuf::from(
        value.get("output").and_then(serde_json::Value::as_str).ok_or("trust output")?,
    ))
}

/// Start one signed node process that owns the admitted collaboration service.
pub fn start(
    signed_main: &std::path::Path,
    node: &crate::producer::Node,
    index: u64,
    issuer: crate::authority::Issuer,
) -> crate::producer::Result<Node> {
    use std::os::unix::process::CommandExt;
    let ticket = node.path.join("live-ticket.preserves");
    let startup = node.root.read(&molten::node_state::RelativePath::parse(setup::STARTUP)?, 65_536)?;
    let log = node.path.join(format!("serve-{}.log", blake3::hash(&startup).to_hex()));
    let output = std::fs::OpenOptions::new().write(true).create_new(true).open(&log)?;
    let errors = output.try_clone()?;
    let mut child = std::process::Command::new(signed_main)
        .arg("node")
        .arg("serve")
        .arg("--state-root")
        .arg(&node.path)
        .args(["--live-iroh", "--live-max-events", "1", "--max-requests-per-tick", "1"])
        .arg("--service-host-control-stdio")
        .args(["--live-event-timeout-ms", LIVE_EVENT_TIMEOUT_MS])
        .arg("--live-ticket-out")
        .arg(&ticket)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::from(errors))
        .process_group(0)
        .spawn()?;
    let control = process::Control::new(
        child.stdin.take().ok_or("retained stdin pipe")?,
        child.stdout.take().ok_or("retained stdout pipe")?,
        output,
    );
    Ok(Node {
        index,
        state_root: node.path.clone(),
        signed_main: signed_main.to_owned(),
        issuer,
        child,
        log,
        control: Some(control),
    })
}

/// Wait until the node publishes its current hosting readiness observation.
pub fn await_ready(node: &mut Node) -> crate::producer::Result<String> {
    // Supervision uses the admitted deadline type; the harness never samples an ambient clock.
    let mut deadline =
        molten::fabric_time::SupervisionDeadline::after(std::time::Duration::from_secs(READINESS_SECONDS))
            .map_err(|error| format!("{error:?}"))?;
    loop {
        let text = std::fs::read_to_string(&node.log).unwrap_or_default();
        if let Some(status) = node.child.try_wait()? {
            return Err(format!("node {} exited before readiness: {status}\n{text}", node.index).into());
        }
        if text.contains(READY) {
            return Ok(text);
        }
        if deadline.is_expired().map_err(|error| format!("{error:?}"))? {
            return Err(format!("node {} readiness timed out\n{text}", node.index).into());
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}
