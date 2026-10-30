//! Native callbacks emit only a staged consistency request. Callback success is
//! never represented as a collaboration commit or protected resource effect.

use super::*;

pub const COMMIT_PORT_ID: &str = "molten.collaboration-scope.commit";
pub const COMMIT_PORT_VERSION: &str = "v1";
pub const COMMIT_OPERATION: &str = "commit-staged";
pub const COMMIT_INPUT_SCHEMA: &str = "molten.collaboration-scope.staged-command.v1";
pub const COMMIT_OUTPUT_SCHEMA: &str = "molten.collaboration-scope.committed-command.v1";

pub fn staged_command_value(command: &crate::fabric_consistency::raft::Command) -> preserves::IOValue {
    crate::preserves_rail::record("collaboration-staged-command-v1", vec![
        crate::preserves_rail::string(&command.request_ref),
        crate::preserves_rail::string(&command.command_ref),
        crate::preserves_rail::string(&command.command_schema_ref),
    ])
}

pub fn parse_staged_command(
    value: &preserves::IOValue,
) -> crate::error::Result<crate::fabric_consistency::raft::Command> {
    let f = decode::fields(value, "collaboration-staged-command-v1", 3)?;
    let command = crate::fabric_consistency::raft::Command {
        request_ref: decode::string(&f[0])?,
        command_ref: decode::string(&f[1])?,
        command_schema_ref: decode::string(&f[2])?,
    };
    for reference in [&command.request_ref, &command.command_ref, &command.command_schema_ref] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    if command.command_schema_ref != command_schema_ref()? {
        return Err(denied(Denial::Domain));
    }
    Ok(command)
}

// r[impl molten.collaboration_scope.versioned_extension]
pub fn handle_callback(
    envelope: &crate::system_extension::DecodedCallbackEnvelope,
) -> crate::error::Result<crate::system_extension::MaterializedCallbackOutcome> {
    let mut bridge = bridge::HostState::observe(envelope)?;
    let mut effects = Vec::new();
    if envelope.invocation.callback == crate::system_extension::CallbackKind::Request {
        let payload = envelope.inputs.payload.as_ref().ok_or_else(|| denied(Denial::Missing))?;
        let value = crate::preserves_rail::parse_canonical_bytes(&payload.bytes)?;
        let command = parse_staged_command(&value)?;
        bridge.intent(&command)?;
        let target = crate::system_extension::EffectTarget::FabricPort(crate::fabric::FabricPortKey {
            port_id: COMMIT_PORT_ID.to_string(),
            version: COMMIT_PORT_VERSION.to_string(),
        });
        effects.push(crate::system_extension::MaterializedEffectRequest {
            effect: crate::system_extension::TypedEffectRequest {
                target,
                operation: COMMIT_OPERATION.to_string(),
                input_schema_ref: COMMIT_INPUT_SCHEMA.to_string(),
                output_schema_ref: COMMIT_OUTPUT_SCHEMA.to_string(),
                request_ref: payload.value_ref.clone(),
                generation: envelope.invocation.generation,
                incarnation: envelope.invocation.incarnation,
                accounted_bytes: payload.bytes.len() as u64,
            },
            request: payload.clone(),
        });
    }
    let output = materialized(&bridge.status())?;
    let state = materialized(&bridge.value())?;
    let is_checkpoint = envelope.invocation.callback == crate::system_extension::CallbackKind::Checkpoint;
    let checkpoint = is_checkpoint.then(|| state.clone());
    Ok(crate::system_extension::MaterializedCallbackOutcome {
        outputs: vec![output],
        effects,
        state: Some(state),
        checkpoint,
        health: crate::system_extension::HealthState::Healthy,
    })
}

fn materialized(value: &preserves::IOValue) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
    let bytes = crate::preserves_rail::canonical_bytes(value)?;
    let value_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    Ok(crate::system_extension::NativeCallbackValue { value_ref, bytes })
}

/// An actual bounded queue into the running consistency replica. Only commands
/// supplied by the fresh retained-runtime authority boundary. Queuing alone
/// returns no FabricEffectPort success and cannot complete native work.
pub(super) struct CommitPort {
    sender: tokio::sync::mpsc::Sender<crate::fabric_consistency::raft::ReplicaEvent>,
    generation: u64,
}
impl CommitPort {
    pub(super) fn new(
        sender: tokio::sync::mpsc::Sender<crate::fabric_consistency::raft::ReplicaEvent>,
        installation: &Installation,
    ) -> Self {
        Self {
            sender,
            generation: installation.generation(),
        }
    }
    pub(super) fn enqueue(
        &self,
        binding: &crate::fabric::CanonicalPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
        command: &crate::fabric_consistency::raft::Command,
    ) -> crate::error::Result<()> {
        if binding.binding.key.port_id != COMMIT_PORT_ID
            || binding.binding.key.version != COMMIT_PORT_VERSION
            || effect.operation != COMMIT_OPERATION
            || effect.generation != self.generation
            || effect.input_schema_ref != COMMIT_INPUT_SCHEMA
            || effect.output_schema_ref != COMMIT_OUTPUT_SCHEMA
        {
            return Err(denied(Denial::Domain));
        }
        self.sender
            .try_send(crate::fabric_consistency::raft::ReplicaEvent::Propose {
                request_ref: command.request_ref.clone(),
                command_ref: command.command_ref.clone(),
                command_schema_ref: command.command_schema_ref.clone(),
            })
            .map_err(|_| denied(Denial::Partition))
    }
}

#[path = "bridge.rs"]
mod bridge;
