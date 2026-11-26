//! The production live bounded execution adapter running one admitted child
//! process whose oversized input is never consumed.

use crate::fabric_execution::ExecutionFabricPort;

/// One admitted live execution request whose child never consumes its
/// oversized input, so a started effect has no definitive completion.
pub(super) struct LiveEffect {
    adapter: crate::fabric_execution::Adapter<RetainedPublisher>,
    request: crate::fabric_execution::Request,
    /// The child's working directory; the admitted script creates
    /// `CHILD_STARTED_MARKER` there as its first (externally visible) effect.
    workspace: tempfile::TempDir,
}

impl LiveEffect {
    pub(super) fn new() -> crate::error::Result<Self> {
        let descriptor = descriptor();
        let profile = crate::fabric_execution::admit_profile(&descriptor)?;
        // The live adapter measures the resolved executable before it spawns
        // anything, so the admitted identity is the measurement of this host's
        // `/bin/sh`. That identity is host-specific and therefore never enters
        // the deterministic observation (only the adapter's typed outcomes do).
        let executable = std::fs::read(super::EFFECT_PROGRAM).map_err(crate::error::Failure::from)?;
        let request = execution_request(crate::preserves_rail::content_ref_from_bytes(&executable));
        let authority = authority(&request);
        let request =
            crate::fabric_execution::admit_request(&profile, &request, &authority, resources(), super::GENERATION)?;
        let adapter = crate::fabric_execution::Adapter::new(profile, RetainedPublisher::default())
            .map_err(|_| super::super::super::invalid("live execution adapter requires the live profile"))?;
        Ok(Self {
            adapter,
            request,
            workspace: super::super::scratch("effect-workspace")?,
        })
    }

    /// Whether a child process actually ran the admitted script.
    pub(super) fn child_started(&self) -> bool {
        self.workspace.path().join(super::CHILD_STARTED_MARKER).is_file()
    }

    /// Consume the start marker so a later attempt must start its own child.
    pub(super) fn take_child_started(&self) -> crate::error::Result<bool> {
        let marker = self.workspace.path().join(super::CHILD_STARTED_MARKER);
        if !marker.is_file() {
            return Ok(false);
        }
        std::fs::remove_file(marker).map_err(crate::error::Failure::from)?;
        Ok(true)
    }

    pub(super) fn profile_value(&self) -> preserves::IOValue {
        self.adapter.profile().value.clone()
    }

    /// Output streams the adapter published through its retained publisher.
    pub(super) fn published(&self) -> u64 {
        self.adapter.publisher().published
    }

    pub(super) fn resolved(&self, program: &str) -> crate::fabric_execution::ResolvedContext {
        let plan = &self.request.plan.resolution;
        crate::fabric_execution::ResolvedContext {
            executable_path: std::path::PathBuf::from(program),
            executable_artifact_ref: plan.executable_artifact_ref.clone(),
            executable_identity_ref: plan.executable_identity_ref.clone(),
            workspace_path: self.workspace.path().to_path_buf(),
            workspace_ref: plan.workspace_ref.clone(),
            stdin_ref: plan.stdin_ref.clone(),
            stdin_bytes: Some(unconsumed_input()),
        }
    }

    /// Run the admitted request; a completed run is not the expected outcome here.
    pub(super) fn execute(
        &mut self,
        program: &str,
        resolved: &crate::fabric_execution::ResolvedContext,
    ) -> crate::error::Result<Box<crate::fabric_execution::PortFailure>> {
        debug_assert!(resolved.executable_path == std::path::Path::new(program));
        match self.adapter.execute(&self.request, resolved, None) {
            Ok(_) => Err(super::super::super::invalid(
                "live effect completed definitively; uncertainty boundary not reached",
            )),
            Err(failure) => Ok(failure),
        }
    }

