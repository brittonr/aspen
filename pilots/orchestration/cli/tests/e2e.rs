// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual separate processes, Iroh, Redb, and Wasmtime. This is same-host live
//! integration, not evidence for WAN/NAT/HA or for arbitrary host effects.
use std::fs;
use std::net::UdpSocket;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use serde_json::json;
use serde_json::Value;
const BIN: &str = env!("CARGO_BIN_EXE_molten-orchestrator");
struct Server {
    child: Child,
    root: PathBuf,
}
impl Server {
    fn start(root: &Path) -> Self {
        let log = fs::OpenOptions::new().create(true).append(true).open(root.join("test.stderr")).unwrap();
        let child = Command::new(BIN)
            .args(["serve", "--root"])
            .arg(root)
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .unwrap();
        Self {
            child,
            root: root.into(),
        }
    }
    fn crash_restart(&mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
        let replacement = Self::start(&self.root);
        let old = std::mem::replace(self, replacement);
        drop(old);
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn text(p: &Path) -> String {
    p.to_str().unwrap().to_owned()
}
fn command(args: &[&str]) -> Value {
    let output = Command::new(BIN).args(args).output().unwrap();
    assert!(output.status.success(), "command {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}
fn address() -> String {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.local_addr().unwrap().to_string()
}
fn status(root: &str, target: &str, run: &str) -> Option<Value> {
    let output = Command::new(BIN).args(["status", "--root", root, "--target", target, "--run", run]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}
fn wait_phase(root: &str, target: &str, run: &str, phase: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if let Some(value) = status(root, target, run) {
            if value["phase"] == phase {
                return value;
            }
            if value["phase"] == "Failed" {
                panic!("run failed: {value}");
            }
        }
        assert!(Instant::now() < deadline, "phase {phase} not reached");
        thread::sleep(Duration::from_millis(100));
    }
}
#[test]
fn real_two_worker_dag_restart_idempotency_and_unenrolled_peer_denial() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path();
    let policy = text(&base.join("policy.json"));
    command(&["new-policy", "--out", &policy, "--allow-pure-components"]);
    let croot = text(&base.join("controller"));
    let uroot = text(&base.join("client"));
    let w1root = text(&base.join("worker1"));
    let w2root = text(&base.join("worker2"));
    let caddr = address();
    let mut w1addr = address();
    while w1addr == caddr {
        w1addr = address();
    }
    let mut w2addr = address();
    while w2addr == caddr || w2addr == w1addr {
        w2addr = address();
    }
    let c = command(&[
        "init",
        "--root",
        &croot,
        "--role",
        "controller",
        "--policy",
        &policy,
        "--bind",
        &caddr,
    ]);
    let cid = c["id"].as_str().unwrap();
    let u = command(&["init", "--root", &uroot, "--role", "client", "--policy", &policy]);
    let uid = u["id"].as_str().unwrap();
    let w1 = command(&[
        "init",
        "--root",
        &w1root,
        "--role",
        "worker",
        "--policy",
        &policy,
        "--bind",
        &w1addr,
        "--controller",
        cid,
    ]);
    let w1id = w1["id"].as_str().unwrap();
    let w2 = command(&[
        "init",
        "--root",
        &w2root,
        "--role",
        "worker",
        "--policy",
        &policy,
        "--bind",
        &w2addr,
        "--controller",
        cid,
    ]);
    let w2id = w2["id"].as_str().unwrap();
    command(&["allow-operator", "--root", &croot, "--peer", uid]);
    command(&["add-worker", "--root", &croot, "--peer", w1id, "--address", &w1addr]);
    command(&["add-worker", "--root", &croot, "--peer", w2id, "--address", &w2addr]);
    // Explicit operator labels force actual work on both workers.
    let path = Path::new(&croot).join("config.json");
    let mut config: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    config["workers"][0]["labels"] = json!({"lane":"one"});
    config["workers"][1]["labels"] = json!({"lane":"two"});
    fs::write(path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
    let mut worker1 = Server::start(Path::new(&w1root));
    let _worker2 = Server::start(Path::new(&w2root));
    let mut controller = Server::start(Path::new(&croot));
    let target = format!("{cid}@{caddr}");
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let output = Command::new(BIN).args(["status", "--root", &uroot, "--target", &target]).output().unwrap();
        if output.status.success() {
            break;
        }
        assert!(Instant::now() < deadline, "controller startup");
        thread::sleep(Duration::from_millis(100));
    }
    let component = text(&base.join("constant.wat"));
    fs::write(&component, include_bytes!("../../examples/constant.wat")).unwrap();
    let put = command(&[
        "put",
        "--root",
        &uroot,
        "--target",
        &target,
        "--kind",
        "component",
        "--file",
        &component,
    ]);
    let artifact = put["id"].as_str().unwrap();
    let task = |name: &str, lane: &str, deps: Vec<&str>| json!({"id":name,"component":artifact,"inputs":[],"dependencies":deps,"resources":{"cpu_millis":1000,"memory_bytes":16777216,"slots":1},"required_labels":{"lane":lane},"fuel":1000000,"timeout_ms":5000,"max_output_bytes":1024,"max_attempts":2,"retry_delay_ticks":1});
    let spec = json!({"name":"live-dag","tasks":[task("a","one",vec![]),task("b","two",vec![]),task("c","one",vec!["a","b"])]});
    let specpath = text(&base.join("job.json"));
    fs::write(&specpath, serde_json::to_vec_pretty(&spec).unwrap()).unwrap();
    let submitted = command(&[
        "submit", "--root", &uroot, "--target", &target, "--key", "live-dag", "--file", &specpath,
    ]);
    let run = submitted["run"].as_str().unwrap();
    let done = wait_phase(&uroot, &target, run, "Succeeded");
    let output_id = done["run"]["tasks"]["c"]["phase"]["Succeeded"]["output"].as_str().unwrap();
    let outfile = text(&base.join("output.bin"));
    command(&[
        "get", "--root", &uroot, "--target", &target, "--id", output_id, "--out", &outfile,
    ]);
    assert_eq!(fs::read(outfile).unwrap(), b"ok");
    let raw = command(&["status", "--root", &uroot, "--target", &target, "--raw"]);
    assert_eq!(raw["attempts"].as_object().unwrap().len(), 3);
    let nodes = raw["attempts"]
        .as_object()
        .unwrap()
        .values()
        .map(|a| a["request"]["fence"]["node"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(nodes.contains(w1id) && nodes.contains(w2id));
    let duplicate = command(&[
        "submit", "--root", &uroot, "--target", &target, "--key", "live-dag", "--file", &specpath,
    ]);
    assert_eq!(duplicate["run"], run);
    controller.crash_restart();
    worker1.crash_restart();
    wait_phase(&uroot, &target, run, "Succeeded");
    let raw = command(&["status", "--root", &uroot, "--target", &target, "--raw"]);
    assert_eq!(raw["attempts"].as_object().unwrap().len(), 3);
    let outsider = text(&base.join("outsider"));
    command(&["init", "--root", &outsider, "--role", "client", "--policy", &policy]);
    let denied = Command::new(BIN).args(["status", "--root", &outsider, "--target", &target]).output().unwrap();
    assert!(!denied.status.success());
    // With both nodes drained the cancellation is deterministic and cannot race
    // an already completed fast constant component.
    command(&["drain", "--root", &uroot, "--target", &target, "--node", w1id]);
    command(&["drain", "--root", &uroot, "--target", &target, "--node", w2id]);
    let cancelled = command(&[
        "submit",
        "--root",
        &uroot,
        "--target",
        &target,
        "--key",
        "cancelled",
        "--file",
        &specpath,
    ]);
    let cancel_run = cancelled["run"].as_str().unwrap();
    command(&["cancel", "--root", &uroot, "--target", &target, "--run", cancel_run]);
    wait_phase(&uroot, &target, cancel_run, "Cancelled");
}
