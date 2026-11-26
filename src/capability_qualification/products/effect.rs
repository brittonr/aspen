//! Post-effect uncertainty over a live bounded child process. The effect runs as
//! a distinct OS process (`/bin/sh`) through the production live execution
//! adapter; the native work boundary is the production capability-rooted
//! service host, whose durable work record must stay Unknown and must never be
//! requeued once that process effect may have happened.

use crate::fabric_execution::Adapter;
use crate::fabric_execution::BOUNDED_EXEC_LICENSE;
use crate::fabric_execution::BOUNDED_EXEC_PACKAGE;
use crate::fabric_execution::BOUNDED_EXEC_REPOSITORY;
use crate::fabric_execution::BOUNDED_EXEC_REVISION;
use crate::fabric_execution::EXECUTION_PROFILE_SCHEMA;
use crate::fabric_execution::EXECUTION_REQUEST_SCHEMA;
use crate::fabric_execution::EnvironmentEntry;
use crate::fabric_execution::EnvironmentValueClass;
use crate::fabric_execution::ExecutableResolutionMode;
use crate::fabric_execution::ExecutionAuthorityFacts;
use crate::fabric_execution::ExecutionEnvironmentMode;
use crate::fabric_execution::ExecutionFabricPort;
use crate::fabric_execution::ExecutionInvocationMode;
use crate::fabric_execution::ExecutionOutputPublisher;
use crate::fabric_execution::ExecutionPlatform;
use crate::fabric_execution::ExecutionProfileDescriptor;
use crate::fabric_execution::ExecutionProfileKind;
use crate::fabric_execution::ExecutionRequest;
use crate::fabric_execution::ExecutionRequestLimits;
use crate::fabric_execution::ExecutionResourceGrant;
use crate::fabric_execution::ExecutionTerminationScope;
use crate::fabric_execution::OutputPublicationError;
use crate::fabric_execution::PortFailure;
use crate::fabric_execution::PortFailureKind;
use crate::fabric_execution::PublishedStream;
use crate::fabric_execution::REQUIRED_EXECUTION_NON_CLAIMS;
use crate::fabric_execution::ReconciliationStatus;
use crate::fabric_execution::ResolvedContext;
use crate::fabric_execution::RetainedStream;
use crate::fabric_execution::WorkspaceMode;
use crate::node_service_hosting::AuthorityPort;
use crate::node_service_hosting::DeliveryInput;
use crate::node_service_hosting::Host;
use crate::node_service_hosting::OperationRequest;
use crate::node_service_hosting::RuntimeAction;
use crate::node_service_hosting::RuntimeDelivery;
use crate::node_service_hosting::RuntimePort;
use crate::system_extension::Generation;
use crate::system_extension::HealthState;
use crate::system_extension::Incarnation;
use crate::system_extension::LifecyclePhase;
use crate::system_extension::LifecycleState;
use crate::system_extension::NATIVE_INGRESS_SCHEMA;
use crate::system_extension::NATIVE_INSTANCE_STATE_SCHEMA;
use crate::system_extension::NativeCallbackValue;
use crate::system_extension::NativeIngressEnvelope;
use crate::system_extension::NativeInstanceRecord;
use crate::system_extension::RecoveryGroupDeclaration;
use crate::system_extension::RecoveryGroupMember;
use crate::system_extension::ResourceUsage;
use crate::system_extension::RestartClass;
use crate::system_extension::WorkClass;
use crate::system_extension::WorkPresentation;