    pub(super) fn reconcile(&self) -> crate::fabric_execution::ReconciliationStatus {
        self.adapter.reconcile(&self.request.plan.request.operation_ref, super::GENERATION)
    }
}

/// Retains published streams in memory; the scenarios assert nothing is
/// published for an uncertain or pre-start-failed effect.
#[derive(Debug, Default)]
struct RetainedPublisher {
    published: u64,
}

impl crate::fabric_execution::ExecutionOutputPublisher for RetainedPublisher {
    fn publish(
        &mut self,
        operation_ref: &str,
        stream: &crate::fabric_execution::RetainedStream,
    ) -> Result<crate::fabric_execution::PublishedStream, crate::fabric_execution::OutputPublicationError> {
        self.published = self.published.saturating_add(1);
        let content_ref = crate::preserves_rail::content_ref_from_bytes(&stream.retained_bytes);
        let material = format!("{operation_ref}\0{}\0{content_ref}", stream.role);
        Ok(crate::fabric_execution::PublishedStream {
            content_ref,
            publication_receipt_ref: crate::preserves_rail::content_ref_from_bytes(material.as_bytes()),
        })
    }
}

/// Canonical projection of the adapter's typed port failure (kind, code and
/// operation binding). Adapter detail text is excluded.
pub(super) fn failure_value(failure: &crate::fabric_execution::PortFailure) -> preserves::IOValue {
    crate::preserves_rail::record("execution-port-failure-v1", vec![
        crate::preserves_rail::string(format!("{:?}", failure.kind)),
        crate::preserves_rail::string(failure.diagnostic_code),
        crate::preserves_rail::string(&failure.operation_ref),
        crate::preserves_rail::bool_value(failure.process_observation.is_some()),
        crate::preserves_rail::bool_value(failure.receipt.is_some()),
    ])
}

fn descriptor() -> crate::fabric_execution::ExecutionProfileDescriptor {
    crate::fabric_execution::ExecutionProfileDescriptor {
        schema: crate::fabric_execution::EXECUTION_PROFILE_SCHEMA.to_string(),
        profile_id: "bounded-process-live-v1".to_string(),
        profile_ref: super::reference("qualification effect execution profile declaration"),
        kind: crate::fabric_execution::ExecutionProfileKind::LiveBoundedProcess,
        platform: crate::fabric_execution::ExecutionPlatform::UnixProcessGroup,
        supported_termination_scopes: vec![crate::fabric_execution::ExecutionTerminationScope::ProcessGroup],
        max_timeout_ms: super::TIMEOUT_MS,
        max_stdin_bytes: super::UNCONSUMED_INPUT_BYTES,
        max_stdout_bytes: super::STREAM_BYTES,
        max_stderr_bytes: super::STREAM_BYTES,
        max_poll_interval_ms: super::POLL_INTERVAL_MS,
        max_teardown_timeout_ms: super::TEARDOWN_TIMEOUT_MS,
        max_arguments: 8,
        max_argument_bytes: 256,
        max_environment_entries: 8,
        max_environment_name_bytes: 64,
        max_environment_value_bytes: 256,
        max_concurrency_units: super::UNITS,
        max_queue_units: super::UNITS,
        component_repository: crate::fabric_execution::BOUNDED_EXEC_REPOSITORY.to_string(),
        component_revision: crate::fabric_execution::BOUNDED_EXEC_REVISION.to_string(),
        component_license: crate::fabric_execution::BOUNDED_EXEC_LICENSE.to_string(),
        component_package: crate::fabric_execution::BOUNDED_EXEC_PACKAGE.to_string(),
        conformance_refs: vec![super::reference("qualification effect bounded-exec conformance")],
        fabric_non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        non_claims: crate::fabric_execution::REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
    }
}

