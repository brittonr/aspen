// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real producer execution through the async native port; no fake reducer.
#![cfg(target_os = "linux")]
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::time::Duration;

use molten_batch_application::*;
use molten_batch_core::*;
use molten_orchestrator_adapters::*;
use molten_pure_net as net;
use tokio::sync::Notify;

struct Material {
    manifest: Content,
    program: Content,
    inputs: Vec<u8>,
    installed: InstalledPureNet,
}
impl Material {
    fn new(source: &str, limits: net::Limits) -> Self {
        let host = pure_net_host_bindings().unwrap();
        let decoded = net::manifest(host.clone(), net::Backend::SequentialNet, limits).unwrap();
        let manifest = content(ContentKind::PureNetManifest, net::encode_manifest(&decoded).unwrap()).unwrap();
        let program = content(ContentKind::PureNetProgram, net::compile(source, 2).unwrap()).unwrap();
        let installed = InstalledPureNet {
            manifest_ref: content_id(&manifest).unwrap(),
            manifest: decoded,
            host,
            content: manifest.clone(),
        };
        Self {
            manifest,
            program,
            inputs: net::encode_inputs(&[19, 7]).unwrap(),
            installed,
        }
    }
    fn request(&self, key: &str) -> ExecutionRequest {
        let identity = CanonicalIdentity;
        let workload = Workload::PureNet {
            manifest: content_id(&self.manifest).unwrap(),
            artifact: content_id(&self.program).unwrap(),
        };
        let input = content(ContentKind::Data, self.inputs.clone()).unwrap();
        let resources = Resources {
            cpu_millis: 1000,
            memory_bytes: 64 * 1024 * 1024,
            slots: 1,
        };
        let task = TaskSpec {
            id: "evaluation".into(),
            workload,
            inputs: vec![content_id(&input).unwrap()],
            dependencies: vec![],
            resources,
            required_labels: Default::default(),
            fuel: 1_000_000_000,
            timeout_ms: 30_000,
            max_output_bytes: MAX_CONTENT_BYTES as u64,
            max_attempts: 2,
            retry_delay_ticks: 1,
        };
        let spec = JobSpec {
            name: key.into(),
            tasks: vec![task.clone()],
        };
        let spec_ref = identity.spec(&spec).unwrap();
        let cluster = Id([1; 32]);
        let run = identity.run(cluster, key).unwrap();
        let placement = Placement {
            run,
            task: task.id.clone(),
            node: Id([4; 32]),
            number: 1,
            inputs: task.inputs.clone(),
        };
        ExecutionRequest {
            fence: Fence {
                cluster,
                generation: 1,
                run,
                task: task.id,
                assignment: identity.assignment(cluster, 1, &placement, spec_ref).unwrap(),
                spec_ref,
                node: placement.node,
                epoch: 1,
            },
            policy_ref: Id([2; 32]),
            workload,
            inputs: task.inputs,
            resources,
            fuel: task.fuel,
            timeout_ms: task.timeout_ms,
            max_output_bytes: task.max_output_bytes,
            max_attempts: task.max_attempts,
        }
    }
    fn materialized(&self) -> MaterializedWorkload {
        MaterializedWorkload::PureNet {
            manifest: self.manifest.clone(),
            artifact: self.program.clone(),
        }
    }
}

#[derive(Default)]
struct Fairness {
    first_turn: Notify,
    turns: AtomicUsize,
    unrelated_done: AtomicBool,
    observed_between_turns: AtomicBool,
}
impl ObservabilityPort for Fairness {
    fn record(&self, diagnostic: Diagnostic) {
        if diagnostic == Diagnostic::Boundary("pure-net-local-quantum") {
            let previous = self.turns.fetch_add(1, Ordering::AcqRel);
            if previous == 0 {
                self.first_turn.notify_one();
            } else if self.unrelated_done.load(Ordering::Acquire) {
                self.observed_between_turns.store(true, Ordering::Release);
            }
        }
    }
}

// r[verify molten.pure_net.quanta]
// r[verify molten.pure_net.replay]
#[tokio::test(flavor = "current_thread")]
async fn unrelated_runnable_completes_between_real_native_quanta() {
    let workspace = tempfile::tempdir().unwrap();
    let root = CapabilityRoot::open(workspace.path()).unwrap();
    let disk = Disk::open(&root).unwrap();
    let data = Material::new("-", net::Limits::default());
    let observed = Arc::new(Fairness::default());
    let runtime = PureNetRuntime::for_worker(disk, &data.installed, observed.clone());
    let sibling = observed.clone();
    let other = tokio::spawn(async move {
        sibling.first_turn.notified().await;
        sibling.unrelated_done.store(true, Ordering::Release);
    });
    let output = tokio::time::timeout(
        Duration::from_secs(30),
        runtime.run(data.request("fairness"), data.materialized(), vec![data.inputs.clone()]),
    )
    .await
    .unwrap()
    .unwrap();
    other.await.unwrap();
    let encoded = output.output.unwrap();
    let receipt = net::decode_receipt(&encoded).unwrap();
    assert!(output.stopped);
    assert_eq!(receipt.outcome, net::Outcome::Completed(vec![net::Value::I64(12)]));
    assert!(
        observed.observed_between_turns.load(Ordering::Acquire),
        "sibling must run after the first and before a later actual handoff"
    );
    let (_, direct) = net::execute(
        net::ExecutionInput {
            manifest_bytes: &data.manifest.bytes,
            program_bytes: &data.program.bytes,
            input_bytes: &data.inputs,
            host: &data.installed.host,
            grants: &receipt.grants,
        },
        &mut |_| net::Control::Continue,
    )
    .unwrap();
    assert_eq!(direct, encoded, "async handoff must not restart or perturb the real calculation");
}