/// The admitted effect executable. A real distinct OS process, never a shell
/// builtin inside this process.
const EFFECT_PROGRAM: &str = "/bin/sh";
const MISSING_PROGRAM: &str = "/nonexistent/aspen-qualification-effect";
/// Created by the child in its workspace before it exits without reading its
/// input: proof that a distinct process started and its effect may have happened.
const CHILD_STARTED_MARKER: &str = "effect-started";
const EFFECT_SCRIPT: &str = ": > effect-started; exit 0";
const GENERATION: u64 = 1;
const TIMEOUT_MS: u64 = 5_000;
const POLL_INTERVAL_MS: u64 = 5;
const TEARDOWN_TIMEOUT_MS: u64 = 1_000;
const STREAM_BYTES: u64 = 4_096;
/// Exceeds the Linux pipe capacity, so a child that exits without reading
/// leaves delivery of its input, and therefore its effect, unknown.
const UNCONSUMED_INPUT_BYTES: u64 = 262_144;
const UNITS: u64 = 1;
const DEADLINE_TICKS: u64 = 10_000;
const SERVICE_ID: &str = "qualification-effect-service";
const EXTENSION_ID: &str = "qualification-effect-extension";
const GROUP_ID: &str = "qualification-effect-group";
const WORK_PAYLOAD: &[u8] = b"qualification durable effect work";
const INSTALL_TICK: u64 = 1;
const SUBMIT_TICK: u64 = 2;
const CLAIM_TICK: u64 = 3;
const DELIVER_TICK: u64 = 4;
const RETRY_TICK: u64 = 5;
const REOPEN_TICK: u64 = 6;

pub(super) fn run(scenario: super::super::Scenario) -> crate::error::Result<super::Evidence> {
    match scenario {
        super::super::Scenario::ExecutionUnknownAfterStart => execution_unknown_after_start(),
        super::super::Scenario::ExecutionPreStartDefinite => execution_prestart_definite(),
        super::super::Scenario::NativeWorkUnknownAfterEffect => native_work_unknown_after_effect(),
        super::super::Scenario::NativeUncertainWorkNotReadmitted => native_uncertain_work_not_readmitted(),
        _ => Err(super::super::invalid("scenario is not an effect-uncertainty scenario")),
    }
}

// r[impl molten.fabric_execution.uncertainty]
fn execution_unknown_after_start() -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let mut effect = LiveEffect::new()?;
    evidence.runtime(effect.profile_value());
    evidence.runtime(effect.request.value.clone());
    let failure = effect.execute(EFFECT_PROGRAM, &effect.resolved(EFFECT_PROGRAM))?;
    evidence.check("effect-child-process-started", effect.child_started());
    evidence.check("post-start-failure-classified-unknown", failure.kind == PortFailureKind::UnknownAfterStart);
    evidence.check("no-terminal-receipt-claimed", failure.receipt.is_none());
    evidence.check("no-process-completion-claimed", failure.process_observation.is_none());
    evidence.check("no-output-published", effect.adapter.publisher().published == 0);
    evidence
        .check("reconciliation-required", effect.reconcile() == ReconciliationStatus::UnknownRequiresReconciliation);
    evidence.runtime(failure_value(&failure));
    Ok(evidence)
}

// r[impl molten.fabric_execution.uncertainty]
fn execution_prestart_definite() -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let mut effect = LiveEffect::new()?;
    evidence.runtime(effect.request.value.clone());

    let mut crossed = effect.resolved(EFFECT_PROGRAM);
    crossed.executable_identity_ref = reference("qualification effect substituted executable measurement");
    let denied = effect.execute(EFFECT_PROGRAM, &crossed)?;
    evidence.check("crossed-resolution-denied-before-start", denied.kind == PortFailureKind::ResolutionDenied);
    evidence.check("crossed-resolution-not-recorded", effect.reconcile() == ReconciliationStatus::NotFound);
    evidence.check("crossed-resolution-started-no-child", !effect.child_started());
    evidence.runtime(failure_value(&denied));

    let missing = effect.execute(MISSING_PROGRAM, &effect.resolved(MISSING_PROGRAM))?;
    evidence.check("spawn-failure-rejected-before-start", missing.kind == PortFailureKind::RejectedBeforeStart);
    evidence.check(
        "spawn-failure-definite-not-unknown",
        effect.reconcile() == ReconciliationStatus::DefinitePreStartFailure,
    );
    evidence.check("definite-failure-has-no-receipt", missing.receipt.is_none());
    evidence.check("no-output-published", effect.adapter.publisher().published == 0);
    evidence.check("spawn-failure-started-no-child", !effect.child_started());
    evidence.runtime(failure_value(&missing));
    Ok(evidence)
}

