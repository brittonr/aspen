//! System-extension resource reservations: conservation against the admitted envelope.

const MAX_LIMIT: u64 = 6;
const MAX_OPERATIONS: usize = 48;
const MAX_EFFECT_BATCH: u64 = 3;
const OPERATION_KINDS: u8 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ledger {
    streams: u64,
    timers: u64,
    effects: u64,
}

fn envelope(tc: &hegel::TestCase) -> molten_core::system_extension::ResourceEnvelope {
    let limit = || hegel::generators::integers::<u64>().min_value(1).max_value(MAX_LIMIT);
    molten_core::system_extension::ResourceEnvelope {
        max_concurrent_callbacks: 1,
        max_queued_events: 1,
        max_inflight_bytes: 1,
        max_open_streams: tc.draw(limit()),
        max_timers: tc.draw(limit()),
        max_effect_requests: tc.draw(limit()),
        callback_deadline_ticks: 1,
        shutdown_grace_ticks: 1,
        max_restart_attempts: 1,
        overload_policy: molten_core::system_extension::OverloadPolicy::Reject,
    }
}

fn ledger(usage: molten_core::system_extension::ResourceUsage) -> Ledger {
    Ledger {
        streams: usage.open_streams,
        timers: usage.timers,
        effects: usage.effect_requests,
    }
}

fn untouched_fields_match(
    before: molten_core::system_extension::ResourceUsage,
    after: molten_core::system_extension::ResourceUsage,
) -> bool {
    before.concurrent_callbacks == after.concurrent_callbacks
        && before.queued_events == after.queued_events
        && before.inflight_bytes == after.inflight_bytes
}

/// Applies one operation; returns the next usage or the core's denial, plus the independent ledger.
fn step(
    envelope: &molten_core::system_extension::ResourceEnvelope,
    usage: molten_core::system_extension::ResourceUsage,
    model: Ledger,
    kind: u8,
    batch: u64,
) -> (
    Result<molten_core::system_extension::ResourceUsage, Vec<molten_core::system_extension::ResourceIssue>>,
    Option<Ledger>,
) {
    let within = |next: Option<u64>, maximum: u64| next.filter(|count| *count <= maximum);
    // The drawn batch is at most MAX_EFFECT_BATCH, so it always fits; an impossible conversion denies.
    let batch_count = usize::try_from(batch).ok();
    match (kind, batch_count) {
        (0, _) => (
            molten_core::system_extension::reserve_stream(envelope, usage),
            within(model.streams.checked_add(1), envelope.max_open_streams).map(|streams| Ledger { streams, ..model }),
        ),
        (1, _) => (
            molten_core::system_extension::release_stream(usage),
            model.streams.checked_sub(1).map(|streams| Ledger { streams, ..model }),
        ),
        (2, _) => (
            molten_core::system_extension::reserve_timer(envelope, usage),
            within(model.timers.checked_add(1), envelope.max_timers).map(|timers| Ledger { timers, ..model }),
        ),
        (3, _) => (
            molten_core::system_extension::release_timer(usage),
            model.timers.checked_sub(1).map(|timers| Ledger { timers, ..model }),
        ),
        (4, Some(count)) => (
            molten_core::system_extension::reserve_effect_requests(envelope, usage, count),
            within(model.effects.checked_add(batch), envelope.max_effect_requests)
                .map(|effects| Ledger { effects, ..model }),
        ),
        (_, Some(count)) => (
            molten_core::system_extension::release_effect_requests(usage, count),
            model.effects.checked_sub(batch).map(|effects| Ledger { effects, ..model }),
        ),
        (_, None) => (Err(Vec::new()), None),
    }
}

// Every admitted reservation is matched in an independent ledger, usage never exceeds the envelope,
// denials (over-reservation or release underflow) leave usage unchanged, and draining every
// outstanding reservation returns exactly to zero.
#[hegel::test(test_cases = 150, derandomize = true, database = None)]
fn hegel_reservations_are_conserved_within_the_envelope(tc: hegel::TestCase) {
    let envelope = envelope(&tc);
    let mut usage = molten_core::system_extension::ResourceUsage::default();
    let mut model = ledger(usage);
    let operations = tc.draw(hegel::generators::integers::<usize>().min_value(1).max_value(MAX_OPERATIONS));
    for _ in 0..operations {
        let kind = tc.draw(hegel::generators::integers::<u8>().min_value(0).max_value(OPERATION_KINDS - 1));
        let batch = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(MAX_EFFECT_BATCH));
        let (outcome, expected) = step(&envelope, usage, model, kind, batch);
        assert_eq!(outcome.is_ok(), expected.is_some(), "core {outcome:?} and ledger {expected:?} disagree");
        if let (Ok(next), Some(next_model)) = (&outcome, expected) {
            assert!(untouched_fields_match(usage, *next), "unrelated counters must not move");
            assert_eq!(ledger(*next), next_model, "core and ledger diverged on operation {kind}");
            usage = *next;
            model = next_model;
        }
        if let Err(issues) = &outcome {
            assert!(
                issues.iter().all(|issue| matches!(
                    issue,
                    molten_core::system_extension::ResourceIssue::UsageExceedsEnvelope { .. }
                        | molten_core::system_extension::ResourceIssue::CounterUnderflow(_)
                )),
                "denials must name the exceeded envelope or the underflow: {issues:?}"
            );
        }
        assert!(usage.open_streams <= envelope.max_open_streams);
        assert!(usage.timers <= envelope.max_timers);
        assert!(usage.effect_requests <= envelope.max_effect_requests);
    }

    let mut drained = Ok(usage);
    for _ in 0..model.streams {
        drained = drained.and_then(molten_core::system_extension::release_stream);
    }
    for _ in 0..model.timers {
        drained = drained.and_then(molten_core::system_extension::release_timer);
    }
    let effects = usize::try_from(model.effects).ok();
    let drained = drained
        .ok()
        .zip(effects)
        .and_then(|(usage, effects)| molten_core::system_extension::release_effect_requests(usage, effects).ok());
    let zero = molten_core::system_extension::ResourceUsage::default();
    assert_eq!(drained, Some(zero), "draining every reservation must return to zero");
    assert!(
        molten_core::system_extension::release_stream(zero).is_err(),
        "a drained stream counter cannot underflow"
    );
    assert!(
        molten_core::system_extension::release_timer(zero).is_err(),
        "a drained timer counter cannot underflow"
    );
    assert!(
        molten_core::system_extension::release_effect_requests(zero, 1).is_err(),
        "drained effects cannot underflow"
    );
}
