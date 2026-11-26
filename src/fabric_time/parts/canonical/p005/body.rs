/// Entropy events bind the stream capability reference under their own versioned record label. Events of
/// every other kind keep their existing labels, so their canonical bytes are unchanged.
const FABRIC_ENTROPY_EVENT_RECORD: &str = "fabric-time-entropy-event-v2";

fn event_record_label(kind: EventKind) -> (&'static str, &'static str) {
    match kind {
        EventKind::Scheduler => (FABRIC_SCHEDULER_EVENT_RECORD, super::FABRIC_SCHEDULER_EVENT_SCHEMA),
        EventKind::Entropy => (FABRIC_ENTROPY_EVENT_RECORD, super::FABRIC_ENTROPY_EVENT_SCHEMA),
        EventKind::ClockAnomaly
        | EventKind::Timer
        | EventKind::Deadline
        | EventKind::Lease
        | EventKind::Fault
        | EventKind::Conformance => (FABRIC_TIME_EVENT_RECORD, super::FABRIC_TIME_OBSERVATION_SCHEMA),
    }
}

/// Read the capability reference bound into a recorded entropy event. Entropy evidence without the
/// versioned capability-bound label is rejected rather than trusted.
// r[impl molten.fabric_time.entropy_capability_evidence]
pub fn entropy_event_capability_ref(event: &Event) -> crate::error::Result<String> {
    const EVENT_FIELD_COUNT: usize = 9;
    const DETAILS_FIELD_INDEX: usize = 7;
    if crate::preserves_rail::canonical_hash(&event.value)? != event.evidence_ref {
        return Err(crate::error::Failure::invalid_harness("entropy event evidence ref does not match its value"));
    }
    let fields = event
        .value
        .collect_simple_record(FABRIC_ENTROPY_EVENT_RECORD, Some(EVENT_FIELD_COUNT))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected a capability-bound entropy event v2"))?;
    let schema = required_string(&fields[0], "entropy event schema")?;
    if schema != super::FABRIC_ENTROPY_EVENT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!("entropy event schema mismatch: {schema}")));
    }
    let details = named_field_value(&fields[DETAILS_FIELD_INDEX], "details")?;
    let capability_ref = event_detail_string(&details, "capability-ref")?;
    crate::preserves_rail::validate_content_ref(&capability_ref)?;
    Ok(capability_ref)
}

fn event_detail_string(details: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    const NAMED_FIELD_ARITY: usize = 2;
    let items = details
        .as_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected event details sequence"))?;
    for item in items {
        let fields = item
            .collect_simple_record("field", Some(NAMED_FIELD_ARITY))
            .ok_or_else(|| crate::error::Failure::invalid_harness("expected named event detail"))?;
        if required_string(&fields[0], "event detail name")? == label {
            return required_string(&fields[1], label);
        }
    }
    Err(crate::error::Failure::invalid_harness(format!("event details lack {label}")))
}