#[derive(Default)]
struct HoldTwo {
    entered: AtomicUsize,
    notification: Notify,
    released: Mutex<bool>,
    changed: Condvar,
}
impl HoldTwo {
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}
impl ObservabilityPort for HoldTwo {
    fn record(&self, diagnostic: Diagnostic) {
        if diagnostic != Diagnostic::Boundary("pure-net-local-quantum") {
            return;
        }
        if self.entered.fetch_add(1, Ordering::AcqRel) < 2 {
            self.notification.notify_one();
            let mut released = self.released.lock().unwrap();
            while !*released {
                released = self.changed.wait(released).unwrap();
            }
        }
    }
}
struct ReleaseOnDrop(Arc<HoldTwo>);
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}

// r[verify molten.pure_net.bounds]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_native_limit_rejects_the_next_evaluation_without_starting_it() {
    let workspace = tempfile::tempdir().unwrap();
    let root = CapabilityRoot::open(workspace.path()).unwrap();
    let disk = Disk::open(&root).unwrap();
    let data = Material::new("-", net::Limits::default());
    let observed = Arc::new(HoldTwo::default());
    let _release_on_failure = ReleaseOnDrop(observed.clone());
    let runtime = Arc::new(PureNetRuntime::for_worker(disk, &data.installed, observed.clone()));
    let mut calculations = Vec::new();
    for key in ["first", "second"] {
        let executor = runtime.clone();
        let request = data.request(key);
        let material = data.materialized();
        let inputs = vec![data.inputs.clone()];
        calculations.push(tokio::spawn(async move { executor.run(request, material, inputs).await }));
    }
    tokio::time::timeout(Duration::from_secs(30), async {
        while observed.entered.load(Ordering::Acquire) < 2 {
            observed.notification.notified().await;
        }
    })
    .await
    .unwrap();
    let third = runtime.run(data.request("third"), data.materialized(), vec![data.inputs.clone()]).await.unwrap();
    assert!(third.stopped);
    assert_eq!(third.output, Err(FailureCode::ConcurrencyLimit));
    assert_eq!(third.report, None);
    observed.release();
    for calculation in calculations {
        let completed = tokio::time::timeout(Duration::from_secs(30), calculation).await.unwrap().unwrap().unwrap();
        assert_eq!(
            net::decode_receipt(&completed.output.unwrap()).unwrap().outcome,
            net::Outcome::Completed(vec![net::Value::I64(12)])
        );
    }
}

// r[verify molten.pure_net.bounds]
// r[verify molten.pure_net.outcomes]
#[tokio::test]
async fn exhausted_native_job_releases_the_only_permit_for_a_valid_sibling() {
    let workspace = tempfile::tempdir().unwrap();
    let root = CapabilityRoot::open(workspace.path()).unwrap();
    let disk = Disk::open(&root).unwrap();
    let limits = net::Limits {
        interaction_fuel: 0,
        max_concurrent_evaluations: 1,
        ..net::Limits::default()
    };
    let failed = Material::new("-", limits.clone());
    let observed = Arc::new(Fairness::default());
    let runtime = PureNetRuntime::for_worker(disk, &failed.installed, observed);
    let output = runtime
        .run(failed.request("exhausted"), failed.materialized(), vec![failed.inputs.clone()])
        .await
        .unwrap();
    assert!(output.stopped);
    assert_eq!(output.output, Err(FailureCode::ReductionLimit));
    assert_eq!(
        net::decode_receipt(&output.report.unwrap()).unwrap().outcome,
        net::Outcome::Rejected(net::Error::Exhausted(net::Resource::Reduction))
    );
    let mut sibling = Material::new("swap", limits);
    // Both programs use exactly the same installed manifest and input binding.
    sibling.manifest = failed.manifest.clone();
    let output = runtime
        .run(sibling.request("valid-sibling"), sibling.materialized(), vec![sibling.inputs.clone()])
        .await
        .unwrap();
    assert_eq!(
        net::decode_receipt(&output.output.unwrap()).unwrap().outcome,
        net::Outcome::Completed(vec![net::Value::I64(7), net::Value::I64(19)])
    );
}