// r[impl molten.system_extension.native_effect_uncertainty]
fn native_work_unknown_after_effect() -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let scratch = super::scratch("native-effect")?;
    let root = crate::node_state::DirectoryRoot::open_existing(scratch.path())?;
    root.create_layout()?;
    let attempts = std::rc::Rc::new(std::cell::Cell::new(0_u64));
    let mut host = Host::open(&root, Authority, EffectRuntime::new(attempts.clone())?)?;
    let delivered = deliver_effect_work(&mut host)?;
    evidence.check("effect-delivery-reported-uncertain", is_uncertain(&delivered));
    if let Err(error) = &delivered {
        evidence.runtime(super::error_class(error));
    }
    evidence.check("process-effect-child-started-once", attempts.get() == 1);
    let status = host.status(DELIVER_TICK)?;
    evidence.check("durable-work-unknown", work_state(&status)? == "unknown");
    evidence.runtime(status);
    drop(host);

    let mut reopened = Host::open(&root, Authority, EffectRuntime::new(attempts.clone())?)?;
    let status = reopened.status(REOPEN_TICK)?;
    evidence.check("reopened-work-still-unknown", work_state(&status)? == "unknown");
    evidence.check("reopen-does-not-requeue-effect", attempts.get() == 1);
    evidence.runtime(status);
    Ok(evidence)
}

// r[impl molten.system_extension.native_effect_uncertainty]
fn native_uncertain_work_not_readmitted() -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let scratch = super::scratch("native-effect-readmit")?;
    let root = crate::node_state::DirectoryRoot::open_existing(scratch.path())?;
    root.create_layout()?;
    let attempts = std::rc::Rc::new(std::cell::Cell::new(0_u64));
    let mut host = Host::open(&root, Authority, EffectRuntime::new(attempts.clone())?)?;
    let delivered = deliver_effect_work(&mut host)?;
    evidence.check("effect-delivery-reported-uncertain", is_uncertain(&delivered));
    evidence.check("process-effect-child-started-once", attempts.get() == 1);

    let readmit = host.readmit(SERVICE_ID, &work_ref(), RETRY_TICK);
    evidence.check("uncertain-work-readmission-denied", readmit.is_err());
    let retry = host.deliver(&delivery_input(Some(reference("qualification replayed delivery claim"))));
    evidence.check("uncertain-work-redelivery-denied", retry.is_err());
    evidence.check("uncertain-effect-not-repeated", attempts.get() == 1);
    drop(host);

    let mut reopened = Host::open(&root, Authority, EffectRuntime::new(attempts.clone())?)?;
    evidence.check(
        "reopened-uncertain-work-readmission-denied",
        reopened.readmit(SERVICE_ID, &work_ref(), REOPEN_TICK).is_err(),
    );
    evidence.check("reopen-does-not-requeue-effect", attempts.get() == 1);
    let status = reopened.status(REOPEN_TICK)?;
    evidence.check("durable-work-unknown", work_state(&status)? == "unknown");
    evidence.runtime(status);
    Ok(evidence)
}

