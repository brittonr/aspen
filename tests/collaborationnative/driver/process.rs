//! The signed node process surface: bootstrap CLI and serial retained-owner control.

/// The process-facing facts of one signed node, with explicit retained ownership after startup.
pub struct Host<'a> {
    pub signed_main: &'a std::path::Path,
    pub state_root: &'a std::path::Path,
    pub issuer: &'a crate::authority::Issuer,
    pub index: u64,
    pub control: Option<&'a Control>,
}

pub fn host_of(node: &super::Node) -> Host<'_> {
    Host {
        signed_main: node.signed_main.as_path(),
        state_root: node.state_root.as_path(),
        issuer: &node.issuer,
        index: node.index,
        control: node.control.as_ref(),
    }
}

const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
const CONTROL_TIMEOUT_SECONDS: u64 = 43_200;

struct Session {
    stdin: Option<std::process::ChildStdin>,
    responses: std::sync::mpsc::Receiver<Result<String, String>>,
    is_failed: bool,
}

/// One serial command stream and its bounded, owned stdout spooler.
pub struct Control {
    session: std::sync::Mutex<Session>,
    reader: Option<std::thread::JoinHandle<()>>,
}

impl Control {
    pub fn new(stdin: std::process::ChildStdin, stdout: std::process::ChildStdout, log: std::fs::File) -> Self {
        let (sender, responses) = std::sync::mpsc::sync_channel(1);
        let reader = std::thread::spawn(move || spool(stdout, log, sender));
        Self {
            session: std::sync::Mutex::new(Session {
                stdin: Some(stdin),
                responses,
                is_failed: false,
            }),
            reader: Some(reader),
        }
    }

    pub fn close(&mut self) {
        if let Ok(session) = self.session.get_mut() {
            session.stdin.take();
        }
    }

    pub fn join(&mut self) {
        if let Some(reader) = self.reader.take() {
            let _joined = reader.join();
        }
    }

    fn invoke(
        &self,
        command: &str,
        tick: u64,
        mode: &str,
        operation: preserves::IOValue,
    ) -> crate::producer::Result<(bool, String)> {
        use std::io::Write;
        let request = molten::preserves_rail::record("node-service-host-control-v1", vec![
            molten::preserves_rail::string(mode),
            molten::preserves_rail::string(command),
            molten::preserves_rail::u64_value(tick),
            operation,
        ]);
        let text = molten::preserves_rail::to_text(&request)?;
        if text.len() > MAX_FRAME_BYTES {
            return Err("retained control request exceeds frame bound".into());
        }
        let mut session = self.session.lock().map_err(|_| "retained control mutex poisoned")?;
        if session.is_failed {
            return Err("retained control failed; operation outcome may be uncertain".into());
        }
        // Any failed exchange poisons the stream: never consume a late response as a new result.
        session.is_failed = true;
        let stdin = session.stdin.as_mut().ok_or("retained control stdin closed")?;
        stdin.write_all(text.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        let response = session
            .responses
            .recv_timeout(std::time::Duration::from_secs(CONTROL_TIMEOUT_SECONDS))?
            .map_err(|error| format!("retained control reader: {error}"))?;
        let outcome = response_value(&response, command, tick, mode)?;
        session.is_failed = false;
        Ok(outcome)
    }
}

fn response_value(response: &str, command: &str, tick: u64, mode: &str) -> crate::producer::Result<(bool, String)> {
    let parsed = molten::preserves_rail::parse_text(response)?;
    if response.trim_start().starts_with("<node-service-host-control-error-v1") {
        let fields = molten::preserves_rail::simple_record_fields(&parsed, "node-service-host-control-error-v1", 3)?;
        if super::operations::wire::text(&fields[0])? != command || super::operations::wire::number(&fields[1])? != tick
        {
            return Err("retained control error identity mismatch".into());
        }
        return Ok((false, super::operations::wire::text(&fields[2])?));
    }
    let fields = molten::preserves_rail::simple_record_fields(&parsed, "node-service-host-control-result-v1", 4)?;
    if super::operations::wire::text(&fields[0])? != mode
        || super::operations::wire::text(&fields[1])? != command
        || super::operations::wire::number(&fields[2])? != tick
    {
        return Err("retained control result identity mismatch".into());
    }
    Ok((true, molten::preserves_rail::to_text(&molten::preserves_rail::value_to_iovalue(&fields[3]))?))
}

fn spool(
    stdout: std::process::ChildStdout,
    mut log: std::fs::File,
    sender: std::sync::mpsc::SyncSender<Result<String, String>>,
) {
    use std::io::Write;
    let mut reader = std::io::BufReader::new(stdout);
    loop {
        let frame = match read_frame(&mut reader) {
            Ok(Some(frame)) => frame,
            Ok(None) => break,
            Err(error) => {
                let _sent = sender.try_send(Err(error.to_string()));
                break;
            }
        };
        if let Err(error) = log.write_all(&frame).and_then(|()| log.flush()) {
            let _sent = sender.try_send(Err(error.to_string()));
            break;
        }
        let text = match String::from_utf8(frame) {
            Ok(text) => text,
            Err(error) => {
                let _sent = sender.try_send(Err(error.to_string()));
                break;
            }
        };
        if text.trim_start().starts_with("<node-service-host-control-") && sender.try_send(Ok(text)).is_err() {
            break;
        }
    }
}

fn read_frame(reader: &mut impl std::io::BufRead) -> std::io::Result<Option<Vec<u8>>> {
    let mut frame = Vec::new();
    loop {
        let bytes = reader.fill_buf()?;
        if bytes.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unterminated stdout frame"))
            };
        }
        let newline = bytes.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(bytes.len(), |at| at + 1);
        if frame.len() + count > MAX_FRAME_BYTES + 1 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "stdout frame exceeds 16 MiB"));
        }
        frame.extend_from_slice(&bytes[..count]);
        reader.consume(count);
        if newline.is_some() {
            return Ok(Some(frame));
        }
    }
}

