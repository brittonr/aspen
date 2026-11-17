// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authored Linux CLI/service process qualification; no test is evidence until run.
//! Native calculation is trusted in-process work, not a sandbox. Same host only.
#![cfg(target_os = "linux")]
use std::fs;
use std::io::Read;
use std::net::UdpSocket;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Output;
use std::process::Stdio;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use molten_batch_application::IdentityPort;
use molten_batch_application::WorkerJournal;
use molten_batch_core::*;
use molten_orchestrator_adapters::*;
use molten_orchestrator_codec as codec;
use molten_pure_net as net;
use serde_json::json;
use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_molten-orchestrator");
const FUEL: u64 = 1_000_000_000;
const MEMORY: u64 = 64 * 1024 * 1024;
const OUTPUT: u64 = 262_144;
const ATTEMPTS: u32 = 2;
const DIAGNOSTIC_LIMIT: usize = 4 * 1024 * 1024;
static PROCESS_CAMPAIGN: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    PROCESS_CAMPAIGN.lock().unwrap_or_else(|p| p.into_inner())
}
fn text(path: &Path) -> String {
    path.to_str().expect("UTF-8 fixture path").to_owned()
}
fn invoke(args: &[&str]) -> Output {
    Command::new(BIN)
        .env("TOKIO_WORKER_THREADS", "2")
        .args(args)
        .output()
        .expect("run actual CLI binary")
}
fn ok(args: &[&str]) -> Value {
    let output = invoke(args);
    assert!(output.status.success(), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).expect("CLI JSON response")
}
fn port() -> String {
    UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().to_string()
}
fn elapsed_ns(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap()
}
fn capacity() -> Resources {
    Resources {
        cpu_millis: 1000,
        memory_bytes: MEMORY,
        slots: 1,
    }
}
fn bounded_file(path: &Path) -> Vec<u8> {
    let file = fs::File::open(path).unwrap();
    assert!(file.metadata().unwrap().len() <= DIAGNOSTIC_LIMIT as u64);
    let mut bytes = Vec::new();
    file.take(DIAGNOSTIC_LIMIT as u64 + 1).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() <= DIAGNOSTIC_LIMIT, "diagnostic retention exceeded bound: {}", path.display());
    bytes
}

