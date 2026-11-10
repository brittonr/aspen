
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

impl super::CallbackReceipt {
    /// The bounded diagnostic code this receipt recorded for a failed dispatch.
    ///
    /// A redacted denial names the branch only; the decision and its code are the receipt's own
    /// admitted fields, so a failed activation can report which check refused it.
    pub fn diagnostic(&self) -> Option<String> {
        use preserves::ValueImpl as _;
        const FIELDS: usize = 16;
        let fields = self.value.collect_simple_record("system-extension-callback-receipt-v1", Some(FIELDS))?;
        let mut diagnostic = None;
        for field in fields.iter() {
            let Some(record) = field.collect_simple_record("diagnostic", Some(1)) else {
                continue;
            };
            let Some(value) = record.iter().next() else {
                continue;
            };
            if let Some(some) = value.collect_simple_record("some", Some(1)) {
                diagnostic = some.iter().next().and_then(|item| item.as_string().map(|text| text.to_string()));
            }
            break;
        }
        diagnostic
    }
}
