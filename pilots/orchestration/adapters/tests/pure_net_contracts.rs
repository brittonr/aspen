// SPDX-License-Identifier: AGPL-3.0-or-later
mod common;
use std::collections::BTreeSet;
use std::sync::Arc;

use common::*;
use molten_batch_application::*;
use molten_batch_core::*;
use molten_orchestrator_adapters::*;

fn policy(profile: &str) -> Policy {
    Policy {
        profile: profile.into(),
        cluster: id(1),
        generation: 1,
        allow_any_pure_component: profile == PROFILE,
        allowed_components: BTreeSet::new(),
        allowed_pure_net_manifests: BTreeSet::new(),
        allowed_pure_net_programs: BTreeSet::new(),
        max_memory_bytes: resources().memory_bytes,
        max_fuel: 1_000_000,
        max_timeout_ms: 2000,
        max_output_bytes: 1024,
    }
}
fn install(root: &CapabilityRoot, policy: &Policy) -> Id {
    root.create_new("policy.json", &serde_json::to_vec(policy).unwrap()).unwrap();
    root.create_new("revoked.json", b"[]").unwrap();
    policy_id(policy).unwrap()
}

#[test]
fn file_admission_separates_native_grants_and_rechecks_each_revocation() {
    let temp = tempfile::tempdir().unwrap();
    let root = CapabilityRoot::open(temp.path()).unwrap();
    let mut p = policy(PURE_NET_PROFILE);
    p.allowed_pure_net_manifests.insert(id(7));
    p.allowed_pure_net_programs.insert(id(8));
    let policy_ref = install(&root, &p);
    let admission = FileAdmission::new(root.clone(), policy_ref).unwrap();
    let mut r = request(id(8));
    r.policy_ref = policy_ref;
    r.workload = Workload::PureNet {
        manifest: id(7),
        artifact: id(8),
    };
    for action in [
        AdmissionAction::Reserve,
        AdmissionAction::Execute,
        AdmissionAction::Publish,
    ] {
        admission.assignment(&r, action).unwrap();
    }
    for revoked in [id(7), id(8)] {
        root.replace("revoked.json", &serde_json::to_vec(&[revoked]).unwrap()).unwrap();
        for action in [
            AdmissionAction::Reserve,
            AdmissionAction::Execute,
            AdmissionAction::Publish,
        ] {
            assert!(matches!(admission.assignment(&r, action), Err(Error::Denied)));
        }
        admission.assignment(&r, AdmissionAction::Complete).unwrap();
        admission.assignment(&r, AdmissionAction::Cancel).unwrap();
    }
    root.replace("revoked.json", b"[]").unwrap();
    let mut wrong = r.clone();
    wrong.workload = Workload::PureNet {
        manifest: id(8),
        artifact: id(7),
    };
    assert!(matches!(admission.assignment(&wrong, AdmissionAction::Execute), Err(Error::Denied)));
    wrong = r.clone();
    wrong.fence.generation += 1;
    assert!(matches!(admission.assignment(&wrong, AdmissionAction::Publish), Err(Error::Denied)));
    wrong = r.clone();
    wrong.workload = Workload::Component { artifact: id(8) };
    assert!(matches!(admission.assignment(&wrong, AdmissionAction::Execute), Err(Error::Denied)));

    let temp = tempfile::tempdir().unwrap();
    let root = CapabilityRoot::open(temp.path()).unwrap();
    r.policy_ref = install(&root, &policy(PROFILE));
    let wasm_only = FileAdmission::new(root, r.policy_ref).unwrap();
    assert!(matches!(wasm_only.assignment(&r, AdmissionAction::Execute), Err(Error::Denied)));
}

struct RevokeAtBoundary {
    root: CapabilityRoot,
    artifact: Id,
    boundary: &'static str,
}
impl ObservabilityPort for RevokeAtBoundary {
    fn record(&self, event: Diagnostic) {
        if event == Diagnostic::Boundary(self.boundary) {
            self.root.replace("revoked.json", &serde_json::to_vec(&[self.artifact]).unwrap()).unwrap();
        }
    }
}