struct Server {
    child: Option<Child>,
    root: PathBuf,
    logs: Vec<PathBuf>,
    vm_hwm_kib: u64,
    heaptrack_raws: Vec<PathBuf>,
}
impl Server {
    fn start(root: &Path, pause: Option<&str>) -> Self {
        let mut server = Self {
            child: None,
            root: root.to_owned(),
            logs: Vec::new(),
            vm_hwm_kib: 0,
            heaptrack_raws: Vec::new(),
        };
        server.launch(pause);
        server
    }
    fn launch(&mut self, pause: Option<&str>) {
        assert!(self.child.is_none(), "old process must be killed AND waited before restart");
        let log = self.root.join(format!("pure-net-process-{:02}.stderr", self.logs.len()));
        let stderr = fs::OpenOptions::new().write(true).create_new(true).open(&log).unwrap();
        let mut command = Command::new("prlimit");
        command.args(["--as=8589934592:8589934592", "--cpu=3600:3660", "--"]);
        if let Some(prefix) = std::env::var_os("MOLTEN_PURE_NET_HEAPTRACK_PREFIX") {
            let config: NodeConfig = serde_json::from_slice(&bounded_file(&self.root.join("config.json"))).unwrap();
            if config.role == Role::Worker {
                let directory = PathBuf::from(
                    std::env::var_os("MOLTEN_PURE_NET_ALLOCATIONS_DIR")
                        .expect("explicit private allocation-output directory is required"),
                );
                fs::create_dir_all(&directory).unwrap();
                let (file, raw) = tempfile::Builder::new()
                    .prefix("heaptrack-worker-")
                    .suffix(".raw")
                    .tempfile_in(directory)
                    .unwrap()
                    .keep()
                    .unwrap();
                drop(file);
                let library = PathBuf::from(prefix).join("lib/heaptrack/libheaptrack_preload.so");
                assert!(library.is_file(), "selected profiler preload must exist");
                // prlimit and env exec in-place: Child still owns the actual
                // worker PID, unlike a profiler shell supervising another child.
                command
                    .arg("env")
                    .arg(format!("LD_PRELOAD={}", text(&library)))
                    .arg(format!("DUMP_HEAPTRACK_OUTPUT={}", text(&raw)));
                self.heaptrack_raws.push(raw);
            }
        }
        command
            .args([BIN, "serve", "--root"])
            .arg(&self.root)
            .env("TOKIO_WORKER_THREADS", "2")
            .stdout(Stdio::null())
            .stderr(stderr);
        if let Some(point) = pause {
            command.args(["--pause-at", point]);
        }
        self.child = Some(command.spawn().expect("prlimit must be installed; no unbounded launch fallback"));
        self.logs.push(log);
    }
    fn sample_memory(&mut self) -> u64 {
        let child = self.child.as_mut().expect("live owned process");
        assert!(child.try_wait().unwrap().is_none(), "process exited before memory observation");
        let status =
            fs::read_to_string(format!("/proc/{}/status", child.id())).expect("owned Linux child /proc status");
        let hwm = status.lines().find_map(|line| line.strip_prefix("VmHWM:")).expect("Linux VmHWM observation");
        let mut fields = hwm.split_whitespace();
        let value: u64 = fields.next().unwrap().parse().unwrap();
        assert_eq!(fields.next(), Some("kB"));
        self.vm_hwm_kib = self.vm_hwm_kib.max(value);
        self.vm_hwm_kib
    }
    fn stderr(&self) -> String {
        self.logs
            .iter()
            .map(|path| String::from_utf8(bounded_file(path)).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn wait_pause(&mut self, point: &str) {
        let marker = format!("orchestrator paused-process-boundary point={point}");
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            let current = String::from_utf8(bounded_file(self.logs.last().unwrap())).unwrap();
            if current.lines().any(|line| line == marker) {
                self.sample_memory();
                return;
            }
            assert!(
                self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "worker exited before {point}: {current}"
            );
            assert!(Instant::now() < deadline, "owned worker did not reach {point}: {current}");
            thread::sleep(Duration::from_millis(25));
        }
    }
    fn resume(&self) {
        let pid = self.child.as_ref().expect("owned paused child").id().to_string();
        let status = Command::new("kill").args(["-USR1", &pid]).status().expect("signal tool available");
        assert!(status.success(), "SIGUSR1 must release the owned conformance pause");
    }
    fn stop(&mut self) -> Option<ExitStatus> {
        let mut child = self.child.take()?;
        // Child::kill is SIGKILL on Linux; wait confirms that this exact owner
        // cannot still calculate. Never infer process exit from a lost RPC.
        let _ = child.kill();
        Some(child.wait().expect("wait for exact owned child"))
    }
    fn stop_gracefully(&mut self) {
        let Some(child) = self.child.as_mut() else {
            return;
        };
        if child.try_wait().unwrap().is_none() {
            let status = Command::new("kill").args(["-TERM", &child.id().to_string()]).status().unwrap();
            if !status.success() {
                self.stop();
                panic!("failed to stop the exact owned profiling worker");
            }
        }
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                self.child.take();
                assert!(status.success(), "profiling requires graceful completion, not a truncated killed trace");
                return;
            }
            if Instant::now() >= deadline {
                self.stop();
                panic!("profiling worker did not stop within its bounded wait");
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

struct Prepared {
    manifest_file: PathBuf,
    program_file: PathBuf,
    input_file: PathBuf,
    manifest: net::Manifest,
    manifest_id: Id,
    program_id: Id,
    input_id: Id,
    program_semantics: Id,
    input_semantics: Id,
    preparation_ns: u64,
}
impl Prepared {
    fn new(
        base: &Path,
        backend: net::Backend,
        fixture: &net::fixtures::Fixture,
        limits: net::Limits,
        forge_ir: bool,
    ) -> Self {
        let start = Instant::now();
        let manifest_file = base.join("manifest.frame");
        let program_file = base.join("program.frame");
        let input_file = base.join("inputs.frame");
        let limits_file = base.join("limits.json");
        let source_file = base.join("source.noble");
        let values_file = base.join("ordered-inputs.json");
        fs::write(&limits_file, serde_json::to_vec_pretty(&limits).unwrap()).unwrap();
        fs::write(&source_file, fixture.source).unwrap();
        fs::write(&values_file, serde_json::to_vec(fixture.inputs).unwrap()).unwrap();
        let backend_name = match backend {
            net::Backend::SequentialNet => "sequential-net",
            net::Backend::IndependentReference => "independent-reference",
        };
        let manifest_reply = ok(&[
            "pure-net",
            "manifest",
            "--backend",
            backend_name,
            "--out",
            &text(&manifest_file),
            "--limits",
            &text(&limits_file),
        ]);
        let manifest_bytes = bounded_file(&manifest_file);
        let manifest = net::decode_manifest(&manifest_bytes).unwrap();
        assert_eq!(manifest.backend, backend);
        assert_eq!(manifest.limits, limits);
        assert_eq!(manifest_reply["profile"], serde_json::to_value(&manifest).unwrap());
        let manifest_id = net::content_id_manifest(&manifest_bytes).unwrap();
        assert_eq!(manifest_reply["manifest"], json!(manifest_id));
        // Only manifest bytes generated by the actual CLI enter installation;
        // this test image never generates or supplies native host bindings.
        let compile_reply = ok(&[
            "pure-net",
            "compile",
            "--source",
            &text(&source_file),
            "--input-arity",
            &fixture.inputs.len().to_string(),
            "--inputs",
            &text(&values_file),
            "--program-out",
            &text(&program_file),
            "--inputs-out",
            &text(&input_file),
        ]);
        let mut program_bytes = bounded_file(&program_file);
        assert_eq!(compile_reply["program"], json!(net::content_id_program(&program_bytes).unwrap()));
        let input_id = net::content_id_inputs(&bounded_file(&input_file)).unwrap();
        assert_eq!(compile_reply["input"], json!(input_id));
        assert_eq!(compile_reply["producer"], net::PRODUCER_REVISION);
        if forge_ir {
            let frame: Value = serde_json::from_slice(&program_bytes).unwrap();
            let mut body = frame["body"].clone();
            body["ir"] = json!(Id([0; 32]));
            let new_ref = codec::identity("pure-net-eval/program/v1", &body).unwrap();
            // Preserve the producer DTO's exact JSON field order. Framing a
            // generic Value would reorder its projection and test noncanonical
            // decoding instead of the intended source/IR identity mismatch.
            let original = String::from_utf8(program_bytes).unwrap();
            let old_ir = format!("\"ir\":{}", frame["body"]["ir"]);
            let old_ref = format!("\"value_ref\":{}", frame["value_ref"]);
            assert_eq!(original.matches(&old_ir).count(), 1);
            assert_eq!(original.matches(&old_ref).count(), 1);
            program_bytes = original
                .replacen(&old_ir, &format!("\"ir\":{}", json!(Id([0; 32]))), 1)
                .replacen(&old_ref, &format!("\"value_ref\":{}", json!(new_ref)), 1)
                .into_bytes();
            fs::write(&program_file, &program_bytes).unwrap();
        }
        let program_id = net::content_id_program(&program_bytes).unwrap();
        let program_frame: Value = serde_json::from_slice(&program_bytes).unwrap();
        let input_frame: Value = serde_json::from_slice(&bounded_file(&input_file)).unwrap();
        Self {
            manifest_file,
            program_file,
            input_file,
            manifest,
            manifest_id,
            program_id,
            input_id,
            program_semantics: serde_json::from_value(program_frame["value_ref"].clone()).unwrap(),
            input_semantics: serde_json::from_value(input_frame["value_ref"].clone()).unwrap(),
            preparation_ns: elapsed_ns(start),
        }
    }
}

struct Fixture {
    controller: Server,
    worker: Server,
    client: PathBuf,
    target: String,
    worker_target: PeerAddress,
    policy: Policy,
    policy_ref: Id,
    prepared: Prepared,
    base: tempfile::TempDir,
    evidence: Option<PathBuf>,
}
impl Fixture {
    fn new(
        label: &str,
        backend: net::Backend,
        source: &net::fixtures::Fixture,
        limits: net::Limits,
        pause: Option<&str>,
        grant_program: bool,
        forge_ir: bool,
    ) -> Self {
        let base = tempfile::tempdir().unwrap();
        let prepared = Prepared::new(base.path(), backend, source, limits, forge_ir);
        let policy_file = base.path().join("policy.json");
        let manifest_id = prepared.manifest_id.hex();
        let program_id = prepared.program_id.hex();
        let policy_path = text(&policy_file);
        let mut args = vec![
            "new-policy",
            "--out",
            policy_path.as_str(),
            "--pure-net-manifest",
            manifest_id.as_str(),
        ];
        if grant_program {
            args.extend(["--pure-net-program", program_id.as_str()]);
        }
        ok(&args);
        let policy: Policy = serde_json::from_slice(&bounded_file(&policy_file)).unwrap();
        let policy_ref = policy_id(&policy).unwrap();
        let controller_root = base.path().join("controller");
        let worker_root = base.path().join("worker");
        let client = base.path().join("client");
        let controller_address = port();
        let mut worker_address = port();
        while worker_address == controller_address {
            worker_address = port();
        }
        let controller = ok(&[
            "init",
            "--root",
            &text(&controller_root),
            "--role",
            "controller",
            "--policy",
            &policy_path,
            "--bind",
            &controller_address,
        ]);
        let client_identity = ok(&[
            "init",
            "--root",
            &text(&client),
            "--role",
            "client",
            "--policy",
            &policy_path,
        ]);
        let controller_id = controller["id"].as_str().unwrap();
        let worker = ok(&[
            "init",
            "--root",
            &text(&worker_root),
            "--role",
            "worker",
            "--policy",
            &policy_path,
            "--bind",
            &worker_address,
            "--controller",
            controller_id,
            "--cpu-millis",
            "1000",
            "--memory-bytes",
            &MEMORY.to_string(),
            "--slots",
            "1",
        ]);
        let worker_id = worker["id"].as_str().unwrap();
        ok(&[
            "allow-operator",
            "--root",
            &text(&controller_root),
            "--peer",
            client_identity["id"].as_str().unwrap(),
        ]);
        ok(&[
            "add-worker",
            "--root",
            &text(&controller_root),
            "--peer",
            worker_id,
            "--address",
            &worker_address,
            "--cpu-millis",
            "1000",
            "--memory-bytes",
            &MEMORY.to_string(),
            "--slots",
            "1",
        ]);
        for root in [&controller_root, &worker_root] {
            let installed = ok(&[
                "pure-net",
                "install",
                "--root",
                &text(root),
                "--manifest",
                &text(&prepared.manifest_file),
            ]);
            assert_eq!(installed["manifest"], json!(prepared.manifest_id));
            assert_eq!(installed["installed"], true);
            assert_eq!(
                bounded_file(&root.join("pure-net-manifest.preserves-frame")),
                bounded_file(&prepared.manifest_file)
            );
            let repeated = invoke(&[
                "pure-net",
                "install",
                "--root",
                &text(root),
                "--manifest",
                &text(&prepared.manifest_file),
            ]);
            assert!(
                !repeated.status.success(),
                "manifest installation is create-only, not default adoption/replacement"
            );
        }
        let evidence = std::env::var_os("MOLTEN_PURE_NET_EVIDENCE_DIR").map(|parent| {
            let parent = PathBuf::from(parent);
            fs::create_dir_all(&parent).unwrap();
            let leaf = format!("{label}-{}", base.path().file_name().unwrap().to_string_lossy());
            let path = parent.join(leaf);
            fs::create_dir(&path).expect("new diagnostic evidence directory, never overwrite");
            path
        });
        let mut fixture = Self {
            controller: Server::start(&controller_root, None),
            worker: Server::start(&worker_root, pause),
            client,
            target: format!("{controller_id}@{controller_address}"),
            worker_target: PeerAddress {
                id: Id::parse(worker_id).unwrap(),
                address: worker_address.parse().unwrap(),
            },
            policy,
            policy_ref,
            prepared,
            base,
            evidence,
        };
        fixture.ready();
        for (kind, path, expected) in [
            ("pure-net-manifest", &fixture.prepared.manifest_file, fixture.prepared.manifest_id),
            ("pure-net-program", &fixture.prepared.program_file, fixture.prepared.program_id),
            ("data", &fixture.prepared.input_file, fixture.prepared.input_id),
        ] {
            let put = fixture.call("put", &["--kind", kind, "--file", &text(path)]);
            assert_eq!(put["id"], json!(expected));
        }
        fixture.worker.sample_memory();
        fixture
    }
    fn ready(&self) {
        let deadline = Instant::now() + Duration::from_secs(45);
        loop {
            let output = self.invoke("status", &[]);
            if output.status.success() {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "service not ready: {}\n{}",
                String::from_utf8_lossy(&output.stderr),
                self.controller.stderr()
            );
            thread::sleep(Duration::from_millis(25));
        }
    }
    fn invoke(&self, command: &str, extra: &[&str]) -> Output {
        let client = text(&self.client);
        let mut args = vec![command, "--root", client.as_str(), "--target", self.target.as_str()];
        args.extend_from_slice(extra);
        invoke(&args)
    }
    fn call(&self, command: &str, extra: &[&str]) -> Value {
        let output = self.invoke(command, extra);
        assert!(
            output.status.success(),
            "{command} {extra:?}: {}\nworker={}\ncontroller={}",
            String::from_utf8_lossy(&output.stderr),
            self.worker.stderr(),
            self.controller.stderr()
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn job(&self, key: &str) -> PathBuf {
        let task = json!({"id":"eval","workload":{"PureNet":{"manifest":self.prepared.manifest_id,"artifact":self.prepared.program_id}},
            "inputs":[self.prepared.input_id],"dependencies":[],"required_labels":{},"resources":capacity(),
            "fuel":FUEL,"timeout_ms":60000,"max_output_bytes":OUTPUT,"max_attempts":ATTEMPTS,"retry_delay_ticks":1});
        let path = self.base.path().join(format!("{key}.job.json"));
        fs::write(&path, serde_json::to_vec_pretty(&json!({"name":key,"tasks":[task]})).unwrap()).unwrap();
        path
    }
    fn submit(&self, key: &str) -> (Id, PathBuf) {
        let job = self.job(key);
        let reply = self.call("submit", &["--key", key, "--file", &text(&job)]);
        let run = CanonicalIdentity.run(self.policy.cluster, key).unwrap();
        assert_eq!(reply["run"], json!(run));
        (run, job)
    }
    fn state(&self) -> State {
        serde_json::from_value(self.call("status", &["--raw"])).unwrap()
    }
    fn wait(&self, run: Id, phase: RunPhase) -> State {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let state = self.state();
            let observed = state.runs.get(&run).expect("submitted run").phase();
            if observed == phase {
                return state;
            }
            assert!(
                !terminal_run(observed),
                "expected {phase:?}, got {observed:?}: {}\n{}",
                serde_json::to_string(&state).unwrap(),
                self.worker.stderr()
            );
            assert!(
                Instant::now() < deadline,
                "run failed to reach {phase:?}: {}\n{}",
                serde_json::to_string(&state).unwrap(),
                self.worker.stderr()
            );
            thread::sleep(Duration::from_millis(25));
        }
    }
    fn stopped_attempt(&self, run: Id) -> State {
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            let state = self.state();
            if state
                .attempts
                .values()
                .any(|a| a.request.fence.run == run && matches!(a.phase, AttemptPhase::Terminal { .. }))
            {
                return state;
            }
            assert!(Instant::now() < deadline, "no stopped attempt after resumed publication denial");
            thread::sleep(Duration::from_millis(25));
        }
    }
    fn receipt(&self, id: Id, name: &str) -> (net::Receipt, Vec<u8>) {
        let file = self.base.path().join(format!("{name}.receipt.frame"));
        self.call("get", &["--id", &id.hex(), "--out", &text(&file)]);
        let bytes = bounded_file(&file);
        assert_eq!(content_id(&content(ContentKind::Data, bytes.clone()).unwrap()).unwrap(), id);
        let receipt = net::decode_receipt(&bytes).unwrap();
        assert_eq!(ok(&["pure-net", "receipt", "--file", &text(&file)]), serde_json::to_value(&receipt).unwrap());
        assert_eq!(receipt.manifest, self.prepared.manifest_id);
        assert_eq!(receipt.program, self.prepared.program_id);
        assert_eq!(receipt.ordered_inputs, self.prepared.input_id);
        assert_eq!(
            receipt.manifest_semantics,
            codec::identity("pure-net-eval/manifest/v1", &self.prepared.manifest).unwrap()
        );
        assert_eq!(receipt.program_semantics, self.prepared.program_semantics);
        assert_eq!(receipt.ordered_input_semantics, self.prepared.input_semantics);
        assert_eq!(receipt.bindings, self.prepared.manifest.bindings);
        assert_eq!(receipt.limits, self.prepared.manifest.limits);
        assert_eq!(receipt.grants, net::ExecutionGrants {
            fuel: FUEL,
            memory_bytes: MEMORY,
            max_output_bytes: OUTPUT,
            max_attempts: ATTEMPTS
        });
        (receipt, bytes)
    }
    fn successful_receipt(&self, state: &State, run: Id, name: &str, expected: &[i64]) -> (Id, net::Receipt, Vec<u8>) {
        let TaskPhase::Succeeded { output, .. } = state.runs[&run].tasks["eval"].phase else {
            panic!("no accepted native output")
        };
        let (receipt, bytes) = self.receipt(output, name);
        assert_eq!(receipt.outcome, net::Outcome::Completed(expected.iter().copied().map(net::Value::I64).collect()));
        assert!(receipt.checked_ir.is_some());
        assert!(receipt.producer_graph_digest.is_some());
        assert_eq!(receipt.compiled_net.is_some(), receipt.backend == net::Backend::SequentialNet);
        (output, receipt, bytes)
    }
    fn events(&self, run: Id, expected_outputs: usize) -> Value {
        let snapshot = self.call("progress", &["--run", &run.hex()]);
        let mut cursor = snapshot["watermark"].clone();
        cursor["revision"] = json!(0);
        cursor["ordinal"] = json!(0);
        let file = self.base.path().join(format!("{run}.cursor.json"));
        fs::write(&file, serde_json::to_vec(&cursor).unwrap()).unwrap();
        let page = self.call("progress", &["--cursor", &text(&file), "--limit", "64"]);
        assert_eq!(page, self.call("progress", &["--cursor", &text(&file), "--limit", "64"]));
        let events = page["Events"]["events"].as_array().expect("small campaign must fit retained event window");
        let run_value = json!(run);
        assert_eq!(
            events.iter().filter(|e| e["payload"]["AcceptedOutput"]["fence"]["run"] == run_value).count(),
            expected_outputs
        );
        assert_eq!(events.iter().filter(|e| e["payload"]["RunTerminal"]["run"] == run_value).count(), 1);
        let unique: std::collections::BTreeSet<_> = events.iter().map(|e| e["event_ref"].as_str().unwrap()).collect();
        assert_eq!(unique.len(), events.len());
        fs::write(self.base.path().join(format!("{run}.progress.json")), serde_json::to_vec_pretty(&page).unwrap())
            .unwrap();
        page
    }
    fn revoke(&self, root: &Path, artifact: Id) {
        ok(&["revoke", "--root", &text(root), "--artifact", &artifact.hex()]);
    }
    fn inspect_stopped_worker(&self) -> WorkerState {
        assert!(self.worker.child.is_none(), "exclusive journal inspection requires confirmed worker exit");
        let root = CapabilityRoot::open(&self.worker.root).unwrap();
        let disk = Disk::open(&root).expect("old owner has been killed and waited");
        let initial = WorkerState::empty(
            self.policy.cluster,
            self.worker_target.id,
            self.policy.generation,
            self.policy_ref,
            capacity(),
        )
        .unwrap();
        let journal = RedbWorkerJournal::open(disk, initial).unwrap();
        // Inspection does not itself invoke recovery. The restarted service owns it.
        journal.load().unwrap()
    }
    fn cancel_paused_worker(&self, request: ExecutionRequest) {
        assert!(self.controller.child.is_none(), "never create a second live controller endpoint");
        let root = CapabilityRoot::open(&self.controller.root).unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
        runtime.block_on(async {
            let endpoint = bind_endpoint(secret(&root).unwrap(), "127.0.0.1:0".parse().unwrap()).await.unwrap();
            let reply = rpc(&endpoint, &self.worker_target, &Request::CancelAttempt { request: request.clone() }, Duration::from_secs(15)).await.unwrap();
            endpoint.close().await;
            assert!(matches!(reply, Reply::Observation(WireObservation::Running { request_ref }) if request_ref == CanonicalIdentity.request(&request).unwrap()));
        });
    }
    fn measurements(&self) -> Vec<Value> {
        let values: Vec<_> = self
            .worker
            .stderr()
            .lines()
            .filter_map(|line| line.strip_prefix("pure-net-measurement "))
            .map(|json| serde_json::from_str::<Value>(json).expect("actual runtime measurement JSON"))
            .collect();
        assert!(!values.is_empty(), "runtime must retain phase measurements, not substitute total wall time");
        assert!(values.len() <= 16);
        for value in &values {
            assert_eq!(value["authoritative"], false);
            assert_eq!(
                value["phase_order"],
                json!([
                    "Decode",
                    "ParseCheck",
                    "Extract",
                    "Lower",
                    "Validate",
                    "Setup",
                    "Reduce",
                    "Readback",
                    "Reference",
                    "Complete"
                ])
            );
            let phases = value["phase_ns_excluding_gate_wait"].as_array().unwrap();
            assert_eq!(phases.len(), 10);
            assert!(phases.iter().all(|phase| phase.as_u64().is_some()));
            assert!(value["gate_wait_ns"].as_u64().is_some());
            assert!(value["calculation_wall_ns"].as_u64().is_some());
            assert!(value["handoffs"].as_u64().is_some());
        }
        values
    }
    fn retain(&self, name: &str, value: &Value) {
        let bytes = serde_json::to_vec_pretty(value).unwrap();
        assert!(bytes.len() <= DIAGNOSTIC_LIMIT);
        fs::write(self.base.path().join(name), bytes).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.worker.stop();
        let _ = self.controller.stop();
        if let Some(directory) = &self.evidence {
            for entry in fs::read_dir(self.base.path()).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    fs::copy(entry.path(), directory.join(entry.file_name())).unwrap();
                }
            }
            for (role, server) in [("worker", &self.worker), ("controller", &self.controller)] {
                for name in ["policy.json", "revoked.json", "config.json"] {
                    fs::write(directory.join(format!("{role}-{name}")), bounded_file(&server.root.join(name))).unwrap();
                }
                for (n, log) in server.logs.iter().enumerate() {
                    fs::write(directory.join(format!("{role}-{n:02}.stderr")), bounded_file(log)).unwrap();
                }
            }
            eprintln!("pure-net-evidence {}", directory.display());
        }
    }
}
fn subtraction() -> &'static net::fixtures::Fixture {
    &net::fixtures::POSITIVE_FIXTURES[0]
}
fn success_count(state: &State, run: Id) -> usize {
    state
        .attempts
        .values()
        .filter(|a| {
            a.request.fence.run == run
                && matches!(a.phase, AttemptPhase::Terminal {
                    result: ExecutionResult {
                        outcome: ExecutionOutcome::Succeeded { .. },
                        ..
                    },
                    ..
                })
        })
        .count()
}

