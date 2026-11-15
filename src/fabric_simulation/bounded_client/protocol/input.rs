pub const CLIENT_SCHEMA: &str = "molten.chaoscontrol-client.v1";
pub const NATIVE_PROFILE: &str = "native-three-process-raft-v1";
pub const CLIENT_PARTICIPANTS: [&str; 3] = ["node-a", "node-b", "node-c"];
pub const MAX_RECOVERY_TICKS: u64 = 32;
pub const CLIENT_EVENT_LOOP_LIMIT: usize = 1_200;
pub const CLIENT_CHILD_TIMEOUT_SECONDS: u64 = 30;
pub const CLIENT_INPUT_FILE: &str = "client-input.preserves";
pub const APPLY_ONCE_PROPERTY: &str = "molten.client.apply-once.v1";
pub const CLIENT_PHASES: [Command; 6] = [
    Command::Start,
    Command::CommitReadSnapshotPartition,
    Command::Checkpoint,
    Command::Crash,
    Command::Reopen,
    Command::FaultOffRecovery,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Start,
    CommitReadSnapshotPartition,
    Checkpoint,
    Crash,
    Reopen,
    FaultOffRecovery,
}

impl Command {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::CommitReadSnapshotPartition => "commit-read-snapshot-partition",
            Self::Checkpoint => "checkpoint",
            Self::Crash => "crash",
            Self::Reopen => "reopen",
            Self::FaultOffRecovery => "fault-off-recovery",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticControl {
    Healthy,
    DuplicateApplication,
}

impl SemanticControl {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::DuplicateApplication => "duplicate-application",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub request_ref: String,
    pub command: Command,
}

/// The first admitted vocabulary consists of six coarse, ordered scenario phases.
/// The service accepts opaque command refs, not application payload bytes. This
/// profile therefore explicitly admits zero payload bytes and one logical submit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub schema: String,
    pub profile: String,
    pub submit_request_ref: String,
    pub command_ref: String,
    pub participants: Vec<String>,
    pub max_payload_bytes: u64,
    pub recovery_ticks: u64,
    pub max_event_loop_iterations: u64,
    pub child_timeout_seconds: u64,
    pub control: SemanticControl,
    pub steps: Vec<Step>,
}

impl Input {
    pub fn native(recovery_ticks: u64, control: SemanticControl) -> crate::error::Result<Self> {
        let steps = CLIENT_PHASES
            .into_iter()
            .map(|command| {
                Ok(Step {
                    request_ref: native_phase_ref(command)?,
                    command,
                })
            })
            .collect::<crate::error::Result<Vec<_>>>()?;
        let input = Self {
            schema: CLIENT_SCHEMA.to_string(),
            profile: NATIVE_PROFILE.to_string(),
            submit_request_ref: native_operation_ref("distinct-process-request")?,
            command_ref: native_operation_ref("distinct-process-command")?,
            participants: CLIENT_PARTICIPANTS.into_iter().map(str::to_string).collect(),
            max_payload_bytes: 0,
            recovery_ticks,
            max_event_loop_iterations: CLIENT_EVENT_LOOP_LIMIT as u64,
            child_timeout_seconds: CLIENT_CHILD_TIMEOUT_SECONDS,
            control,
            steps,
        };
        input.validate()?;
        Ok(input)
    }