#[tokio::test]
async fn real_component_publication_rechecks_revocation_before_bytes_and_terminal() {
    for boundary in [
        None,
        Some("after-worker-calculation"),
        Some("after-worker-output-persist"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = CapabilityRoot::open(temp.path()).unwrap();
        let policy_ref = install(&root, &policy(PROFILE));
        let disk = Disk::open(&root).unwrap();
        let journal = RedbWorkerJournal::open(
            disk.clone(),
            WorkerState::empty(id(1), id(4), 1, policy_ref, resources()).unwrap(),
        )
        .unwrap();
        let content = RedbContent::new(disk.clone());
        let artifact = content.put(&fixture()).unwrap();
        let observations: Arc<dyn ObservabilityPort> = match boundary {
            Some(boundary) => Arc::new(RevokeAtBoundary {
                root: root.clone(),
                artifact,
                boundary,
            }),
            None => Arc::new(Counters::default()),
        };
        let worker = WorkerService::new(WorkerPorts {
            journal: journal.clone(),
            content: content.clone(),
            identity: Arc::new(CanonicalIdentity),
            admission: Arc::new(FileAdmission::new(root, policy_ref).unwrap()),
            execution: Arc::new(PureComponentRuntime::for_worker(disk)),
            observations,
        });
        worker.recover_after_exclusive_open().unwrap();
        let mut r = request(artifact);
        r.policy_ref = policy_ref;
        let (_, ticket) = worker.start(r.clone()).unwrap();
        worker.execute(ticket.unwrap()).await.unwrap();
        let expected_output = content_id(&content.make_data(b"ok".to_vec()).unwrap()).unwrap();
        let state = worker.state().unwrap();
        let WorkerPhase::Terminal(result) = state.records[&r.fence.assignment].phase else {
            panic!("not stopped")
        };
        assert_eq!(result.report, None);
        assert_eq!(state.usage().unwrap(), Resources::default());
        match boundary {
            None => {
                assert_eq!(result.outcome, ExecutionOutcome::Succeeded {
                    output: expected_output
                });
                assert_eq!(content.get(expected_output).unwrap().bytes, b"ok");
            }
            Some(boundary) => {
                assert_eq!(result.outcome, ExecutionOutcome::Failed {
                    code: FailureCode::PublicationDenied
                });
                assert_eq!(content.get(expected_output).is_ok(), boundary == "after-worker-output-persist");
            }
        }
        let (duplicate, ticket) = worker.start(r).unwrap();
        assert!(ticket.is_none());
        assert!(matches!(duplicate, WorkerObservation::Terminal { result: observed, .. } if observed == result));
    }
}

#[tokio::test]
async fn revoked_retry_cannot_starve_a_later_authorized_run() {
    let worker_temp = tempfile::tempdir().unwrap();
    let worker_root = CapabilityRoot::open(worker_temp.path()).unwrap();
    let p = policy(PROFILE);
    let policy_ref = install(&worker_root, &p);
    let disk = Disk::open(&worker_root).unwrap();
    let journal =
        RedbWorkerJournal::open(disk.clone(), WorkerState::empty(id(1), id(4), 1, policy_ref, resources()).unwrap())
            .unwrap();
    let worker = Arc::new(WorkerService::new(WorkerPorts {
        journal,
        content: RedbContent::new(disk.clone()),
        identity: Arc::new(CanonicalIdentity),
        admission: Arc::new(FileAdmission::new(worker_root, policy_ref).unwrap()),
        execution: Arc::new(PureComponentRuntime::for_worker(disk)),
        observations: Arc::new(Counters::default()),
    }));
    worker.recover_after_exclusive_open().unwrap();

    let controller_temp = tempfile::tempdir().unwrap();
    let controller_root = CapabilityRoot::open(controller_temp.path()).unwrap();
    assert_eq!(install(&controller_root, &p), policy_ref);
    let disk = Disk::open(&controller_root).unwrap();
    let store = RedbControllerStore::open(disk.clone(), State::empty(id(1), policy_ref, 1, 3).unwrap()).unwrap();
    let content = RedbContent::new(disk);
    let denied_artifact = content.put(&fixture()).unwrap();
    let mut sibling_bytes = fixture().bytes;
    sibling_bytes.extend_from_slice(b"\n;; authorized sibling\n");
    let sibling_artifact = content
        .put(&molten_orchestrator_adapters::content(ContentKind::Component, sibling_bytes).unwrap())
        .unwrap();
    let new_controller = || {
        Controller::new(ControllerPorts {
            store: store.clone(),
            content: content.clone(),
            identity: Arc::new(CanonicalIdentity),
            placement: Arc::new(FirstFitPlacement),
            admission: Arc::new(FileAdmission::new(controller_root.clone(), policy_ref).unwrap()),
            workers: Arc::new(InProcessWorker(worker.clone())),
            observations: Arc::new(RevokeAtBoundary {
                root: controller_root.clone(),
                artifact: denied_artifact,
                boundary: "before-terminal-commit",
            }),
        })
    };
    let controller = new_controller();
    controller
        .register(Node {
            id: id(4),
            capacity: resources(),
            labels: Default::default(),
            draining: false,
            enabled: true,
            last_seen: None,
            boot: 0,
        })
        .unwrap();

    let sibling_key = "authorized";
    let sibling_id = CanonicalIdentity.run(id(1), sibling_key).unwrap();
    let denied_key = (0..128)
        .map(|i| format!("revoked-{i}"))
        .find(|key| CanonicalIdentity.run(id(1), key).unwrap() < sibling_id)
        .unwrap();
    let denied_run = controller
        .submit(denied_key.clone(), JobSpec {
            name: denied_key,
            tasks: vec![task(denied_artifact)],
        })
        .unwrap();
    assert!(denied_run < sibling_id);
    controller.tick().await.unwrap();
    let state = controller.state().unwrap();
    assert_eq!(state.runs[&denied_run].phase(), RunPhase::Pending);
    assert_eq!(state.attempts.len(), 1);
    assert!(matches!(state.attempts.values().next().unwrap().phase, AttemptPhase::Terminal {
        result: ExecutionResult {
            outcome: ExecutionOutcome::Failed {
                code: FailureCode::PublicationDenied
            },
            ..
        },
        ..
    }));

    let sibling_run = controller
        .submit(sibling_key.into(), JobSpec {
            name: sibling_key.into(),
            tasks: vec![task(sibling_artifact)],
        })
        .unwrap();
    assert_eq!(sibling_run, sibling_id);
    drop(controller);
    // No in-memory exclusion survives this controller recreation: Reserve must
    // skip the newly ready, revoked retry without terminalizing it.
    let controller = new_controller();
    controller.tick().await.unwrap();
    let state = controller.state().unwrap();
    assert_eq!(state.runs[&denied_run].phase(), RunPhase::Pending);
    assert_eq!(state.runs[&sibling_run].phase(), RunPhase::Succeeded);
    assert_eq!(state.attempts.len(), 2);
    assert_eq!(state.attempts.values().filter(|a| a.request.fence.run == denied_run).count(), 1);
    assert_eq!(worker.state().unwrap().records.values().filter(|r| r.request.fence.run == denied_run).count(), 1);
    assert!(state.attempts.values().any(|a| a.request.fence.run == sibling_run
        && matches!(a.phase, AttemptPhase::Terminal {
            result: ExecutionResult {
                outcome: ExecutionOutcome::Succeeded { .. },
                ..
            },
            ..
        })));
}

#[tokio::test]
async fn denied_first_task_does_not_starve_authorized_sibling_in_same_run() {
    let worker_temp = tempfile::tempdir().unwrap();
    let worker_root = CapabilityRoot::open(worker_temp.path()).unwrap();
    let p = policy(PROFILE);
    let policy_ref = install(&worker_root, &p);
    let disk = Disk::open(&worker_root).unwrap();
    let journal =
        RedbWorkerJournal::open(disk.clone(), WorkerState::empty(id(1), id(4), 1, policy_ref, resources()).unwrap())
            .unwrap();
    let worker = Arc::new(WorkerService::new(WorkerPorts {
        journal,
        content: RedbContent::new(disk.clone()),
        identity: Arc::new(CanonicalIdentity),
        admission: Arc::new(FileAdmission::new(worker_root, policy_ref).unwrap()),
        execution: Arc::new(PureComponentRuntime::for_worker(disk)),
        observations: Arc::new(Counters::default()),
    }));
    worker.recover_after_exclusive_open().unwrap();

    let controller_temp = tempfile::tempdir().unwrap();
    let root = CapabilityRoot::open(controller_temp.path()).unwrap();
    assert_eq!(install(&root, &p), policy_ref);
    let disk = Disk::open(&root).unwrap();
    let store = RedbControllerStore::open(disk.clone(), State::empty(id(1), policy_ref, 1, 3).unwrap()).unwrap();
    let content = RedbContent::new(disk);
    let denied = content.put(&fixture()).unwrap();
    let mut sibling_bytes = fixture().bytes;
    sibling_bytes.extend_from_slice(b"\n;; sibling task\n");
    let authorized = content
        .put(&molten_orchestrator_adapters::content(ContentKind::Component, sibling_bytes).unwrap())
        .unwrap();
    let controller = Controller::new(ControllerPorts {
        store,
        content,
        identity: Arc::new(CanonicalIdentity),
        placement: Arc::new(FirstFitPlacement),
        admission: Arc::new(FileAdmission::new(root.clone(), policy_ref).unwrap()),
        workers: Arc::new(InProcessWorker(worker.clone())),
        observations: Arc::new(Counters::default()),
    });
    controller
        .register(Node {
            id: id(4),
            capacity: resources(),
            labels: Default::default(),
            draining: false,
            enabled: true,
            last_seen: None,
            boot: 0,
        })
        .unwrap();
    let mut first = task(denied);
    first.id = "a".into();
    let mut sibling = task(authorized);
    sibling.id = "z".into();
    let run = controller
        .submit("same-run".into(), JobSpec {
            name: "same-run".into(),
            tasks: vec![first, sibling],
        })
        .unwrap();
    root.replace("revoked.json", &serde_json::to_vec(&[denied]).unwrap()).unwrap();
    controller.tick().await.unwrap();
    let state = controller.state().unwrap();
    assert!(matches!(state.runs[&run].tasks["a"].phase, TaskPhase::Pending { .. }));
    assert!(matches!(state.runs[&run].tasks["z"].phase, TaskPhase::Succeeded { .. }));
    assert_eq!(state.attempts.len(), 1);
    assert_eq!(state.attempts.values().next().unwrap().request.fence.task, "z");
    assert_eq!(worker.state().unwrap().records.len(), 1);
}

struct UnknownDataContent {
    inner: Arc<RedbContent>,
}
impl ContentPort for UnknownDataContent {
    fn get(&self, id: Id) -> Result<Content, Error> {
        self.inner.get(id)
    }
    fn put(&self, value: &Content) -> Result<Id, Error> {
        let id = self.inner.put(value)?;
        if value.manifest.kind == ContentKind::Data {
            Err(Error::Unknown)
        } else {
            Ok(id)
        }
    }
    fn make_data(&self, bytes: Vec<u8>) -> Result<Content, Error> {
        self.inner.make_data(bytes)
    }
}

#[tokio::test]
async fn unknown_actual_output_persistence_keeps_ownership_and_no_second_ticket() {
    let temp = tempfile::tempdir().unwrap();
    let root = CapabilityRoot::open(temp.path()).unwrap();
    let policy_ref = install(&root, &policy(PROFILE));
    let disk = Disk::open(&root).unwrap();
    let journal =
        RedbWorkerJournal::open(disk.clone(), WorkerState::empty(id(1), id(4), 1, policy_ref, resources()).unwrap())
            .unwrap();
    let content = RedbContent::new(disk.clone());
    let artifact = content.put(&fixture()).unwrap();
    let worker = WorkerService::new(WorkerPorts {
        journal,
        content: Arc::new(UnknownDataContent { inner: content.clone() }),
        identity: Arc::new(CanonicalIdentity),
        admission: Arc::new(FileAdmission::new(root, policy_ref).unwrap()),
        execution: Arc::new(PureComponentRuntime::for_worker(disk)),
        observations: Arc::new(Counters::default()),
    });
    worker.recover_after_exclusive_open().unwrap();
    let mut r = request(artifact);
    r.policy_ref = policy_ref;
    let (_, ticket) = worker.start(r.clone()).unwrap();
    assert!(matches!(worker.execute(ticket.unwrap()).await, Err(Error::Unknown)));
    let state = worker.state().unwrap();
    assert_eq!(state.records[&r.fence.assignment].phase, WorkerPhase::Running);
    assert_eq!(state.usage().unwrap(), resources());
    let output = content_id(&content.make_data(b"ok".to_vec()).unwrap()).unwrap();
    assert_eq!(content.get(output).unwrap().bytes, b"ok");
    assert!(worker.start(r).unwrap().1.is_none());
}

#[tokio::test]
async fn wasm_adapter_rejects_native_even_when_program_bytes_are_a_valid_component() {
    let artifact = fixture();
    let manifest = content(ContentKind::PureNetManifest, b"explicit native manifest".to_vec()).unwrap();
    let mut r = request(content_id(&artifact).unwrap());
    r.workload = Workload::PureNet {
        manifest: content_id(&manifest).unwrap(),
        artifact: content_id(&artifact).unwrap(),
    };
    assert!(matches!(
        PureComponentRuntime::default()
            .run(r, MaterializedWorkload::PureNet { manifest, artifact }, vec![])
            .await,
        Err(Error::Denied)
    ));
}

#[test]
fn content_boundary_is_inclusive_and_kind_and_length_are_bound() {
    let bytes = vec![0; MAX_CONTENT_BYTES];
    let manifest = content(ContentKind::PureNetManifest, bytes.clone()).unwrap();
    let program = content(ContentKind::PureNetProgram, bytes).unwrap();
    assert_ne!(content_id(&manifest).unwrap(), content_id(&program).unwrap());
    assert!(matches!(content(ContentKind::PureNetProgram, vec![0; MAX_CONTENT_BYTES + 1]), Err(Error::Bound)));
    let mut bad = program;
    bad.manifest.length -= 1;
    assert!(content_id(&bad).is_err());
}

// This adapter exercises the report carrier using a real Wasmtime compile
// failure. It makes no claim to execute or emulate the native evaluator.
struct ReportingComponentRuntime(PureComponentRuntime);
#[async_trait::async_trait]
impl ExecutionPort for ReportingComponentRuntime {
    async fn run(
        &self,
        request: ExecutionRequest,
        workload: MaterializedWorkload,
        inputs: Vec<Vec<u8>>,
    ) -> Result<ExecutionOutput, Error> {
        let request_ref = CanonicalIdentity.request(&request)?;
        let mut output = self.0.run(request, workload, inputs).await?;
        if let Err(code) = output.output {
            output.report = Some(
                molten_orchestrator_codec::encode("actual-component-failure-report.v1", &(request_ref, code))
                    .map_err(|e| Error::Invalid(e.to_string()))?
                    .canonical,
            );
        }
        Ok(output)
    }
    fn cancel(&self, id: Id) -> Result<(), Error> {
        self.0.cancel(id)
    }
}

struct InProcessWorker(Arc<WorkerService>);
#[async_trait::async_trait]
impl WorkerClient for InProcessWorker {
    async fn info(&self, _: Id) -> Result<WorkerInfo, Error> {
        self.0.info()
    }
    async fn put(&self, _: Id, id: Id, content: Content) -> Result<(), Error> {
        if self.0.put_content(&content)? != id {
            return Err(Error::Denied);
        }
        Ok(())
    }
    async fn get(&self, _: Id, id: Id) -> Result<Content, Error> {
        self.0.get_content(id)
    }
    async fn start(&self, request: ExecutionRequest) -> Result<WorkerObservation, Error> {
        let (_, ticket) = self.0.start(request.clone())?;
        if let Some(ticket) = ticket {
            self.0.execute(ticket).await?;
        }
        self.0.observe(&request)
    }
    async fn observe(&self, request: &ExecutionRequest) -> Result<WorkerObservation, Error> {
        self.0.observe(request)
    }
    async fn cancel(&self, request: &ExecutionRequest) -> Result<WorkerObservation, Error> {
        self.0.cancel(request)
    }
}

#[tokio::test]
async fn stopped_failure_receipt_is_copied_before_controller_terminal_and_unknown_retains_capacity() {
    for unknown_report_copy in [false, true] {
        let worker_temp = tempfile::tempdir().unwrap();
        let worker_root = CapabilityRoot::open(worker_temp.path()).unwrap();
        let policy = policy(PROFILE);
        let policy_ref = install(&worker_root, &policy);
        let disk = Disk::open(&worker_root).unwrap();
        let journal = RedbWorkerJournal::open(
            disk.clone(),
            WorkerState::empty(id(1), id(4), 1, policy_ref, resources()).unwrap(),
        )
        .unwrap();
        let worker_content = RedbContent::new(disk.clone());
        let worker = Arc::new(WorkerService::new(WorkerPorts {
            journal,
            content: worker_content.clone(),
            identity: Arc::new(CanonicalIdentity),
            admission: Arc::new(FileAdmission::new(worker_root.clone(), policy_ref).unwrap()),
            execution: Arc::new(ReportingComponentRuntime(PureComponentRuntime::for_worker(disk))),
            observations: Arc::new(Counters::default()),
        }));
        worker.recover_after_exclusive_open().unwrap();

        let controller_temp = tempfile::tempdir().unwrap();
        let controller_root = CapabilityRoot::open(controller_temp.path()).unwrap();
        assert_eq!(install(&controller_root, &policy), policy_ref);
        let disk = Disk::open(&controller_root).unwrap();
        let store = RedbControllerStore::open(disk.clone(), State::empty(id(1), policy_ref, 1, 3).unwrap()).unwrap();
        let controller_content = RedbContent::new(disk);
        let content_port: Arc<dyn ContentPort> = if unknown_report_copy {
            Arc::new(UnknownDataContent {
                inner: controller_content.clone(),
            })
        } else {
            controller_content.clone()
        };
        let controller = Controller::new(ControllerPorts {
            store: store.clone(),
            content: content_port,
            identity: Arc::new(CanonicalIdentity),
            placement: Arc::new(FirstFitPlacement),
            admission: Arc::new(FileAdmission::new(controller_root.clone(), policy_ref).unwrap()),
            workers: Arc::new(InProcessWorker(worker.clone())),
            observations: Arc::new(Counters::default()),
        });
        controller
            .register(Node {
                id: id(4),
                capacity: resources(),
                labels: Default::default(),
                draining: false,
                enabled: true,
                last_seen: None,
                boot: 0,
            })
            .unwrap();
        let malformed = content(ContentKind::Component, b"not a component".to_vec()).unwrap();
        let artifact = controller.put_content(&malformed).unwrap();
        let mut task = task(artifact);
        task.max_attempts = 1;
        let run = controller
            .submit("failed-report".into(), JobSpec {
                name: "failed-report".into(),
                tasks: vec![task],
            })
            .unwrap();
        controller.tick().await.unwrap();
        let worker_state = worker.state().unwrap();
        let record = worker_state.records.values().next().unwrap();
        let WorkerPhase::Terminal(result) = record.phase else {
            panic!("worker did not stop")
        };
        assert_eq!(result.outcome, ExecutionOutcome::Failed {
            code: FailureCode::Compile
        });
        let report = result.report.expect("stopped failure receipt");
        let expected = molten_orchestrator_codec::encode(
            "actual-component-failure-report.v1",
            &(result.request_ref, FailureCode::Compile),
        )
        .unwrap();
        assert_eq!(worker_content.get(report).unwrap().bytes, expected.canonical);
        assert_eq!(controller_content.get(report).unwrap().bytes, expected.canonical);
        let state = controller.state().unwrap();
        if unknown_report_copy {
            assert_eq!(state.usage(id(4)).unwrap(), resources());
            assert!(!matches!(state.attempts.values().next().unwrap().phase, AttemptPhase::Terminal { .. }));
            assert_ne!(state.runs[&run].phase(), RunPhase::Failed);
        } else {
            assert_eq!(state.usage(id(4)).unwrap(), Resources::default());
            assert_eq!(state.runs[&run].phase(), RunPhase::Failed);
            assert!(matches!(state.attempts.values().next().unwrap().phase,
                AttemptPhase::Terminal { result: accepted, .. } if accepted == result));
        }
        // Revocation cannot prevent copying/reconciling a stopped failure.
        controller_root.replace("revoked.json", &serde_json::to_vec(&[artifact]).unwrap()).unwrap();
        controller.tick().await.unwrap();
        assert_eq!(worker.state().unwrap(), worker_state);
    }
}

struct SubstitutedContent {
    inner: Arc<RedbContent>,
    requested: Id,
    substituted: Content,
}
impl ContentPort for SubstitutedContent {
    fn get(&self, id: Id) -> Result<Content, Error> {
        if id == self.requested {
            Ok(self.substituted.clone())
        } else {
            self.inner.get(id)
        }
    }
    fn put(&self, content: &Content) -> Result<Id, Error> {
        self.inner.put(content)
    }
    fn make_data(&self, bytes: Vec<u8>) -> Result<Content, Error> {
        self.inner.make_data(bytes)
    }
}

#[tokio::test]
async fn native_materialization_rejects_wrong_kind_or_same_kind_identity_substitution_before_begin() {
    for case in 0..4 {
        let temp = tempfile::tempdir().unwrap();
        let root = CapabilityRoot::open(temp.path()).unwrap();
        let disk = Disk::open(&root).unwrap();
        let content_port = RedbContent::new(disk.clone());
        let manifest_kind = if case == 0 {
            ContentKind::Data
        } else {
            ContentKind::PureNetManifest
        };
        let program_kind = if case == 1 {
            ContentKind::Component
        } else {
            ContentKind::PureNetProgram
        };
        let manifest = content(manifest_kind, b"manifest".to_vec()).unwrap();
        let artifact = content(program_kind, b"program".to_vec()).unwrap();
        let manifest_id = content_port.put(&manifest).unwrap();
        let artifact_id = content_port.put(&artifact).unwrap();
        let mut policy = policy(PURE_NET_PROFILE);
        policy.allowed_pure_net_manifests.insert(manifest_id);
        policy.allowed_pure_net_programs.insert(artifact_id);
        let policy_ref = install(&root, &policy);
        let content: Arc<dyn ContentPort> = if case >= 2 {
            let (requested, kind) = if case == 2 {
                (manifest_id, ContentKind::PureNetManifest)
            } else {
                (artifact_id, ContentKind::PureNetProgram)
            };
            Arc::new(SubstitutedContent {
                inner: content_port,
                requested,
                substituted: molten_orchestrator_adapters::content(kind, b"different exact bytes".to_vec()).unwrap(),
            })
        } else {
            content_port
        };
        let journal = RedbWorkerJournal::open(
            disk.clone(),
            WorkerState::empty(id(1), id(4), 1, policy_ref, resources()).unwrap(),
        )
        .unwrap();
        let worker = WorkerService::new(WorkerPorts {
            journal,
            content,
            identity: Arc::new(CanonicalIdentity),
            admission: Arc::new(FileAdmission::new(root, policy_ref).unwrap()),
            execution: Arc::new(PureComponentRuntime::for_worker(disk)),
            observations: Arc::new(Counters::default()),
        });
        worker.recover_after_exclusive_open().unwrap();
        let mut r = request(artifact_id);
        r.policy_ref = policy_ref;
        r.workload = Workload::PureNet {
            manifest: manifest_id,
            artifact: artifact_id,
        };
        let (_, ticket) = worker.start(r.clone()).unwrap();
        worker.execute(ticket.unwrap()).await.unwrap();
        let state = worker.state().unwrap();
        assert!(matches!(
            state.records[&r.fence.assignment].phase,
            WorkerPhase::Terminal(ExecutionResult {
                outcome: ExecutionOutcome::RejectedBeforeStart,
                report: None,
                ..
            })
        ));
        assert_eq!(state.usage().unwrap(), Resources::default());
        assert!(worker.start(r).unwrap().1.is_none());
    }
}

#[tokio::test]
async fn stopped_report_over_task_bound_cannot_become_a_terminal_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let root = CapabilityRoot::open(temp.path()).unwrap();
    let policy_ref = install(&root, &policy(PROFILE));
    let disk = Disk::open(&root).unwrap();
    let content_port = RedbContent::new(disk.clone());
    let artifact = content_port.put(&content(ContentKind::Component, b"not a component".to_vec()).unwrap()).unwrap();
    let journal =
        RedbWorkerJournal::open(disk.clone(), WorkerState::empty(id(1), id(4), 1, policy_ref, resources()).unwrap())
            .unwrap();
    let worker = WorkerService::new(WorkerPorts {
        journal,
        content: content_port,
        identity: Arc::new(CanonicalIdentity),
        admission: Arc::new(FileAdmission::new(root, policy_ref).unwrap()),
        execution: Arc::new(ReportingComponentRuntime(PureComponentRuntime::for_worker(disk))),
        observations: Arc::new(Counters::default()),
    });
    worker.recover_after_exclusive_open().unwrap();
    let mut request = request(artifact);
    request.policy_ref = policy_ref;
    request.max_output_bytes = 1;
    let (_, ticket) = worker.start(request.clone()).unwrap();
    worker.execute(ticket.unwrap()).await.unwrap();
    assert!(matches!(
        worker.state().unwrap().records[&request.fence.assignment].phase,
        WorkerPhase::Terminal(ExecutionResult {
            outcome: ExecutionOutcome::Failed {
                code: FailureCode::OutputLimit
            },
            report: None,
            stopped: true,
            ..
        })
    ));
}