/// Install the member, submit durable work, claim it and deliver it. The
/// delivery runs one real child-process effect whose outcome is unknown.
fn deliver_effect_work(host: &mut Host<Authority, EffectRuntime>) -> crate::error::Result<crate::error::Result<()>> {
    host.install(
        RecoveryGroupDeclaration {
            group_id: GROUP_ID.to_string(),
            members: vec![RecoveryGroupMember {
                member_id: SERVICE_ID.to_string(),
                restart_class: RestartClass::Permanent,
                after: Vec::new(),
            }],
            window_ticks: 100,
            backoff_ticks: 1,
            max_restarts_per_member: 1,
            max_restarts_per_group: 1,
            elapsed_source_ref: reference("qualification effect logical clock"),
        },
        INSTALL_TICK,
    )?;
    host.submit(SERVICE_ID, presentation(), payload(), SUBMIT_TICK)?;
    let claim = host.readmit(SERVICE_ID, &work_ref(), CLAIM_TICK)?;
    Ok(host.deliver(&delivery_input(Some(claim.claim_ref))).map(|_| ()))
}

fn delivery_input(claim_ref: Option<String>) -> DeliveryInput {
    DeliveryInput {
        member: SERVICE_ID.to_string(),
        presentation: presentation(),
        claim_ref,
        ingress: NativeIngressEnvelope {
            schema: NATIVE_INGRESS_SCHEMA.to_string(),
            request_ref: reference("qualification effect ingress request"),
            endpoint_ref: reference("qualification effect endpoint"),
            peer_ref: reference("qualification effect peer"),
            service_id: SERVICE_ID.to_string(),
            manifest_ref: reference("qualification effect manifest"),
            generation: Generation::new(GENERATION),
            authority_ref: reference("qualification effect ingress authority"),
            policy_ref: reference("qualification effect policy"),
            resource_ref: reference("qualification effect resources"),
            transport_profile_ref: reference("qualification effect transport profile"),
            alpn: "aspen/qualification-effect/1".to_string(),
            framing: "length-prefixed".to_string(),
            payload: payload(),
            accounted_bytes: WORK_PAYLOAD.len() as u64,
        },
        logical_tick: DELIVER_TICK,
    }
}

fn presentation() -> WorkPresentation {
    WorkPresentation {
        work_ref: work_ref(),
        class: WorkClass::Durable,
        generation: Generation::new(GENERATION),
        incarnation: Incarnation::new(0),
    }
}

fn payload() -> NativeCallbackValue {
    NativeCallbackValue {
        value_ref: crate::preserves_rail::content_ref_from_bytes(WORK_PAYLOAD),
        bytes: WORK_PAYLOAD.to_vec(),
    }
}

/// Uncertainty is the typed `UncertainExternalOutcome` class, never any error.
fn is_uncertain(delivered: &crate::error::Result<()>) -> bool {
    matches!(
        delivered,
        Err(crate::error::Failure::Classified {
            class: crate::error::FailureClass::UncertainExternalOutcome,
            ..
        })
    )
}

fn work_ref() -> String {
    reference("qualification durable effect work item")
}

/// The persisted work state from the host's own canonical status readback.
fn work_state(status: &preserves::IOValue) -> crate::error::Result<String> {
    let text = crate::preserves_rail::to_text(status)?;
    let marker = format!("<host-work-status-v1 \"{SERVICE_ID}\" \"{}\" \"", work_ref());
    let start = text.find(&marker).ok_or_else(|| super::super::invalid("host status lacks the retained work"))?;
    let rest = &text[start + marker.len()..];
    let end = rest.find('"').ok_or_else(|| super::super::invalid("malformed host work status"))?;
    Ok(rest[..end].to_string())
}

/// Producer-owned admission port: it admits the fixed operations by binding a
/// canonical receipt over the exact request the host asks it to admit.
struct Authority;

impl AuthorityPort for Authority {
    fn admit(&mut self, request: &OperationRequest) -> crate::error::Result<String> {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("qualification-host-admission-v1", vec![
            crate::preserves_rail::string(&request.operation),
            crate::preserves_rail::string(&request.subject),
            crate::preserves_rail::u64_value(request.logical_tick),
            crate::preserves_rail::string(&request.state_ref),
            crate::preserves_rail::string(&request.input_ref),
        ]))
    }
}

