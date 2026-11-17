// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real separate processes; same host only. Tests are not execution evidence until run.
use std::cell::Cell;
use std::fs;
use std::net::UdpSocket;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use serde_json::json;
use serde_json::Value;
const BIN: &str = env!("CARGO_BIN_EXE_molten-orchestrator");
fn invoke(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().unwrap()
}
fn ok(args: &[&str]) -> Value {
    let out = invoke(args);
    assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}
fn text(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}
fn address() -> String {
    UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().to_string()
}
struct Server {
    child: Child,
    root: PathBuf,
}
impl Server {
    fn start(root: &Path, fault: Option<&str>) -> Self {
        let log = fs::OpenOptions::new().create(true).append(true).open(root.join("process.stderr")).unwrap();
        let mut command = Command::new(BIN);
        command.args(["serve", "--root"]).arg(root).stdout(Stdio::null()).stderr(log);
        if let Some(point) = fault {
            command.args(["--crash-at", point]);
        }
        Self {
            child: command.spawn().unwrap(),
            root: root.to_owned(),
        }
    }
    fn restart(&mut self, fault: Option<&str>) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let replacement = Self::start(&self.root, fault);
        let old = std::mem::replace(self, replacement);
        drop(old);
    }
    fn await_fault(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert_eq!(status.code(), Some(86));
                return;
            }
            assert!(Instant::now() < deadline, "fault boundary not reached");
            thread::sleep(Duration::from_millis(50));
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
struct Fixture {
    controller: Server,
    worker: Server,
    client: String,
    target: String,
    sequence: Cell<usize>,
    allow_submit_disconnect: bool,
    base: tempfile::TempDir,
}
impl Fixture {
    fn new(fault: Option<&str>) -> Self {
        let base = tempfile::tempdir().unwrap();
        let policy = text(&base.path().join("policy.json"));
        ok(&["new-policy", "--out", &policy, "--allow-pure-components"]);
        let croot = text(&base.path().join("controller"));
        let wroot = text(&base.path().join("worker"));
        let client = text(&base.path().join("client"));
        let ca = address();
        let mut wa = address();
        while wa == ca {
            wa = address();
        }
        let c = ok(&[
            "init",
            "--root",
            &croot,
            "--role",
            "controller",
            "--policy",
            &policy,
            "--bind",
            &ca,
        ]);
        let u = ok(&["init", "--root", &client, "--role", "client", "--policy", &policy]);
        let cid = c["id"].as_str().unwrap();
        let w = ok(&[
            "init",
            "--root",
            &wroot,
            "--role",
            "worker",
            "--policy",
            &policy,
            "--bind",
            &wa,
            "--controller",
            cid,
        ]);
        ok(&["allow-operator", "--root", &croot, "--peer", u["id"].as_str().unwrap()]);
        ok(&[
            "add-worker",
            "--root",
            &croot,
            "--peer",
            w["id"].as_str().unwrap(),
            "--address",
            &wa,
        ]);
        let worker = Server::start(Path::new(&wroot), None);
        let controller = Server::start(Path::new(&croot), fault);
        let fixture = Self {
            controller,
            worker,
            client,
            target: format!("{cid}@{ca}"),
            sequence: Cell::new(0),
            allow_submit_disconnect: fault.is_some(),
            base,
        };
        fixture.ready();
        fixture
    }
    fn ready(&self) {
        let deadline = Instant::now() + Duration::from_secs(45);
        while !invoke(&["status", "--root", &self.client, "--target", &self.target]).status.success() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(50));
        }
    }
    fn call(&self, command: &str, extra: &[&str]) -> Value {
        let mut args = vec![command, "--root", &self.client, "--target", &self.target];
        args.extend_from_slice(extra);
        let value = ok(&args);
        if command == "progress" && self.sequence.get() < 128 {
            let n = self.sequence.get();
            self.sequence.set(n + 1);
            let encoded = molten_orchestrator_codec::encode("batch-progress.v1", &value).unwrap();
            fs::write(self.base.path().join(format!("progress-{n:04}.preserves")), encoded.canonical).unwrap();
            fs::write(self.base.path().join(format!("progress-{n:04}.json")), encoded.projection).unwrap();
        }
        if command == "status" && extra.contains(&"--raw") {
            let encoded = molten_orchestrator_codec::encode("controller-state.v2", &value).unwrap();
            fs::write(self.base.path().join("controller-state.preserves"), encoded.canonical).unwrap();
            fs::write(self.base.path().join("controller-state.json"), encoded.projection).unwrap();
        }
        value
    }
    fn put(&self, name: &str, bytes: &[u8]) -> String {
        let file = self.base.path().join(name);
        fs::write(&file, bytes).unwrap();
        self.call("put", &["--kind", "component", "--file", &text(&file)])["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    fn submit(&self, key: &str, tasks: Vec<Value>) -> (String, PathBuf) {
        let path = self.base.path().join(format!("{key}.json"));
        fs::write(&path, serde_json::to_vec_pretty(&json!({"name":key,"tasks":tasks})).unwrap()).unwrap();
        let out = invoke(&[
            "submit",
            "--root",
            &self.client,
            "--target",
            &self.target,
            "--key",
            key,
            "--file",
            &text(&path),
        ]);
        let policy: molten_batch_core::Policy =
            serde_json::from_slice(&fs::read(self.base.path().join("policy.json")).unwrap()).unwrap();
        let expected = molten_batch_application::IdentityPort::run(
            &molten_orchestrator_adapters::CanonicalIdentity,
            policy.cluster,
            key,
        )
        .unwrap()
        .hex();
        if out.status.success() {
            let reply: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(reply["run"], expected);
        } else {
            assert!(
                self.allow_submit_disconnect,
                "unexpected submission failure: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        (expected, path)
    }
    fn wait(&self, run_id: &str, phase: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(120);
        let mut last_status = None;
        loop {
            let out = invoke(&[
                "status",
                "--root",
                &self.client,
                "--target",
                &self.target,
                "--run",
                run_id,
            ]);
            if out.status.success() {
                let value: Value = serde_json::from_slice(&out.stdout).unwrap();
                if value["phase"] == phase {
                    return value;
                }
                let terminal = matches!(value["phase"].as_str(), Some("Succeeded" | "Failed" | "Cancelled"));
                last_status = Some(value);
                if terminal {
                    self.retain_wait_failure(run_id, phase, last_status.as_ref(), &out, "different terminal phase");
                }
            }
            if Instant::now() >= deadline {
                self.retain_wait_failure(run_id, phase, last_status.as_ref(), &out, "observation deadline elapsed");
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
    fn retain_wait_failure(&self, run_id: &str, phase: &str, last: Option<&Value>, rpc: &Output, reason: &str) -> ! {
        let retain = |name: &str, bytes: &[u8]| {
            let bound = molten_batch_core::MAX_CONTENT_BYTES;
            fs::write(self.base.path().join(name), &bytes[..bytes.len().min(bound)]).unwrap();
            if bytes.len() > bound {
                fs::write(
                    self.base.path().join(format!("{name}.truncated")),
                    format!("retained_bytes={bound}\noriginal_bytes={}\n", bytes.len()),
                )
                .unwrap();
            }
        };
        if let Some(value) = last {
            retain("wait-last-selected-status.json", &serde_json::to_vec_pretty(value).unwrap());
        }
        retain("wait-last-status.stdout", &rpc.stdout);
        retain("wait-last-status.stderr", &rpc.stderr);
        // One bounded observation of each owner after failure, not an extension
        // of the acceptance deadline or a retry of the failed workload.
        let progress = invoke(&[
            "progress",
            "--root",
            &self.client,
            "--target",
            &self.target,
            "--run",
            run_id,
        ]);
        retain("wait-progress.stdout", &progress.stdout);
        retain("wait-progress.stderr", &progress.stderr);
        if progress.status.success() && progress.stdout.len() <= molten_batch_core::MAX_CONTENT_BYTES {
            let projection: Value = serde_json::from_slice(&progress.stdout).unwrap();
            let encoded = molten_orchestrator_codec::encode("batch-progress.v1", &projection).unwrap();
            retain("wait-progress.preserves", &encoded.canonical);
        }
        let raw = invoke(&["status", "--root", &self.client, "--target", &self.target, "--raw"]);
        retain("wait-controller-state.stdout", &raw.stdout);
        retain("wait-controller-state.stderr", &raw.stderr);
        if raw.status.success() && raw.stdout.len() <= molten_batch_core::MAX_CONTENT_BYTES {
            let state: Value = serde_json::from_slice(&raw.stdout).unwrap();
            let encoded = molten_orchestrator_codec::encode("controller-state.v2", &state).unwrap();
            retain("controller-state.preserves", &encoded.canonical);
            retain("controller-state.json", &encoded.projection);
        }
        let mut counts = std::collections::BTreeMap::<String, usize>::new();
        if let Some(tasks) = last.and_then(|status| status["run"]["tasks"].as_object()) {
            for task in tasks.values() {
                let phase = &task["phase"];
                let name = phase
                    .as_str()
                    .or_else(|| phase.as_object().and_then(|m| m.keys().next().map(String::as_str)))
                    .unwrap_or("Unknown");
                *counts.entry(name.to_owned()).or_default() += 1;
            }
        }
        let observed = last.map(|status| &status["phase"]);
        panic!(
            "run {run_id} did not reach {phase}: {reason}; last phase={observed:?}; task phases={counts:?}; retained wait-* and controller-state diagnostics"
        );
    }
    fn page(&self, cursor: &Value) -> Value {
        let path = self.base.path().join("cursor.json");
        fs::write(&path, serde_json::to_vec(cursor).unwrap()).unwrap();
        self.call("progress", &["--cursor", &text(&path), "--limit", "64"])
    }
}
// r[impl molten.batch_render.evidence]
impl Drop for Fixture {
    fn drop(&mut self) {
        // Copy only explicitly bounded public observations/artifacts. Never keys,
        // controller/worker databases, or private deployment directories.
        let Some(out) = std::env::var_os("MOLTEN_BATCH_EVIDENCE_DIR") else {
            return;
        };
        let destination = PathBuf::from(out).join(self.base.path().file_name().unwrap());
        fs::create_dir(&destination).unwrap();
        let mut count = 0;
        for entry in fs::read_dir(self.base.path()).unwrap() {
            count += 1;
            assert!(count <= 1024);
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() && entry.metadata().unwrap().len() <= 4 * 1024 * 1024 {
                fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
            }
        }
        for (name, root) in [
            ("controller.stderr", &self.controller.root),
            ("worker.stderr", &self.worker.root),
        ] {
            fs::copy(root.join("process.stderr"), destination.join(name)).unwrap();
        }
    }
}
fn task(id: &str, component: &str) -> Value {
    json!({"id":id,"workload":{"Component":{"artifact":component}},"inputs":[],"dependencies":[],
        "resources":{"cpu_millis":1000,"memory_bytes":16777216,"slots":1},"required_labels":{},
        "fuel":1000000,"timeout_ms":5000,"max_output_bytes":1024,"max_attempts":2,"retry_delay_ticks":1})
}
fn start_cursor(snapshot: &Value) -> Value {
    let mut start = snapshot["watermark"].clone();
    start["revision"] = json!(0);
    start["ordinal"] = json!(0);
    start
}
#[test]
fn progress_output_and_terminal_crash_boundaries_recover() {
    for point in [
        "before-dispatch",
        "after-dispatch",
        "before-output-persist",
        "after-output-persist",
        "before-terminal-commit",
        "after-terminal-commit",
        "before-progress-commit",
        "after-progress-commit",
    ] {
        let mut f = Fixture::new(Some(point));
        let component = f.put("constant.wat", include_bytes!("../../examples/constant.wat"));
        let (run_id, spec) = f.submit("boundary", vec![task("tile", &component)]);
        f.controller.await_fault();
        f.controller.restart(None);
        f.ready();
        let done = f.wait(&run_id, "Succeeded");
        assert_eq!(f.call("submit", &["--key", "boundary", "--file", &text(&spec)])["run"], run_id);
        let snapshot = f.call("progress", &["--run", &run_id]);
        let output = snapshot["tasks"]["tile"]["Accepted"]["output"].as_str().unwrap();
        assert_eq!(done["run"]["tasks"]["tile"]["phase"]["Succeeded"]["output"], output);
        let path = f.base.path().join("accepted.bin");
        f.call("get", &["--id", output, "--out", &text(&path)]);
        assert_eq!(fs::read(path).unwrap(), b"ok");
        let page = f.page(&start_cursor(&snapshot));
        assert_eq!(page, f.page(&start_cursor(&snapshot)));
        let events = page["Events"]["events"].as_array().unwrap();
        assert_eq!(events.iter().filter(|e| e["payload"].get("AcceptedOutput").is_some()).count(), 1);
        assert_eq!(events.iter().filter(|e| e["payload"].get("RunTerminal").is_some()).count(), 1);
        let ids = events.iter().map(|e| e["event_ref"].as_str().unwrap()).collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), events.len());
    }
}
// r[verify molten.observations.stream]
// r[verify molten.observations.cursors]
#[test]
fn progress_disconnect_lost_reply_and_reconnect_preserve_event_identity() {
    let mut f = Fixture::new(None);
    let component = f.put("constant.wat", include_bytes!("../../examples/constant.wat"));
    let (run_id, _) = f.submit("resume", vec![task("tile", &component)]);
    f.wait(&run_id, "Succeeded");
    let snapshot = f.call("progress", &["--run", &run_id]);
    let start = start_cursor(&snapshot);
    let before = f.page(&start);
    f.controller.restart(Some("before-progress-reply"));
    f.ready();
    let cursor = f.base.path().join("cursor.json");
    fs::write(&cursor, serde_json::to_vec(&start).unwrap()).unwrap();
    let lost = invoke(&[
        "progress",
        "--root",
        &f.client,
        "--target",
        &f.target,
        "--cursor",
        &text(&cursor),
    ]);
    assert!(!lost.status.success());
    f.controller.await_fault();
    f.controller.restart(None);
    f.ready();
    assert_eq!(before, f.page(&start));
    let recovered = f.call("progress", &["--run", &run_id]);
    assert_eq!(snapshot["tasks"], recovered["tasks"]);
    assert_eq!(snapshot["watermark"], recovered["watermark"]);
    assert!(f.page(&snapshot["watermark"])["Events"]["events"].as_array().unwrap().is_empty());
    let mut stale = start.clone();
    stale["stream_epoch"] = json!(999);
    assert!(f.page(&stale).get("StaleEpoch").is_some());
    let mut foreign = start;
    foreign["service_id"] = json!("not-this-service");
    assert_eq!(f.page(&foreign), json!("ForeignService"));
    let unavailable = invoke(&[
        "get",
        "--root",
        &f.client,
        "--target",
        &f.target,
        "--id",
        &"ff".repeat(32),
        "--out",
        &text(&f.base.path().join("missing.bin")),
    ]);
    assert!(!unavailable.status.success());
    assert!(!f.base.path().join("missing.bin").exists(), "missing content must not publish an output artifact");
    let outsider = text(&f.base.path().join("outsider"));
    ok(&[
        "init",
        "--root",
        &outsider,
        "--role",
        "client",
        "--policy",
        &text(&f.base.path().join("policy.json")),
    ]);
    assert!(!invoke(&["progress", "--root", &outsider, "--target", &f.target, "--run", &run_id]).status.success());
}
#[test]
fn active_worker_loss_retries_only_after_stopped_recovery_and_preserves_accepted_tiles() {
    let mut f = Fixture::new(None);
    let component = f.put("constant.wat", include_bytes!("../../examples/constant.wat"));
    let spin = f.put("spin.wat", include_bytes!("../../examples/spin.wat"));
    let mut slow = task("b", &spin);
    slow["dependencies"] = json!(["a"]);
    slow["fuel"] = json!(1000000000_u64);
    slow["timeout_ms"] = json!(60000);
    let (run_id, _) = f.submit("worker-loss", vec![task("a", &component), slow]);
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        let snapshot = f.call("progress", &["--run", &run_id]);
        if snapshot["tasks"]["b"].get("Running").is_some() {
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(50));
    }
    f.worker.restart(None);
    f.wait(&run_id, "Failed");
    let snapshot = f.call("progress", &["--run", &run_id]);
    assert!(snapshot["tasks"]["a"].get("Accepted").is_some());
    let raw = f.call("status", &["--raw"]);
    let attempts = raw["attempts"]
        .as_object()
        .unwrap()
        .values()
        .filter(|a| a["request"]["fence"]["task"] == "b")
        .collect::<Vec<_>>();
    assert_eq!(attempts.len(), 2);
    assert!(attempts.iter().all(|a| a["reservation_held"] == false));
    assert!(attempts.iter().all(|a| a["phase"]["Terminal"]["result"]["stopped"] == true));
}
#[test]
fn exhaustion_and_running_cancellation_have_authoritative_terminal_observations() {
    let f = Fixture::new(None);
    let constant = f.put("constant.wat", include_bytes!("../../examples/constant.wat"));
    let spin = f.put("spin.wat", include_bytes!("../../examples/spin.wat"));
    for (key, fuel, timeout, output, component) in [
        ("fuel", 100_u64, 5000_u64, 1024_u64, &spin),
        ("deadline", 1000000000, 1, 1024, &spin),
        ("output", 1000000, 5000, 1, &constant),
    ] {
        let mut t = task("tile", component);
        t["fuel"] = json!(fuel);
        t["timeout_ms"] = json!(timeout);
        t["max_output_bytes"] = json!(output);
        t["max_attempts"] = json!(1);
        let (id, _) = f.submit(key, vec![t]);
        f.wait(&id, "Failed");
        assert!(f.call("progress", &["--run", &id])["tasks"]["tile"].get("Failed").is_some());
        let raw = f.call("status", &["--raw"]);
        let attempt =
            raw["attempts"].as_object().unwrap().values().find(|a| a["request"]["fence"]["run"] == id).unwrap();
        let code = &attempt["phase"]["Terminal"]["result"]["outcome"]["Failed"]["code"];
        if key == "output" {
            assert_eq!(code, "OutputLimit");
        } else {
            assert!(code == "Trap" || code == "FuelOrDeadline");
        }
    }
    let mut slow = task("tile", &spin);
    slow["fuel"] = json!(1000000000_u64);
    slow["timeout_ms"] = json!(60000);
    let (id, _) = f.submit("cancel-running", vec![slow]);
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        let snapshot = f.call("progress", &["--run", &id]);
        if snapshot["tasks"]["tile"].get("Running").is_some() {
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(50));
    }
    f.call("cancel", &["--run", &id]);
    f.wait(&id, "Cancelled");
    let raw = f.call("status", &["--raw"]);
    assert!(raw["attempts"]
        .as_object()
        .unwrap()
        .values()
        .filter(|a| a["request"]["fence"]["run"] == id)
        .all(|a| a["reservation_held"] == false && a["phase"]["Terminal"]["result"]["stopped"] == true));
}
// r[verify molten.observations.cursors]
#[test]
fn progress_retention_overflow_requires_a_new_consistent_snapshot() {
    let f = Fixture::new(None);
    // Exercise event eviction, not max-size job throughput. Keep the existing
    // event capacity and per-run deadline while filling it with bounded jobs.
    let tasks_per_run = 16;
    let evicting_runs = molten_batch_core::MAX_BATCH_PROGRESS_EVENTS / (tasks_per_run + 1) + 1;
    let component = f.put("constant.wat", include_bytes!("../../examples/constant.wat"));
    let (first, _) =
        f.submit("retention-00", (0..tasks_per_run).map(|n| task(&format!("tile-{n:02}"), &component)).collect());
    f.wait(&first, "Succeeded");
    let snapshot = f.call("progress", &["--run", &first]);
    let cursor = start_cursor(&snapshot);
    let mut is_evicted = false;
    for n in 1..=evicting_runs {
        let (id, _) = f.submit(
            &format!("retention-{n:02}"),
            (0..tasks_per_run).map(|t| task(&format!("tile-{t:02}"), &component)).collect(),
        );
        f.wait(&id, "Succeeded");
        if f.page(&cursor).get("ResnapshotRequired").is_some() {
            is_evicted = true;
            break;
        }
    }
    assert!(is_evicted, "bounded completed jobs did not evict the saved cursor");
    let fresh = f.call("progress", &["--run", &first]);
    assert!(fresh["tasks"].as_object().unwrap().values().all(|t| t.get("Accepted").is_some()));
    assert!(f.page(&fresh["watermark"])["Events"]["events"].as_array().unwrap().is_empty());
}

fn renderer_job(f: &Fixture, key: &str) -> (String, molten_render_pilot::Manifest, PathBuf) {
    let path = PathBuf::from(
        std::env::var_os("MOLTEN_RENDER_COMPONENT").expect("set MOLTEN_RENDER_COMPONENT using render-faults.sh"),
    );
    assert!(fs::metadata(&path).unwrap().len() <= molten_batch_core::MAX_CONTENT_BYTES as u64);
    let component = f.put("render.component.wasm", &fs::read(path).unwrap());
    let manifest = molten_render_pilot::frozen_manifest(molten_batch_core::Id::parse(&component).unwrap()).unwrap();
    let prepared = molten_render_pilot::prepare(manifest.clone()).unwrap();
    fs::write(f.base.path().join("render-manifest.json"), serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    fs::write(f.base.path().join("render-manifest.frame"), manifest.encode().unwrap()).unwrap();
    for (id, bytes) in prepared.objects {
        let file = f.base.path().join(format!("{id}.input"));
        fs::write(&file, bytes).unwrap();
        assert_eq!(f.call("put", &["--kind", "data", "--file", &text(&file)])["id"], id.hex());
    }
    let tasks = prepared.job.tasks.iter().map(|task| serde_json::to_value(task).unwrap()).collect();
    let (run_id, spec) = f.submit(key, tasks);
    (run_id, manifest, spec)
}
fn verify_render(f: &Fixture, run_id: &str, manifest: &molten_render_pilot::Manifest) -> Value {
    f.wait(run_id, "Succeeded");
    let snapshot = f.call("progress", &["--run", run_id]);
    let output = snapshot["tasks"]["assemble"]["Accepted"]["output"].as_str().unwrap();
    let path = f.base.path().join("render-final.bin");
    f.call("get", &["--id", output, "--out", &text(&path)]);
    let bytes = fs::read(path).unwrap();
    assert_eq!(molten_render_pilot::content_id(molten_batch_core::ContentKind::Data, &bytes).unwrap().hex(), output);
    assert_eq!(molten_render_pilot::verify_final(manifest, &bytes).unwrap().len(), 640 * 360 * 3);
    snapshot
}
// r[verify molten.batch_render.evidence]
#[test]
#[ignore = "requires the real built component; run scripts/render-faults.sh"]
fn real_renderer_recovers_every_controller_publication_boundary() {
    for point in [
        "before-dispatch",
        "after-dispatch",
        "before-output-persist",
        "after-output-persist",
        "before-terminal-commit",
        "after-terminal-commit",
        "before-progress-commit",
        "after-progress-commit",
    ] {
        let mut f = Fixture::new(Some(point));
        let (run_id, manifest, spec) = renderer_job(&f, "render-boundary");
        f.controller.await_fault();
        f.controller.restart(None);
        f.ready();
        assert_eq!(f.call("submit", &["--key", "render-boundary", "--file", &text(&spec)])["run"], run_id);
        let snapshot = verify_render(&f, &run_id, &manifest);
        let first = f.page(&start_cursor(&snapshot));
        assert_eq!(first, f.page(&start_cursor(&snapshot)));
        assert_eq!(
            first["Events"]["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|event| event["payload"].get("AcceptedOutput").is_some())
                .count(),
            33
        );
    }
}
#[test]
#[ignore = "requires the real built component; run scripts/render-faults.sh"]
fn real_renderer_active_worker_loss_and_controller_restart_keep_accepted_tiles() {
    let mut f = Fixture::new(None);
    let (run_id, manifest, _) = renderer_job(&f, "render-worker-loss");
    let deadline = Instant::now() + Duration::from_secs(120);
    let accepted = loop {
        let snapshot = f.call("progress", &["--run", &run_id]);
        let tasks = snapshot["tasks"].as_object().unwrap();
        let accepted = tasks
            .iter()
            .filter_map(|(name, task)| task.get("Accepted").map(|value| (name.clone(), value.clone())))
            .collect::<std::collections::BTreeMap<_, _>>();
        if !accepted.is_empty() && tasks.values().any(|task| task.get("Running").is_some()) {
            break accepted;
        }
        assert!(Instant::now() < deadline, "no active render batch after accepted work");
        thread::sleep(Duration::from_millis(50));
    };
    f.worker.restart(None);
    f.controller.restart(None);
    f.ready();
    let snapshot = verify_render(&f, &run_id, &manifest);
    for (name, value) in accepted {
        assert_eq!(snapshot["tasks"][&name]["Accepted"], value);
    }
    let raw = f.call("status", &["--raw"]);
    assert!(raw["attempts"].as_object().unwrap().values().any(|a| a["request"]["fence"]["epoch"] == 2));
    assert!(raw["attempts"].as_object().unwrap().values().all(|a| a["reservation_held"] == false));
}

#[test]
fn acknowledged_progress_reply_survives_controller_crash_without_deleting_history() {
    let mut f = Fixture::new(None);
    let component = f.put("constant.wat", include_bytes!("../../examples/constant.wat"));
    let (run_id, _) = f.submit("acknowledged", vec![task("tile", &component)]);
    f.wait(&run_id, "Succeeded");
    let snapshot = f.call("progress", &["--run", &run_id]);
    let cursor = start_cursor(&snapshot);
    let expected = f.page(&cursor);
    f.controller.restart(Some("after-progress-ack"));
    f.ready();
    assert_eq!(f.page(&cursor), expected);
    f.controller.await_fault();
    f.controller.restart(None);
    f.ready();
    assert_eq!(f.page(&cursor), expected);
}

#[test]
#[ignore = "requires the real built component; run scripts/render-faults.sh"]
fn real_renderer_pending_and_active_cancellation_do_not_confuse_disconnect_with_stop() {
    let f = Fixture::new(None);
    let config: Value = serde_json::from_slice(&fs::read(f.controller.root.join("config.json")).unwrap()).unwrap();
    let worker = config["workers"][0]["peer"]["id"].as_str().unwrap();
    f.call("drain", &["--node", worker]);
    let (pending, _, _) = renderer_job(&f, "render-cancel-pending");
    f.call("cancel", &["--run", &pending]);
    f.wait(&pending, "Cancelled");
    let snapshot = f.call("progress", &["--run", &pending]);
    assert!(snapshot["tasks"].as_object().unwrap().values().all(|task| task == "Cancelled"));
    f.call("drain", &["--node", worker, "--resume"]);
    let (active, _, _) = renderer_job(&f, "render-cancel-active");
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let snapshot = f.call("progress", &["--run", &active]);
        if snapshot["tasks"].as_object().unwrap().values().any(|task| task.get("Running").is_some()) {
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(50));
    }
    f.call("cancel", &["--run", &active]);
    f.wait(&active, "Cancelled");
    let raw = f.call("status", &["--raw"]);
    let attempts = raw["attempts"]
        .as_object()
        .unwrap()
        .values()
        .filter(|a| a["request"]["fence"]["run"] == active)
        .collect::<Vec<_>>();
    assert!(!attempts.is_empty());
    assert!(attempts
        .iter()
        .all(|a| a["reservation_held"] == false && a["phase"]["Terminal"]["result"]["stopped"] == true));
}
