
// ---------------------------------------------------------------------------
// Pure core: informer snapshot validation
// ---------------------------------------------------------------------------

/// Validate informer snapshot consistency.
pub fn validate_informer_snapshot(input: &InformerValidationInput) -> InformerValidationResult {
    // Validate that applied events match declared events
    if input.snapshot.applied_watch_event_refs.len() != input.watch_events.len() {
        return InformerValidationResult {
            pass: false,
            cache_current: false,
            diagnostics: vec![format!(
                "event count mismatch: snapshot declares {} events but {} were provided",
                input.snapshot.applied_watch_event_refs.len(),
                input.watch_events.len(),
            )],
        };
    }

    // Validate starting cursor matches
    if input.snapshot.starting_cursor != input.starting_cursor {
        return InformerValidationResult {
            pass: false,
            cache_current: false,
            diagnostics: vec![format!(
                "starting cursor mismatch: snapshot {:?} vs input {:?}",
                input.snapshot.starting_cursor, input.starting_cursor,
            )],
        };
    }

    // Validate cursor advances through all events
    let mut expected_cursor = input.starting_cursor;
    for (i, event) in input.watch_events.iter().enumerate() {
        if event.prior_cursor != expected_cursor {
            return InformerValidationResult {
                pass: false,
                cache_current: false,
                diagnostics: vec![format!(
                    "event {} cursor mismatch: expected {:?} but got {:?}",
                    i, expected_cursor, event.prior_cursor,
                )],
            };
        }
        expected_cursor = event.next_cursor;
    }

    // Validate final cursor
    if expected_cursor != input.final_cursor {
        return InformerValidationResult {
            pass: false,
            cache_current: false,
            diagnostics: vec![format!(
                "final cursor mismatch: expected {:?} after all events but got {:?}",
                expected_cursor, input.final_cursor,
            )],
        };
    }

    // Validate snapshot final cursor matches
    if input.snapshot.final_cursor != input.final_cursor {
        return InformerValidationResult {
            pass: false,
            cache_current: false,
            diagnostics: vec![format!(
                "snapshot final cursor {:?} does not match expected {:?}",
                input.snapshot.final_cursor, input.final_cursor,
            )],
        };
    }

    InformerValidationResult {
        pass: true,
        cache_current: true,
        diagnostics: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Preserves encoding helpers
// ---------------------------------------------------------------------------

pub fn watch_event_to_value(event: &WatchEvent) -> IoValue {
    record("watch-event-v1", vec![
        string(&event.resource_ref),
        string(&event.resource_type),
        string(&event.scope_ref),
        u64_value(event.generation),
        string(event.kind.as_str()),
        u64_value(event.prior_cursor.cursor),
        u64_value(event.next_cursor.cursor),
        refs_sequence(&event.admission_receipt_refs),
        refs_sequence(&event.selector_refs),
        refs_sequence(&event.observer_authority_refs),
        string(&event.event_body_ref),
        refs_sequence(&event.evidence_refs),
    ])
}