/// Runtime port whose native work delivery performs one real child-process
/// effect through the live bounded execution adapter.
struct EffectRuntime {
    instance: NativeInstanceRecord,
    effect: LiveEffect,
    attempts: std::rc::Rc<std::cell::Cell<u64>>,
}

impl EffectRuntime {
    fn new(attempts: std::rc::Rc<std::cell::Cell<u64>>) -> crate::error::Result<Self> {
        Ok(Self {
            instance: NativeInstanceRecord {
                schema: NATIVE_INSTANCE_STATE_SCHEMA.to_string(),
                instance_id: reference("qualification effect instance"),
                extension_id: EXTENSION_ID.to_string(),
                service_id: SERVICE_ID.to_string(),
                manifest_ref: reference("qualification effect manifest"),
                executable_ref: reference("qualification effect executable"),
                profile_ref: reference("qualification effect native profile"),
                state_schema_ref: reference("qualification effect state schema"),
                lifecycle: LifecycleState {
                    generation: Generation::new(GENERATION),
                    incarnation: Incarnation::new(0),
                    phase: LifecyclePhase::Admitted,
                    restart_attempts: 0,
                    health: HealthState::Healthy,
                    checkpoint_ref: None,
                },
                usage: ResourceUsage::default(),
                callback_sequence: 0,
                event_sequence: 0,
                state_ref: None,
                checkpoint_ref: None,
                unresolved: Vec::new(),
                completed_operations: Vec::new(),
                completed_operation_refs: Vec::new(),
                evidence_refs: Vec::new(),
                is_accepting_ingress: false,
            },
            effect: LiveEffect::new()?,
            attempts,
        })
    }
}

impl RuntimePort for EffectRuntime {
    fn instance(&self, member: &str) -> crate::error::Result<NativeInstanceRecord> {
        if member != SERVICE_ID {
            return Err(super::super::invalid("unknown qualification effect member"));
        }
        Ok(self.instance.clone())
    }

    fn apply(
        &mut self,
        member: &str,
        action: &RuntimeAction,
        _logical_tick: u64,
    ) -> crate::error::Result<NativeInstanceRecord> {
        if member != SERVICE_ID {
            return Err(super::super::invalid("unknown qualification effect member"));
        }
        match action {
            RuntimeAction::Start => {
                self.instance.lifecycle.phase = LifecyclePhase::Running;
                self.instance.is_accepting_ingress = true;
                self.instance.state_ref = Some(reference("qualification effect started state"));
            }
            RuntimeAction::Checkpoint => {
                let checkpoint = reference("qualification effect checkpoint");
                self.instance.lifecycle.checkpoint_ref = Some(checkpoint.clone());
                self.instance.checkpoint_ref = Some(checkpoint);
            }
            _ => return Err(super::super::invalid("qualification effect runtime supports only start/checkpoint")),
        }
        self.instance.callback_sequence = self.instance.callback_sequence.saturating_add(1);
        Ok(self.instance.clone())
    }

    fn deliver(
        &mut self,
        member: &str,
        _ingress: &NativeIngressEnvelope,
        _logical_tick: u64,
    ) -> crate::error::Result<RuntimeDelivery> {
        if member != SERVICE_ID {
            return Err(super::super::invalid("unknown qualification effect member"));
        }
        let failure = self.effect.execute(EFFECT_PROGRAM, &self.effect.resolved(EFFECT_PROGRAM))?;
        // Count only effects whose child process actually started.
        if self.effect.take_child_started()? {
            self.attempts.set(self.attempts.get().saturating_add(1));
        }
        if failure.kind == PortFailureKind::UnknownAfterStart
            && self.effect.reconcile() == ReconciliationStatus::UnknownRequiresReconciliation
        {
            return Err(crate::error::Failure::classified(
                crate::error::FailureClass::UncertainExternalOutcome,
                "native work process effect started without definitive completion",
            ));
        }
        Err(super::super::invalid("qualification effect did not reach its post-start boundary"))
    }