// r[verify molten.pure_net.profile]
// r[verify molten.pure_net.identity]
// r[verify molten.pure_net.lifecycle]
// r[verify molten.pure_net.replay]
// r[verify molten.pure_net.evidence]
#[test]
fn pure_net_cli_receipt_binds_exact_objects_and_repeats() {
    let _serial = serial();
    let mut fixture = Fixture::new(
        "identity-repeat",
        net::Backend::SequentialNet,
        subtraction(),
        net::Limits::default(),
        None,
        true,
        false,
    );
    let (first, job) = fixture.submit("first");
    let first_state = fixture.wait(first, RunPhase::Succeeded);
    let (output, receipt, bytes) = fixture.successful_receipt(&first_state, first, "first", subtraction().expected);
    assert_eq!(fixture.call("submit", &["--key", "first", "--file", &text(&job)])["run"], json!(first));
    let (repeat, _) = fixture.submit("repeat");
    let repeat_state = fixture.wait(repeat, RunPhase::Succeeded);
    let (repeat_output, repeat_receipt, repeat_bytes) =
        fixture.successful_receipt(&repeat_state, repeat, "repeat", subtraction().expected);
    assert_eq!(output, repeat_output);
    assert_eq!(receipt, repeat_receipt);
    assert_eq!(bytes, repeat_bytes, "timing/process identities must not enter semantic receipt");
    fixture.events(first, 1);
    fixture.events(repeat, 1);
    assert_eq!(success_count(&repeat_state, first), 1);
    assert_eq!(success_count(&repeat_state, repeat), 1);
    fixture.worker.sample_memory();
    fixture.retain("identity-state.json", &serde_json::to_value(repeat_state).unwrap());
}