fn execution_request(executable_identity_ref: String) -> crate::fabric_execution::ExecutionRequest {
    crate::fabric_execution::ExecutionRequest {
        schema: crate::fabric_execution::EXECUTION_REQUEST_SCHEMA.to_string(),
        operation_ref: super::reference("qualification effect operation"),
        idempotency_ref: super::reference("qualification effect idempotency"),
        extension_id: super::EXTENSION_ID.to_string(),
        service_id: super::SERVICE_ID.to_string(),
        callback_ref: super::reference("qualification effect callback"),
        effect_ref: super::reference("qualification effect intent"),
        generation: super::GENERATION,
        profile_ref: super::reference("qualification effect execution profile declaration"),
        executable_artifact_ref: super::reference("qualification effect executable artifact"),
        executable_identity_ref,
        arguments: vec!["-c".to_string(), super::EFFECT_SCRIPT.to_string()],
        environment: vec![crate::fabric_execution::EnvironmentEntry {
            name: "ASPEN_QUALIFICATION".to_string(),
            value: "effect".to_string(),
            value_class: crate::fabric_execution::EnvironmentValueClass::Public,
        }],
        environment_mode: crate::fabric_execution::ExecutionEnvironmentMode::Clear,
        invocation_mode: crate::fabric_execution::ExecutionInvocationMode::Direct,
        executable_resolution: crate::fabric_execution::ExecutableResolutionMode::ExactArtifact,
        workspace_ref: super::reference("qualification effect workspace"),
        workspace_mode: crate::fabric_execution::WorkspaceMode::CapabilityRoot,
        stdin_ref: Some(crate::preserves_rail::content_ref_from_bytes(&unconsumed_input())),
        limits: crate::fabric_execution::ExecutionRequestLimits {
            timeout_ms: super::TIMEOUT_MS,
            stdin_max_bytes: super::UNCONSUMED_INPUT_BYTES,
            stdout_max_bytes: super::STREAM_BYTES,
            stderr_max_bytes: super::STREAM_BYTES,
            poll_interval_ms: super::POLL_INTERVAL_MS,
            teardown_timeout_ms: super::TEARDOWN_TIMEOUT_MS,
            concurrency_units: super::UNITS,
            queue_units: super::UNITS,
        },
        termination_scope: crate::fabric_execution::ExecutionTerminationScope::ProcessGroup,
        accepted_exit_codes: vec![0],
        reject_stdout_truncation: true,
        reject_stderr_truncation: true,
        authority_ref: super::reference("qualification effect authority"),
        resource_grant_ref: super::reference("qualification effect resource grant"),
    }
}

/// Input the child never reads; its admitted stdin ref is the hash of these
/// bytes. Built from the `u64` bound itself, so no platform-width cast applies.
fn unconsumed_input() -> Vec<u8> {
    (0..super::UNCONSUMED_INPUT_BYTES).map(|_| 0_u8).collect()
}

fn authority(request: &crate::fabric_execution::ExecutionRequest) -> crate::fabric_execution::ExecutionAuthorityFacts {
    crate::fabric_execution::ExecutionAuthorityFacts {
        authority_ref: request.authority_ref.clone(),
        executable_authority_ref: super::reference("qualification effect executable authority"),
        provenance_ref: super::reference("qualification effect provenance"),
        effect_admission_ref: super::reference("qualification effect admission"),
        workspace_authority_ref: super::reference("qualification effect workspace authority"),
        process_authority_ref: super::reference("qualification effect process authority"),
        resource_grant_ref: request.resource_grant_ref.clone(),
        policy_ref: super::reference("qualification effect policy"),
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

const fn resources() -> crate::fabric_execution::ExecutionResourceGrant {
    crate::fabric_execution::ExecutionResourceGrant {
        memory_bytes: super::STREAM_BYTES * 4,
        storage_bytes: super::STREAM_BYTES * 4,
        diagnostic_bytes: super::STREAM_BYTES * 4,
        logical_deadline_ticks: super::DEADLINE_TICKS,
        concurrency_units: super::UNITS,
        queue_units: super::UNITS,
    }
}
