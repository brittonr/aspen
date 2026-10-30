//! Owned child processes with bounded output retention.
//!
//! Shared by the outside-in shell and the native system-extension recovery twin.

use std::io::Read;

const READ_CHUNK_BYTES: usize = 4096;

/// Output retained up to a byte limit; the rest is noted, not kept.
struct Capture {
    bytes: Vec<u8>,
    limit: usize,
    is_overflowed: bool,
}

impl Capture {
    fn append(&mut self, chunk: &[u8]) {
        let room = self.limit.saturating_sub(self.bytes.len());
        self.bytes.extend_from_slice(&chunk[..chunk.len().min(room)]);
        if chunk.len() > room {
            self.is_overflowed = true;
        }
    }
}

type SharedCapture = std::sync::Arc<std::sync::Mutex<Capture>>;

/// Spawn `command` with a null stdin and piped stdout/stderr, ready to be adopted.
pub(crate) fn spawn_piped(mut command: std::process::Command) -> std::io::Result<std::process::Child> {
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
}

/// One owned child process. Its output is retained up to the scenario budget.
pub(crate) struct Child {
    child: std::process::Child,
    capture: SharedCapture,
    readers: Vec<std::thread::JoinHandle<()>>,
    status: Option<std::process::ExitStatus>,
}

impl Child {
    /// Take ownership of a spawned child, retaining its output up to `limit` bytes.
    pub(crate) fn adopt(mut child: std::process::Child, limit: usize) -> Self {
        let capture = std::sync::Arc::new(std::sync::Mutex::new(Capture {
            bytes: Vec::with_capacity(limit.min(READ_CHUNK_BYTES)),
            limit,
            is_overflowed: false,
        }));
        let streams: [Option<Box<dyn Read + Send>>; 2] = [
            child.stdout.take().map(|stream| Box::new(stream) as Box<dyn Read + Send>),
            child.stderr.take().map(|stream| Box::new(stream) as Box<dyn Read + Send>),
        ];
        let readers = streams.into_iter().flatten().map(|stream| capture_stream(stream, &capture)).collect();
        Self {
            child,
            capture,
            readers,
            status: None,
        }
    }

    pub(crate) fn poll(&mut self) -> Option<std::process::ExitStatus> {
        if self.status.is_none() {
            self.status = self.child.try_wait().ok().flatten();
        }
        self.status
    }

    /// Join the output readers of an exited process, so its output is whole.
    pub(crate) fn drain(&mut self) {
        for reader in self.readers.drain(..) {
            let _join = reader.join();
        }
    }

    pub(crate) fn is_overflowed(&self) -> bool {
        self.capture.lock().map_or(true, |capture| capture.is_overflowed)
    }

    pub(crate) fn output(&self) -> String {
        self.capture
            .lock()
            .map(|capture| String::from_utf8_lossy(&capture.bytes).into_owned())
            .unwrap_or_default()
    }

    /// Kill and reap the process. A reaped process reports its exit status.
    pub(crate) fn stop(&mut self) -> bool {
        if self.poll().is_none() {
            let _kill = self.child.kill();
            self.status = self.child.wait().ok();
        }
        self.drain();
        self.status.is_some()
    }
}

fn capture_stream(mut stream: Box<dyn Read + Send>, capture: &SharedCapture) -> std::thread::JoinHandle<()> {
    let capture = std::sync::Arc::clone(capture);
    std::thread::spawn(move || {
        let mut chunk = [0_u8; READ_CHUNK_BYTES];
        while let Ok(read) = stream.read(&mut chunk) {
            if read == 0 {
                break;
            }
            if let Ok(mut capture) = capture.lock() {
                capture.append(&chunk[..read]);
            }
        }
    })
}