// r[verify molten.pure_net.bounds]
// r[verify molten.pure_net.outcomes]
#[test]
fn pure_net_typed_exhaustion_retains_failed_receipt_without_success() {
    let _serial = serial();
    let limits = net::Limits {
        interaction_fuel: 0,
        ..net::Limits::default()
    };
    let fixture =
        Fixture::new("reduction-exhaustion", net::Backend::SequentialNet, subtraction(), limits, None, true, false);
    let (run, _) = fixture.submit("exhausted");
    let state = fixture.wait(run, RunPhase::Failed);
    let mut reports = 0;
    for attempt in state.attempts.values().filter(|a| a.request.fence.run == run) {
        let AttemptPhase::Terminal { result, .. } = attempt.phase else {
            panic!("failure retry still owns work")
        };
        assert_eq!(result.outcome, ExecutionOutcome::Failed {
            code: FailureCode::ReductionLimit
        });
        let (receipt, _) = fixture.receipt(
            result.report.expect("typed exhaustion receipt must survive"),
            &format!("exhausted-{}", attempt.request.fence.epoch),
        );
        assert_eq!(receipt.outcome, net::Outcome::Rejected(net::Error::Exhausted(net::Resource::Reduction)));
        assert!(receipt.compiled_net.is_some());
        reports += 1;
    }
    assert_eq!(reports, ATTEMPTS);
    assert_eq!(success_count(&state, run), 0);
    assert_eq!(state.usage(fixture.worker_target.id).unwrap(), Resources::default());
    fixture.events(run, 0);
    fixture.retain("exhaustion-state.json", &serde_json::to_value(state).unwrap());
}

