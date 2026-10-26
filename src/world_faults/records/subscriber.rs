use super::*;

// r[impl molten.observations.stream]
pub(super) fn result_value(result: &CommittedSubscriberResult) -> IOValue {
    record("committed-subscriber-result", vec![
        field(
            "observation",
            result.observation.as_ref().map_or_else(
                || record("none", Vec::new()),
                |observation| record("some", vec![observation_value(observation)]),
            ),
        ),
        field("disposition", string(result.disposition.as_str())),
        field("diagnostics", sequence(result.diagnostics.iter().map(|issue| string(issue_code(issue))).collect())),
    ])
}

fn observation_value(observation: &CommittedSubscriberObservation) -> IOValue {
    record("committed-subscriber-observation", vec![
        field("case-id", string(&observation.case_id)),
        field("phase", string(observation.phase.as_str())),
        field("checkpoint-before", cursor_value(&observation.checkpoint_before)),
        field("checkpoint-after-restart", cursor_value(&observation.checkpoint_after_restart)),
        field("published", events_value(&observation.published)),
        field("delivered-before-restart", events_value(&observation.delivered_before_restart)),
        field("delivered-after-restart", events_value(&observation.delivered_after_restart)),
        field("deduplicated", events_value(&observation.deduplicated)),
        field("restart-count", number(u64::from(observation.restart_count))),
    ])
}

fn cursor_value(cursor: &molten_core::committed_stream::StreamCursor) -> IOValue {
    record("stream-cursor", vec![
        field("service-id", string(&cursor.service_id)),
        field("stream-epoch", number(cursor.stream_epoch)),
        field("revision", number(cursor.revision)),
        field("ordinal", number(u64::from(cursor.ordinal))),
    ])
}

fn events_value(events: &[molten_core::committed_stream::CommittedChangeEvent]) -> IOValue {
    sequence(
        events
            .iter()
            .map(|event| {
                record("committed-change-event", vec![
                    field("event-ref", string(&event.event_ref)),
                    field("service-id", string(&event.service_id)),
                    field("stream-epoch", number(event.stream_epoch)),
                    field("revision", number(event.revision)),
                    field("ordinal", number(u64::from(event.ordinal))),
                    field("payload-ref", string(&event.payload_ref)),
                    field("committed-at-tick", number(event.committed_at_tick)),
                ])
            })
            .collect(),
    )
}