    fn migration(&mut self, _member: &str, _migration_ref: &str) -> crate::error::Result<preserves::IOValue> {
        Err(super::super::invalid("qualification effect runtime has no migrations"))
    }
}

/// One admitted live execution request whose child never consumes its
/// oversized input, so a started effect has no definitive completion.
struct LiveEffect {
    adapter: Adapter<RetainedPublisher>,
    request: crate::fabric_execution::Request,
    /// The child's working directory; the admitted script creates
    /// `CHILD_STARTED_MARKER` there as its first (externally visible) effect.
    workspace: tempfile::TempDir,
}

impl LiveEffect {
    fn new() -> crate::error::Result<Self> {
        let descriptor = descriptor();
        let profile = crate::fabric_execution::admit_profile(&descriptor)?;
        let request = execution_request();
        let authority = authority(&request);
        let request = crate::fabric_execution::admit_request(&profile, &request, &authority, resources(), GENERATION)?;
        let adapter = Adapter::new(profile, RetainedPublisher::default())
            .map_err(|_| super::super::invalid("live execution adapter requires the live profile"))?;
        Ok(Self {
            adapter,
            request,
            workspace: super::scratch("effect-workspace")?,
        })
    }

    /// Whether a child process actually ran the admitted script.
    fn child_started(&self) -> bool {
        self.workspace.path().join(CHILD_STARTED_MARKER).is_file()
    }

    /// Consume the start marker so a later attempt must start its own child.
    fn take_child_started(&self) -> crate::error::Result<bool> {
        let marker = self.workspace.path().join(CHILD_STARTED_MARKER);
        if !marker.is_file() {
            return Ok(false);
        }
        std::fs::remove_file(marker).map_err(crate::error::Failure::from)?;
        Ok(true)
    }

    fn profile_value(&self) -> preserves::IOValue {
        self.adapter.profile().value.clone()
    }

    fn resolved(&self, program: &str) -> ResolvedContext {
        let plan = &self.request.plan.resolution;
        ResolvedContext {
            executable_path: std::path::PathBuf::from(program),
            executable_artifact_ref: plan.executable_artifact_ref.clone(),
            executable_identity_ref: plan.executable_identity_ref.clone(),
            workspace_path: self.workspace.path().to_path_buf(),
            workspace_ref: plan.workspace_ref.clone(),
            stdin_ref: plan.stdin_ref.clone(),
            stdin_bytes: Some(vec![0_u8; UNCONSUMED_INPUT_BYTES as usize]),
        }
    }

    /// Run the admitted request; a completed run is not the expected outcome here.
    fn execute(&mut self, program: &str, resolved: &ResolvedContext) -> crate::error::Result<Box<PortFailure>> {
        debug_assert!(resolved.executable_path == std::path::Path::new(program));
        match self.adapter.execute(&self.request, resolved, None) {
            Ok(_) => Err(super::super::invalid("live effect completed definitively; uncertainty boundary not reached")),
            Err(failure) => Ok(failure),
        }
    }

    fn reconcile(&self) -> ReconciliationStatus {
        self.adapter.reconcile(&self.request.plan.request.operation_ref, GENERATION)
    }
}

/// Retains published streams in memory; the scenarios assert nothing is
/// published for an uncertain or pre-start-failed effect.
#[derive(Debug, Default)]
struct RetainedPublisher {
    published: u64,
}

impl ExecutionOutputPublisher for RetainedPublisher {
    fn publish(
        &mut self,
        operation_ref: &str,
        stream: &RetainedStream,
    ) -> Result<PublishedStream, OutputPublicationError> {
        self.published = self.published.saturating_add(1);
        let content_ref = crate::preserves_rail::content_ref_from_bytes(&stream.retained_bytes);
        let material = format!("{operation_ref}\0{}\0{content_ref}", stream.role);
        Ok(PublishedStream {
            content_ref,
            publication_receipt_ref: crate::preserves_rail::content_ref_from_bytes(material.as_bytes()),
        })
    }
}

