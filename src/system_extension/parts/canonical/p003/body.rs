
pub(crate) struct CallbackEventInput<'a> {
    pub(crate) callback: super::CallbackKind,
    pub(crate) generation: u64,
    pub(crate) sequence: u64,
    pub(crate) payload_ref: Option<&'a str>,
    pub(crate) logical_tick: u64,
    pub(crate) deadline_tick: u64,
}

pub(crate) fn callback_event_value(input: CallbackEventInput<'_>) -> preserves::IOValue {
    let CallbackEventInput {
        callback,
        generation,
        sequence,
        payload_ref,
        logical_tick,
        deadline_tick,
    } = input;
    crate::preserves_rail::record("system-extension-callback-event-v1", vec![
        crate::preserves_rail::string(super::SYSTEM_EXTENSION_CALLBACK_SCHEMA),
        field("callback", crate::preserves_rail::string(callback.as_str())),
        field("generation", crate::preserves_rail::u64_value(generation)),
        field("sequence", crate::preserves_rail::u64_value(sequence)),
        field("payload-ref", optional_string(payload_ref)),
        field("logical-tick", crate::preserves_rail::u64_value(logical_tick)),
        field("deadline-tick", crate::preserves_rail::u64_value(deadline_tick)),
    ])
}