    // r[impl molten.chaoscontrol_client.bounded_input]
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.schema != CLIENT_SCHEMA || self.profile != NATIVE_PROFILE {
            return Err(crate::error::Failure::invalid_harness("unsupported client schema or execution profile"));
        }
        if !self.participants.iter().map(String::as_str).eq(CLIENT_PARTICIPANTS) {
            return Err(crate::error::Failure::invalid_harness(
                "client topology must match the three admitted Raft participants",
            ));
        }
        if self.submit_request_ref != native_operation_ref("distinct-process-request")?
            || self.command_ref != native_operation_ref("distinct-process-command")?
        {
            return Err(crate::error::Failure::invalid_harness(
                "unsupported native submit request or command identity",
            ));
        }
        if self.max_payload_bytes != 0 || !(1..=MAX_RECOVERY_TICKS).contains(&self.recovery_ticks) {
            return Err(crate::error::Failure::invalid_harness("unsupported payload or recovery budget"));
        }
        if self.max_event_loop_iterations != CLIENT_EVENT_LOOP_LIMIT as u64
            || self.child_timeout_seconds != CLIENT_CHILD_TIMEOUT_SECONDS
        {
            return Err(crate::error::Failure::invalid_harness("unsupported native termination budget"));
        }
        if self.steps.is_empty() || self.steps.len() > CLIENT_PHASES.len() {
            return Err(crate::error::Failure::invalid_harness("client operation count is outside the finite profile"));
        }
        for (step, required) in self.steps.iter().zip(CLIENT_PHASES) {
            if step.command != required || step.request_ref != native_phase_ref(required)? {
                return Err(crate::error::Failure::invalid_harness(
                    "client prefix violates phase prerequisites or canonical request identity",
                ));
            }
        }
        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.steps.len() == CLIENT_PHASES.len()
    }

    pub fn differential_workload(&self) -> crate::error::Result<crate::fabric_simulation::LiveWorkload> {
        self.validate()?;
        crate::fabric_simulation::LiveWorkload::new(
            self.steps
                .iter()
                .enumerate()
                .map(|(sequence, step)| crate::fabric_simulation::LiveWorkloadStep {
                    sequence: sequence as u64,
                    class: crate::fabric::FabricPortClass::Consistency,
                    request_ref: step.request_ref.clone(),
                    permitted: vec![
                        crate::fabric_simulation::OutcomeClass::Delivered,
                        crate::fabric_simulation::OutcomeClass::Unsupported,
                    ],
                    ordering_key: Some(NATIVE_PROFILE.to_string()),
                })
                .collect(),
        )
        .map_err(|issues| crate::error::Failure::invalid_harness(format!("client differential input: {issues:?}")))
    }

    /// Reducing suffixes preserves every participant and prerequisite. The caller
    /// reruns candidates; agreement of stored outcomes is not a reduction oracle.
    pub fn valid_prefix(&self, length: u32) -> crate::error::Result<Self> {
        self.validate()?;
        let length = usize::try_from(length)
            .map_err(|_| crate::error::Failure::invalid_harness("invalid client prefix length"))?;
        if length == 0 || length > self.steps.len() {
            return Err(crate::error::Failure::invalid_harness("invalid client prefix length"));
        }
        let mut prefix = self.clone();
        prefix.steps.truncate(length);
        prefix.validate()?;
        Ok(prefix)
    }
}

/// Retains both inputs; failure identity must be re-observed by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReducedInput {
    pub original: Input,
    pub prefix: Input,
    pub property: String,
}

pub fn reduce_valid_prefix(
    input: &Input,
    property: &str,
    mut reproduces: impl FnMut(&Input, &str) -> crate::error::Result<bool>,
) -> crate::error::Result<ReducedInput> {
    input.validate()?;
    if property != APPLY_ONCE_PROPERTY || !reproduces(input, property)? {
        return Err(crate::error::Failure::invalid_harness("original client failure/property did not reproduce"));
    }
    for length in 1..=input.steps.len() as u32 {
        let prefix = input.valid_prefix(length)?;
        if reproduces(&prefix, property)? {
            return Ok(ReducedInput {
                original: input.clone(),
                prefix,
                property: property.to_string(),
            });
        }
    }
    Err(crate::error::Failure::invalid_harness("no valid client prefix reproduced the original property"))
}

fn native_phase_ref(command: Command) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("molten-client-phase-request-v1", vec![
        crate::preserves_rail::string(command.as_str()),
    ]))
}

fn native_operation_ref(label: &str) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("fabric-consistency-live-test-ref", vec![
        crate::preserves_rail::string(label),
    ]))
}
