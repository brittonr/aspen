
fn barrier_records(barriers: &OrderedMap<String, BarrierState>) -> Vec<IoValue> {
    barriers
        .iter()
        .map(|(key, barrier)| {
            let participants = barrier.participants.iter().cloned().collect::<Vec<_>>();
            record("barrier", vec![
                string(key),
                strings_sequence(&participants),
                u64_value(barrier.required),
                string(if barrier.is_released { "released" } else { "waiting" }),
            ])
        })
        .collect()
}

fn registry_records(registry: &OrderedMap<String, RegistryEntry>) -> Vec<IoValue> {
    registry
        .iter()
        .map(|(key, entry)| {
            record("registry-entry", vec![string(key), string(&entry.endpoint_ref), string(&entry.evidence_ref)])
        })
        .collect()
}