struct CancelAtBoundary {
    journal: Arc<RedbWorkerJournal>,
    boundary: &'static str,
}
impl ObservabilityPort for CancelAtBoundary {
    fn record(&self, event: Diagnostic) {
        if event != Diagnostic::Boundary(self.boundary) {
            return;
        }
        let old = self.journal.load().unwrap();
        let record = old.records.values().next().unwrap();
        let next = worker_apply(&old, &WorkerCommand::Cancel {
            request: record.request.clone(),
            request_ref: record.request_ref,
        })
        .unwrap();
        assert_eq!(self.journal.compare_commit(&old, &next).unwrap(), Commit::Applied);
    }
}

#[tokio::test]
async fn actual_component_cancel_between_calculation_storage_and_terminal_never_publishes_success() {
    for boundary in [
        "after-worker-calculation",
        "before-worker-output-persist",
        "after-worker-output-persist",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = CapabilityRoot::open(temp.path()).unwrap();
        let policy_ref = install(&root, &policy(PROFILE));
        let disk = Disk::open(&root).unwrap();
        let journal = RedbWorkerJournal::open(
            disk.clone(),
            WorkerState::empty(id(1), id(4), 1, policy_ref, resources()).unwrap(),
        )
        .unwrap();
        let content = RedbContent::new(disk.clone());
        let artifact = content.put(&fixture()).unwrap();
        let worker = WorkerService::new(WorkerPorts {
            journal: journal.clone(),
            content: content.clone(),
            identity: Arc::new(CanonicalIdentity),
            admission: Arc::new(FileAdmission::new(root, policy_ref).unwrap()),
            execution: Arc::new(PureComponentRuntime::for_worker(disk)),
            observations: Arc::new(CancelAtBoundary { journal, boundary }),
        });
        worker.recover_after_exclusive_open().unwrap();
        let mut request = request(artifact);
        request.policy_ref = policy_ref;
        let (_, ticket) = worker.start(request.clone()).unwrap();
        worker.execute(ticket.unwrap()).await.unwrap();
        let state = worker.state().unwrap();
        assert!(matches!(
            state.records[&request.fence.assignment].phase,
            WorkerPhase::Terminal(ExecutionResult {
                outcome: ExecutionOutcome::Cancelled,
                stopped: true,
                ..
            })
        ));
        assert_eq!(state.usage().unwrap(), Resources::default());
        let output = content_id(&content.make_data(b"ok".to_vec()).unwrap()).unwrap();
        assert_eq!(content.get(output).is_ok(), boundary == "after-worker-output-persist");
        assert!(worker.start(request).unwrap().1.is_none());
    }
}