// r[verify molten.pure_net.profile]
// r[verify molten.pure_net.identity]
// r[verify molten.pure_net.authority]
// r[verify molten.pure_net.producer]
#[test]
fn pure_net_wrong_identity_missing_grants_and_revocation_deny_success() {
    let _serial = serial();
    {
        let base = tempfile::tempdir().unwrap();
        let prepared =
            Prepared::new(base.path(), net::Backend::SequentialNet, subtraction(), net::Limits::default(), false);
        let mut wrong_host = prepared.manifest.clone();
        wrong_host.bindings.host.executable = Id([0; 32]);
        let bytes = net::encode_manifest(&wrong_host).unwrap();
        let manifest = net::content_id_manifest(&bytes).unwrap();
        let wrong_file = base.path().join("wrong-executable.frame");
        fs::write(&wrong_file, bytes).unwrap();
        let policy = base.path().join("wrong-executable-policy.json");
        ok(&[
            "new-policy",
            "--out",
            &text(&policy),
            "--pure-net-manifest",
            &manifest.hex(),
            "--pure-net-program",
            &prepared.program_id.hex(),
        ]);
        let root = base.path().join("wrong-executable-controller");
        ok(&[
            "init",
            "--root",
            &text(&root),
            "--role",
            "controller",
            "--policy",
            &text(&policy),
            "--bind",
            &port(),
        ]);
        let result = invoke(&[
            "pure-net",
            "install",
            "--root",
            &text(&root),
            "--manifest",
            &text(&wrong_file),
        ]);
        assert!(!result.status.success(), "an explicitly granted manifest for another executable must not install");
        assert!(!root.join("pure-net-manifest.preserves-frame").exists());
    }
    {
        let fixture = Fixture::new(
            "missing-program-grant",
            net::Backend::SequentialNet,
            subtraction(),
            net::Limits::default(),
            None,
            false,
            false,
        );
        let job = fixture.job("missing-grant");
        assert!(!fixture.invoke("submit", &["--key", "missing-grant", "--file", &text(&job)]).status.success());
        assert!(fixture.state().runs.is_empty());
    }
    {
        let fixture = Fixture::new(
            "wrong-source-ir-identity",
            net::Backend::SequentialNet,
            subtraction(),
            net::Limits::default(),
            None,
            true,
            true,
        );
        let (run, _) = fixture.submit("forged-ir");
        let state = fixture.wait(run, RunPhase::Failed);
        for attempt in state.attempts.values().filter(|a| a.request.fence.run == run) {
            let AttemptPhase::Terminal { result, .. } = attempt.phase else {
                panic!("forged program still active")
            };
            assert!(matches!(result.outcome, ExecutionOutcome::Failed { .. }));
            let (receipt, _) = fixture.receipt(
                result.report.expect("bound source correspondence rejection receipt"),
                &format!("wrong-ir-{}", attempt.request.fence.epoch),
            );
            assert_eq!(receipt.outcome, net::Outcome::Rejected(net::Error::Bindings));
            assert!(receipt.compiled_net.is_none(), "identity mismatch must precede native lowering/reduction");
        }
        assert_eq!(success_count(&state, run), 0);
        fixture.events(run, 0);
        fixture.retain("wrong-ir-controller.json", &serde_json::to_value(state).unwrap());
    }
    for manifest in [false, true] {
        let fixture = Fixture::new(
            "revoked-before-submit",
            net::Backend::SequentialNet,
            subtraction(),
            net::Limits::default(),
            None,
            true,
            false,
        );
        let revoked = if manifest {
            fixture.prepared.manifest_id
        } else {
            fixture.prepared.program_id
        };
        fixture.revoke(&fixture.controller.root, revoked);
        let job = fixture.job("revoked");
        assert!(!fixture.invoke("submit", &["--key", "revoked", "--file", &text(&job)]).status.success());
        assert!(fixture.state().runs.is_empty());
    }
}