pub(super) fn invoke(
    host: &Host<'_>,
    command: &str,
    tick: u64,
    mode: Option<&str>,
) -> crate::producer::Result<(bool, String)> {
    if let Some(control) = host.control {
        let mode = match mode {
            Some("--preview") => "preview",
            None => "execute",
            _ => return Err("retained owner accepts only actual operation preview or execute".into()),
        };
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(host.state_root.join("operation.preserves"))?
            .take(1_048_577)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 1_048_576 {
            return Err("packed operation exceeds 1 MiB".into());
        }
        let operation = molten::preserves_rail::parse_canonical_bytes(&bytes)?;
        return control.invoke(command, tick, mode, operation);
    }
    let mut process = std::process::Command::new(host.signed_main);
    process
        .args(["node", "service-host", command, "--state-root"])
        .arg(host.state_root)
        .args(["--admission", crate::producer::ADMISSION, "--logical-tick"])
        .arg(tick.to_string())
        .args(["--input", "operation.preserves"]);
    if let Some(mode) = mode {
        process.arg(mode);
    }
    let output = process.output()?;
    Ok((output.status.success(), rendered(&output)))
}

pub fn rendered(output: &std::process::Output) -> String {
    format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))
}

pub(super) fn sign(
    host: &Host<'_>,
    fixture: &crate::producer::Node,
    request: &molten::node_service_hosting::OperationRequest,
    tick: u64,
    role: u64,
) -> crate::producer::Result<()> {
    let window = crate::authority::Window::around_now()?;
    let credential =
        host.issuer.host_credential(&fixture.bundle_ref, request, window, nonce(host.index, tick, role)?)?;
    host.issuer.write_host_credential(&fixture.root, &fixture.bundle_ref, request, &credential)?;
    Ok(())
}

/// The exact existing per-node, tick and role nonce used by actual host credentials.
pub(super) fn nonce(index: u64, tick: u64, role: u64) -> crate::producer::Result<[u8; 32]> {
    let mut nonce = [0u8; 32];
    nonce[..8].copy_from_slice(&(NONCE_START + tick * 4 + role).to_le_bytes());
    nonce[31] = u8::try_from(index).map_err(|_| "cohort position exceeds one byte")?;
    Ok(nonce)
}

const NONCE_START: u64 = 0x1000;

/// A closed owner must retain completion when a genuinely source-admitted process reopens it.
pub(super) fn reopened_status(
    node: &super::Node,
    fixture: &crate::producer::Node,
    tick: u64,
) -> crate::producer::Result<String> {
    let host = Host {
        signed_main: &node.signed_main,
        state_root: &node.state_root,
        issuer: &node.issuer,
        index: node.index,
        control: None,
    };
    let operation = super::operations::wire::operation("status", Vec::new());
    super::flow::authorize(&host, fixture, &operation, tick)?;
    let (success, output) = invoke(&host, "status", tick, None)?;
    if !success {
        return Err(format!("actual completion readback denied: {output}").into());
    }
    Ok(output)
}