/// Canonical projection of the adapter's typed port failure (kind, code and
/// operation binding). Adapter detail text is excluded.
fn failure_value(failure: &PortFailure) -> preserves::IOValue {
    crate::preserves_rail::record("execution-port-failure-v1", vec![
        crate::preserves_rail::string(format!("{:?}", failure.kind)),
        crate::preserves_rail::string(failure.diagnostic_code),
        crate::preserves_rail::string(&failure.operation_ref),
        crate::preserves_rail::bool_value(failure.process_observation.is_some()),
        crate::preserves_rail::bool_value(failure.receipt.is_some()),
    ])
}

fn descriptor() -> ExecutionProfileDescriptor {
    ExecutionProfileDescriptor {
        schema: EXECUTION_PROFILE_SCHEMA.to_string(),
        profile_id: "bounded-process-live-v1".to_string(),
        profile_ref: reference("qualification effect execution profile declaration"),
        kind: ExecutionProfileKind::LiveBoundedProcess,
        platform: ExecutionPlatform::UnixProcessGroup,
        supported_termination_scopes: vec![ExecutionTerminationScope::ProcessGroup],
        max_timeout_ms: TIMEOUT_MS,
        max_stdin_bytes: UNCONSUMED_INPUT_BYTES,
        max_stdout_bytes: STREAM_BYTES,
        max_stderr_bytes: STREAM_BYTES,
        max_poll_interval_ms: POLL_INTERVAL_MS,
        max_teardown_timeout_ms: TEARDOWN_TIMEOUT_MS,
        max_arguments: 8,
        max_argument_bytes: 256,
        max_environment_entries: 8,
        max_environment_name_bytes: 64,
        max_environment_value_bytes: 256,
        max_concurrency_units: UNITS,
        max_queue_units: UNITS,
        component_repository: BOUNDED_EXEC_REPOSITORY.to_string(),
        component_revision: BOUNDED_EXEC_REVISION.to_string(),
        component_license: BOUNDED_EXEC_LICENSE.to_string(),
        component_package: BOUNDED_EXEC_PACKAGE.to_string(),
        conformance_refs: vec![reference("qualification effect bounded-exec conformance")],
        fabric_non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        non_claims: REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
    }
}

fn execution_request() -> ExecutionRequest {
    ExecutionRequest {
        schema: EXECUTION_REQUEST_SCHEMA.to_string(),
        operation_ref: reference("qualification effect operation"),
        idempotency_ref: reference("qualification effect idempotency"),
        extension_id: EXTENSION_ID.to_string(),
        service_id: SERVICE_ID.to_string(),
        callback_ref: reference("qualification effect callback"),
        effect_ref: reference("qualification effect intent"),
        generation: GENERATION,
        profile_ref: reference("qualification effect execution profile declaration"),
        executable_artifact_ref: reference("qualification effect executable artifact"),
        executable_identity_ref: reference("qualification effect executable measurement"),
        arguments: vec!["-c".to_string(), EFFECT_SCRIPT.to_string()],
        environment: vec![EnvironmentEntry {
            name: "ASPEN_QUALIFICATION".to_string(),
            value: "effect".to_string(),
            value_class: EnvironmentValueClass::Public,
        }],
        environment_mode: ExecutionEnvironmentMode::Clear,
        invocation_mode: ExecutionInvocationMode::Direct,
        executable_resolution: ExecutableResolutionMode::ExactArtifact,
        workspace_ref: reference("qualification effect workspace"),
        workspace_mode: WorkspaceMode::CapabilityRoot,
        stdin_ref: Some(reference("qualification effect unconsumed input")),
        limits: ExecutionRequestLimits {
            timeout_ms: TIMEOUT_MS,
            stdin_max_bytes: UNCONSUMED_INPUT_BYTES,
            stdout_max_bytes: STREAM_BYTES,
            stderr_max_bytes: STREAM_BYTES,
            poll_interval_ms: POLL_INTERVAL_MS,
            teardown_timeout_ms: TEARDOWN_TIMEOUT_MS,
            concurrency_units: UNITS,
            queue_units: UNITS,
        },
        termination_scope: ExecutionTerminationScope::ProcessGroup,
        accepted_exit_codes: vec![0],
        reject_stdout_truncation: true,
        reject_stderr_truncation: true,
        authority_ref: reference("qualification effect authority"),
        resource_grant_ref: reference("qualification effect resource grant"),
    }
}