// r[verify molten.pure_net.authority]
// r[verify molten.pure_net.lifecycle]
#[test]
fn pure_net_revocation_after_calculation_cannot_publish() {
    let _serial = serial();
    let mut fixture = Fixture::new(
        "revocation-publication-race",
        net::Backend::SequentialNet,
        subtraction(),
        net::Limits::default(),
        Some("after-worker-calculation"),
        true,
        false,
    );
    let (run, _) = fixture.submit("revocation-race");
    fixture.worker.wait_pause("after-worker-calculation");
    fixture.revoke(&fixture.worker.root, fixture.prepared.program_id);
    fixture.revoke(&fixture.controller.root, fixture.prepared.program_id);
    fixture.worker.resume();
    let stopped = fixture.stopped_attempt(run);
    assert_eq!(success_count(&stopped, run), 0);
    assert_eq!(stopped.usage(fixture.worker_target.id).unwrap(), Resources::default());
    assert!(stopped.attempts.values().any(|a| matches!(a.phase, AttemptPhase::Terminal {
        result: ExecutionResult {
            outcome: ExecutionOutcome::Failed {
                code: FailureCode::PublicationDenied
            },
            stopped: true,
            ..
        },
        ..
    })));
    // A revoked retry cannot execute. Explicit cancellation finishes this bounded
    // operator scenario without treating denial as a new retry grant.
    fixture.call("cancel", &["--run", &run.hex()]);
    let state = fixture.wait(run, RunPhase::Cancelled);
    fixture.events(run, 0);
    fixture.worker.sample_memory();
    fixture.retain("revocation-state.json", &serde_json::to_value(state).unwrap());
}

