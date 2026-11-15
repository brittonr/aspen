//! Bounded keyed counter guest for the admitted native callback protocol.
//!
//! All semantic state and duplicate results live in the existing native value/journal owners.
//! The guest performs no filesystem, network, time, entropy or process effects.
mod codec;
#[cfg(test)]
mod tests;

pub const MAX_COMPLETED_REQUESTS: usize = 128;
const MAX_CALLBACK_BYTES: u64 = 1_048_576;
const MAX_VALUE_BYTES: u64 = 262_144;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Completed {
    pub request_id: String,
    pub increment: u64,
    pub result: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub actor_key_ref: String,
    pub value: u64,
    pub completed: Vec<Completed>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Add {
    pub actor_key_ref: String,
    pub request_id: String,
    pub increment: u64,
}

impl State {
    pub fn decode(bytes: &[u8]) -> crate::error::Result<Self> {
        codec::decode_state(bytes)
    }

    pub fn materialize(&self) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
        codec::state_value(self)
    }

    /// Exact logical replay returns the original result, not today's counter value.
    pub fn apply(&mut self, input: &Add) -> crate::error::Result<u64> {
        codec::validate_add(input)?;
        if input.actor_key_ref != self.actor_key_ref {
            return Err(invalid("counter request targets another actor key"));
        }
        if let Some(done) = self.completed.iter().find(|done| done.request_id == input.request_id) {
            if done.increment != input.increment {
                return Err(invalid("counter logical request was reused with different input"));
            }
            return Ok(done.result);
        }
        if self.completed.len() >= MAX_COMPLETED_REQUESTS {
            return Err(invalid("counter completed-request retention bound exhausted"));
        }
        let result = self.value.checked_add(input.increment).ok_or_else(|| invalid("counter overflow"))?;
        self.completed.push(Completed {
            request_id: input.request_id.clone(),
            increment: input.increment,
            result,
        });
        self.value = result;
        Ok(result)
    }
}

impl Add {
    pub fn materialize(&self) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
        codec::add_value(self)
    }

    pub fn decode(bytes: &[u8]) -> crate::error::Result<Self> {
        codec::decode_add(bytes)
    }
}

/// Pure callback behavior. A callback result is not itself evidence of durable publication.
pub fn invoke(
    envelope: &crate::system_extension::DecodedCallbackEnvelope,
) -> crate::error::Result<crate::system_extension::MaterializedCallbackOutcome> {
    crate::preserves_rail::ContentRef::parse(&envelope.context.service_id)?;
    let mut state = match &envelope.inputs.state {
        Some(value) => State::decode(&value.bytes)?,
        None if envelope.invocation.callback == crate::system_extension::CallbackKind::Initialize => State {
            actor_key_ref: envelope.context.service_id.clone(),
            value: 0,
            completed: Vec::new(),
        },
        None => return Err(invalid("counter callback has no materialized state")),
    };
    if state.actor_key_ref != envelope.context.service_id {
        return Err(invalid("counter state belongs to another installed actor"));
    }
    let mut outputs = Vec::new();
    if envelope.invocation.callback == crate::system_extension::CallbackKind::Request {
        let payload = envelope.inputs.payload.as_ref().ok_or_else(|| invalid("counter request has no payload"))?;
        let input = Add::decode(&payload.bytes)?;
        let result = state.apply(&input)?;
        outputs.push(codec::result_value(&input, result)?);
    }
    if envelope.invocation.callback == crate::system_extension::CallbackKind::Recover {
        let checkpoint =
            envelope.inputs.payload.as_ref().ok_or_else(|| invalid("counter recovery lacks checkpoint"))?;
        let restored = State::decode(&checkpoint.bytes)?;
        if restored != state {
            return Err(invalid("counter checkpoint differs from durably selected semantic state"));
        }
    }
    let materialized = state.materialize()?;
    let checkpoint = (envelope.invocation.callback == crate::system_extension::CallbackKind::Checkpoint)
        .then(|| materialized.clone());
    Ok(crate::system_extension::MaterializedCallbackOutcome {
        outputs,
        effects: Vec::new(),
        state: Some(materialized),
        checkpoint,
        health: crate::system_extension::HealthState::Healthy,
    })
}

/// Native executable boundary; use as the entry point of the measured counter guest binary.
pub fn run_stdio() -> crate::error::Result<()> {
    use std::io::Read;
    use std::io::Write;
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(MAX_CALLBACK_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(crate::error::Failure::from)?;
    if bytes.len() as u64 > MAX_CALLBACK_BYTES {
        return Err(invalid("counter callback exceeds byte limit"));
    }
    let envelope = crate::system_extension::decode_callback_envelope(&bytes, MAX_CALLBACK_BYTES, MAX_VALUE_BYTES, 64)?;
    let result = crate::system_extension::encode_callback_outcome(&invoke(&envelope)?)?;
    let mut output = std::io::stdout().lock();
    output.write_all(&result).map_err(crate::error::Failure::from)?;
    output.flush().map_err(crate::error::Failure::from)
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