fn authority(request: &ExecutionRequest) -> ExecutionAuthorityFacts {
    ExecutionAuthorityFacts {
        authority_ref: request.authority_ref.clone(),
        executable_authority_ref: reference("qualification effect executable authority"),
        provenance_ref: reference("qualification effect provenance"),
        effect_admission_ref: reference("qualification effect admission"),
        workspace_authority_ref: reference("qualification effect workspace authority"),
        process_authority_ref: reference("qualification effect process authority"),
        resource_grant_ref: request.resource_grant_ref.clone(),
        policy_ref: reference("qualification effect policy"),
        executable_artifact_ref: request.executable_artifact_ref.clone(),
        executable_identity_ref: request.executable_identity_ref.clone(),
        workspace_ref: request.workspace_ref.clone(),
        operation_ref: request.operation_ref.clone(),
        extension_id: request.extension_id.clone(),
        service_id: request.service_id.clone(),
        generation: request.generation,
        profile_ref: request.profile_ref.clone(),
    }
}

const fn resources() -> ExecutionResourceGrant {
    ExecutionResourceGrant {
        memory_bytes: STREAM_BYTES * 4,
        storage_bytes: STREAM_BYTES * 4,
        diagnostic_bytes: STREAM_BYTES * 4,
        logical_deadline_ticks: DEADLINE_TICKS,
        concurrency_units: UNITS,
        queue_units: UNITS,
    }
}

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

#[cfg(test)]
mod tests {
    // Controls: the native-work invariants must not hold when no child process
    // started or when delivery fails with anything other than uncertainty.
    // r[verify molten.system_extension.native_effect_uncertainty]
    // r[verify molten.fabric_execution.uncertainty]
    #[test]
    fn no_started_child_or_untyped_failure_is_not_uncertain_effect_evidence() {
        let attempts = std::rc::Rc::new(std::cell::Cell::new(0_u64));
        let mut runtime = super::EffectRuntime::new(attempts.clone()).expect("effect runtime");
        let missing = runtime
            .effect
            .execute(super::MISSING_PROGRAM, &runtime.effect.resolved(super::MISSING_PROGRAM))
            .expect("pre-start failure");
        assert_eq!(missing.kind, super::PortFailureKind::RejectedBeforeStart);
        assert!(!runtime.effect.take_child_started().expect("marker probe"));
        assert_eq!(attempts.get(), 0);
        let started = runtime
            .effect
            .execute(super::EFFECT_PROGRAM, &runtime.effect.resolved(super::EFFECT_PROGRAM))
            .expect("post-start failure");
        assert_eq!(started.kind, super::PortFailureKind::UnknownAfterStart);
        assert!(runtime.effect.take_child_started().expect("marker probe"));
        assert!(!super::is_uncertain(&Err(crate::error::Failure::invalid_harness("untyped failure"))));
        assert!(!super::is_uncertain(&Ok(())));
        assert!(super::is_uncertain(&Err(crate::error::Failure::classified(
            crate::error::FailureClass::UncertainExternalOutcome,
            "uncertain",
        ))));
    }
}