// r[verify molten.pure_net.authority]
// r[verify molten.pure_net.lifecycle]
#[test]
fn pure_net_cancel_after_calculation_wins_over_late_success() {
    let _serial = serial();
    let mut fixture = Fixture::new(
        "cancellation-publication-race",
        net::Backend::SequentialNet,
        subtraction(),
        net::Limits::default(),
        Some("after-worker-calculation"),
        true,
        false,
    );
    let (run, _) = fixture.submit("cancel-race");
    fixture.worker.wait_pause("after-worker-calculation");
    fixture.call("cancel", &["--run", &run.hex()]);
    let latched = fixture.state();
    assert!(latched.runs[&run].cancel_requested);
    let request = latched.attempts.values().find(|a| a.request.fence.run == run).unwrap().request.clone();
    // Stop the actual controller before using its enrolled identity for a real
    // worker CancelAttempt RPC. This obtains a durable stop acknowledgement,
    // rather than sleeping and assuming a controller tick reached the worker.
    let _ = fixture.controller.stop();
    fixture.cancel_paused_worker(request);
    fixture.worker.resume();
    fixture.controller.launch(None);
    fixture.ready();
    let state = fixture.wait(run, RunPhase::Cancelled);
    assert_eq!(success_count(&state, run), 0);
    fixture.events(run, 0);
    fixture.worker.sample_memory();
    let _ = fixture.worker.stop();
    let worker = fixture.inspect_stopped_worker();
    assert!(worker.records.values().all(|r| matches!(
        r.phase,
        WorkerPhase::Terminal(ExecutionResult {
            outcome: ExecutionOutcome::Cancelled,
            stopped: true,
            ..
        })
    )));
    fixture.retain("cancelled-worker.json", &serde_json::to_value(worker).unwrap());
    fixture.retain("cancelled-controller.json", &serde_json::to_value(state).unwrap());
}

const LOSS_POINTS: &[&str] = &[
    "after-worker-accept",
    "after-worker-begin",
    "before-worker-calculation",
    "after-worker-calculation",
    "after-worker-output-persist",
    "before-worker-terminal-commit",
    "after-worker-terminal-commit",
    "before-worker-reply",
    "after-worker-ack",
];

// r[verify molten.pure_net.recovery]
// r[verify molten.pure_net.lifecycle]
#[test]
fn pure_net_kill_wait_reopen_all_worker_boundaries_preserve_one_authoritative_completion() {
    let _serial = serial();
    for point in LOSS_POINTS {
        let mut fixture = Fixture::new(
            point,
            net::Backend::SequentialNet,
            subtraction(),
            net::Limits::default(),
            Some(point),
            true,
            false,
        );
        let (run, job) = fixture.submit("worker-loss");
        fixture.worker.wait_pause(point);
        let before_kill = fixture.state();
        let retry_start = Instant::now();
        let exit = fixture.worker.stop().expect("owned paused worker");
        assert!(!exit.success());
        #[cfg(unix)]
        assert_eq!(std::os::unix::process::ExitStatusExt::signal(&exit), Some(9));
        let interrupted = fixture.inspect_stopped_worker();
        assert_eq!(interrupted.records.len(), 1);
        let old = interrupted.records.values().next().unwrap();
        let already_terminal = matches!(old.phase, WorkerPhase::Terminal(_));
        let late = matches!(*point, "after-worker-terminal-commit" | "before-worker-reply" | "after-worker-ack");
        assert_eq!(already_terminal, late, "actual durable record disagrees with {point}");
        if !late {
            assert_eq!(success_count(&before_kill, run), 0, "nonterminal worker work cannot imply controller success");
            assert_eq!(before_kill.usage(fixture.worker_target.id).unwrap(), capacity());
        }
        fixture.retain("worker-at-confirmed-exit.json", &serde_json::to_value(&interrupted).unwrap());
        fixture.worker.launch(None);
        let done = fixture.wait(run, RunPhase::Succeeded);
        let retry_ns = elapsed_ns(retry_start);
        let (output, receipt, _) = fixture.successful_receipt(&done, run, "recovered", subtraction().expected);
        assert_eq!(success_count(&done, run), 1);
        assert_eq!(done.usage(fixture.worker_target.id).unwrap(), Resources::default());
        assert_eq!(fixture.call("submit", &["--key", "worker-loss", "--file", &text(&job)])["run"], json!(run));
        fixture.events(run, 1);
        fixture.worker.sample_memory();
        let _ = fixture.worker.stop();
        let recovered = fixture.inspect_stopped_worker();
        if late {
            assert_eq!(
                recovered.records, interrupted.records,
                "postterminal restart must retain the same stopped result"
            );
        } else {
            assert_eq!(recovered.records.len(), 2);
            assert!(matches!(
                recovered.records[&old.request.fence.assignment].phase,
                WorkerPhase::Terminal(ExecutionResult {
                    outcome: ExecutionOutcome::Failed {
                        code: FailureCode::Interrupted
                    },
                    stopped: true,
                    ..
                })
            ));
            assert!(recovered.records.values().any(|r| r.request.fence.epoch == 2 && matches!(r.phase,
                WorkerPhase::Terminal(ExecutionResult { outcome: ExecutionOutcome::Succeeded { output: actual }, .. }) if actual == output)));
        }
        fixture.retain("recovered-worker.json", &serde_json::to_value(recovered).unwrap());
        fixture.retain("recovered-controller.json", &serde_json::to_value(done).unwrap());
        let observation = json!({"point":point,"kill_wait_reopen_to_completion_ns":retry_ns,
            "already_terminal_before_loss":late,"receipt_counters":receipt.counters,
            "worker_process_vm_hwm_kib":fixture.worker.vm_hwm_kib,
            "scope":"actual same-host owned processes; no cross-host qualification"});
        fixture.retain("retry-observation.json", &observation);
        eprintln!("pure-net-worker-loss {}", serde_json::to_string(&observation).unwrap());
    }
}

