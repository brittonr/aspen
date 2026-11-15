use preserves::ValueImpl as _;

use super::*;

const INPUT_FIELDS: u64 = 11;
const STEP_FIELDS: u64 = 2;
pub const MAX_CLIENT_INPUT_BYTES: usize = 16_384;

pub fn input_value(input: &Input) -> preserves::IOValue {
    crate::preserves_rail::record("molten-chaoscontrol-client-v1", vec![
        crate::preserves_rail::string(&input.schema),
        crate::preserves_rail::string(&input.profile),
        crate::preserves_rail::sequence(
            input.participants.as_slice().iter().map(crate::preserves_rail::string).collect(),
        ),
        crate::preserves_rail::u64_value(input.max_payload_bytes),
        crate::preserves_rail::u64_value(input.recovery_ticks),
        crate::preserves_rail::string(input.control.as_str()),
        crate::preserves_rail::sequence(
            input
                .steps
                .iter()
                .map(|step| {
                    crate::preserves_rail::record("client-step-v1", vec![
                        crate::preserves_rail::string(&step.request_ref),
                        crate::preserves_rail::string(step.command.as_str()),
                    ])
                })
                .collect(),
        ),
        crate::preserves_rail::string(&input.submit_request_ref),
        crate::preserves_rail::string(&input.command_ref),
        crate::preserves_rail::u64_value(input.max_event_loop_iterations),
        crate::preserves_rail::u64_value(input.child_timeout_seconds),
    ])
}

pub fn input_ref(input: &Input) -> crate::error::Result<String> {
    input.validate()?;
    crate::preserves_rail::canonical_hash(&input_value(input))
}

// r[impl molten.chaoscontrol_client.bounded_input]
pub fn decode_input(bytes: &[u8]) -> crate::error::Result<Input> {
    if bytes.len() > MAX_CLIENT_INPUT_BYTES {
        return Err(crate::error::Failure::invalid_harness("client input exceeds its byte limit"));
    }
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields =
        crate::preserves_rail::simple_record_fields(&decoded.value, "molten-chaoscontrol-client-v1", INPUT_FIELDS)?;
    let members = fields[2]
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness("client participants must be a sequence"))?;
    if members.len() != CLIENT_PARTICIPANTS.len() {
        return Err(crate::error::Failure::invalid_harness("client participant count is unsupported"));
    }
    let commands = fields[6]
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness("client steps must be a sequence"))?;
    if commands.len() > CLIENT_PHASES.len() {
        return Err(crate::error::Failure::invalid_harness("client steps exceed the finite profile"));
    }
    let control = match crate::preserves_rail::required_string_field(&fields[5], "control")?.as_str() {
        "healthy" => SemanticControl::Healthy,
        "duplicate-application" => SemanticControl::DuplicateApplication,
        _ => return Err(crate::error::Failure::invalid_harness("unsupported client semantic control")),
    };
    let steps = commands.as_slice().iter().map(decode_step).collect::<crate::error::Result<Vec<_>>>()?;
    let input = Input {
        schema: crate::preserves_rail::required_string_field(&fields[0], "schema")?,
        profile: crate::preserves_rail::required_string_field(&fields[1], "profile")?,
        submit_request_ref: crate::preserves_rail::required_content_ref_string(&fields[7], "submit request")?,
        command_ref: crate::preserves_rail::required_content_ref_string(&fields[8], "command identity")?,
        max_event_loop_iterations: unsigned_integer(&fields[9])?,
        child_timeout_seconds: unsigned_integer(&fields[10])?,
        participants: members
            .as_slice()
            .iter()
            .map(|member| crate::preserves_rail::required_string_field(member, "participant"))
            .collect::<crate::error::Result<_>>()?,
        max_payload_bytes: unsigned_integer(&fields[3])?,
        recovery_ticks: unsigned_integer(&fields[4])?,
        control,
        steps,
    };
    input.validate()?;
    Ok(input)
}

fn decode_step(command: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Step> {
    let command = crate::preserves_rail::value_to_iovalue(command);
    let parts = crate::preserves_rail::simple_record_fields(&command, "client-step-v1", STEP_FIELDS)?;
    let name = crate::preserves_rail::required_string_field(&parts[1], "command")?;
    let command = CLIENT_PHASES
        .into_iter()
        .find(|command| command.as_str() == name)
        .ok_or_else(|| crate::error::Failure::invalid_harness("unsupported client command"))?;
    Ok(Step {
        request_ref: crate::preserves_rail::required_content_ref_string(&parts[0], "request")?,
        command,
    })
}

pub fn reduced_value(reduction: &ReducedInput) -> crate::error::Result<preserves::IOValue> {
    let length = u32::try_from(reduction.prefix.steps.len())
        .map_err(|_| crate::error::Failure::invalid_harness("invalid client prefix length"))?;
    if reduction.prefix != reduction.original.valid_prefix(length)? {
        return Err(crate::error::Failure::invalid_harness("reduction changed more than a valid suffix"));
    }
    Ok(crate::preserves_rail::record("molten-client-valid-prefix-v1", vec![
        crate::preserves_rail::string(input_ref(&reduction.original)?),
        crate::preserves_rail::string(input_ref(&reduction.prefix)?),
        crate::preserves_rail::string(&reduction.property),
        input_value(&reduction.original),
        input_value(&reduction.prefix),
    ]))
}

pub fn unsigned_integer(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected client unsigned integer"))?
        .map_err(|error| crate::error::Failure::invalid_harness(format!("client integer out of range: {error}")))
}