// r[verify molten.pure_net.baseline]
// r[verify molten.pure_net.replay]
#[test]
fn pure_net_frozen_matched_service_baseline_retains_unfavorable_and_retry_costs() {
    let _serial = serial();
    let mut records = Vec::new();
    for index in [0, 5, 8] {
        let source = &net::fixtures::POSITIVE_FIXTURES[index];
        let mut matching_contract = None;
        for backend in [net::Backend::SequentialNet, net::Backend::IndependentReference] {
            let mut expected_receipt = None;
            for interrupted in [false, true] {
                let mut fixture = Fixture::new(
                    source.name,
                    backend,
                    source,
                    net::Limits::default(),
                    interrupted.then_some("after-worker-calculation"),
                    true,
                    false,
                );
                let start = Instant::now();
                let (run, _) = fixture.submit("matched-baseline");
                let mut retry_start = None;
                if interrupted {
                    fixture.worker.wait_pause("after-worker-calculation");
                    retry_start = Some(Instant::now());
                    let exit = fixture.worker.stop().expect("kill and wait exact interrupted owner");
                    assert!(!exit.success());
                    #[cfg(unix)]
                    assert_eq!(std::os::unix::process::ExitStatusExt::signal(&exit), Some(9));
                    let stopped = fixture.inspect_stopped_worker();
                    assert!(stopped.records.values().all(|r| !matches!(r.phase, WorkerPhase::Terminal(_))));
                    fixture.worker.launch(None);
                }
                let state = fixture.wait(run, RunPhase::Succeeded);
                let retry_ns = retry_start.map(elapsed_ns);
                let (output, receipt, bytes) = fixture.successful_receipt(&state, run, "baseline", source.expected);
                let end_to_end_ns = elapsed_ns(start);
                let canonical = (output, bytes);
                if let Some(expected) = &expected_receipt {
                    assert_eq!(
                        &canonical, expected,
                        "fresh worker PID and interrupted attempt must not change a fully bound canonical receipt"
                    );
                } else {
                    expected_receipt = Some(canonical);
                }
                fixture.events(run, 1);
                let vm_hwm_kib = fixture.worker.sample_memory();
                let measurements = fixture.measurements();
                assert_eq!(measurements.len(), if interrupted { 2 } else { 1 });
                let observed_epochs: std::collections::BTreeSet<_> =
                    measurements.iter().map(|measurement| measurement["request_epoch"].as_u64().unwrap()).collect();
                let expected_epochs: std::collections::BTreeSet<u64> =
                    if interrupted { [1_u64, 2].into() } else { [1_u64].into() };
                assert_eq!(observed_epochs, expected_epochs);
                let contract = (
                    fixture.prepared.program_id,
                    fixture.prepared.input_id,
                    receipt.limits.clone(),
                    receipt.grants.clone(),
                    receipt.bindings.host.clone(),
                );
                if let Some(expected) = &matching_contract {
                    assert_eq!(
                        &contract, expected,
                        "net/reference/retry cases must use identical immutable inputs, limits and grants"
                    );
                } else {
                    matching_contract = Some(contract);
                }
                fixture.retain("baseline-controller.json", &serde_json::to_value(&state).unwrap());
                if interrupted {
                    assert_eq!(state.attempts.values().filter(|a| a.request.fence.run == run).count(), 2);
                }
                if !fixture.worker.heaptrack_raws.is_empty() {
                    // Flush the final process's profiler buffers only after
                    // canonical output/state and VmHWM have been observed.
                    fixture.worker.stop_gracefully();
                }
                let record = json!({"schema":"aspen.pure-net.matched-service-baseline/v1",
                    "fixture":source.name,"source":source.source,"inputs":source.inputs,"independent_expected":source.expected,
                    "backend":backend,"manifest":fixture.prepared.manifest_id,"program":fixture.prepared.program_id,"ordered_inputs":fixture.prepared.input_id,
                    "host_bindings":receipt.bindings.host,"limits":receipt.limits,"grants":receipt.grants,"output":output,
                    "preparation_ns":fixture.prepared.preparation_ns,"submit_through_get_and_cli_receipt_decode_ns":end_to_end_ns,
                    "kill_wait_reopen_to_terminal_ns":retry_ns,"interrupted_attempt":interrupted,"runtime_phase_measurements":measurements,
                    "logical_counters":receipt.counters,"worker_process_vm_hwm_kib":vm_hwm_kib,
                    "heaptrack_raw_private":fixture.worker.heaptrack_raws,
                    "heaptrack_scope":"optional separately instrumented worker lifetime; only the gracefully exited final process is complete; raw addresses stay in the operator's private directory",
                    "logical_counter_scope":"final accepted receipt only; killed uncommitted receipt is not reconstructed; interrupted work is retained in runtime phase diagnostics",
                    "preparation_scope":"CLI manifest/compile plus canonical identity verification; not isolated producer compile time",
                    "memory_scope":"Linux whole-worker VmHWM includes initialization, runtime, retained state and all attempts; not per-evaluation allocator instrumentation",
                    "allocation_scope":"receipt allocated/reused/freed slots are logical arena events, not total process allocation counts",
                    "timing_scope":"diagnostic wall/phase observations only; not canonical receipt fields; no timing pass threshold",
                    "qualification":"actual same-host separate processes; frozen producer performance NO-GO preserved; no adoption/speedup claim"});
                fixture.retain("matched-baseline.json", &record);
                records.push(record);
            }
        }
    }
    let bytes = serde_json::to_vec(&records).unwrap();
    assert!(bytes.len() <= 256 * 1024, "bounded baseline diagnostic record");
    eprintln!("pure-net-baseline {}", String::from_utf8(bytes).unwrap());
}
