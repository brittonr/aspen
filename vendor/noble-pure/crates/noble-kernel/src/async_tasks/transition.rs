mod cleanup;
mod completion;
mod lifecycle;
mod validation;

/// Allocation-free decision core, used directly by the retained production table.
/// Fabricated snapshots/decisions do not mint `Task`, native authority or receipts.
// r[impl RA-ASYNC-02]
// r[impl RA-ASYNC-03]
// r[impl RA-ASYNC-04]
// r[impl RA-ASYNC-05]
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; the production M6 proof binds this exact transition::transition decision path, so preserving its audited semantic identity is part of the extraction contract."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; transition rejects invalid records, handles and native identities in order before classifying the event; assertions would replace these typed refusal outcomes with panics."
)]
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; every classified Rule must select an explicit production transition; a new rule must fail compilation until its lifecycle and accounting semantics are supplied."
)]
pub const fn transition(
    record: super::Snapshot,
    claim: super::Handle,
    event: super::Event,
) -> Result<super::Decision, super::Error> {
    attempt!(validation::record(record));
    attempt!(validation::handle(record.handle, claim));
    attempt!(validation::native(record.native, event));
    let rule = attempt!(super::schema::classify(record.state, event));
    match rule {
        super::schema::Rule::Inspect => {
            Ok(super::Decision::unchanged(record, super::Action::Inspected))
        }
        super::schema::Rule::Complete {
            outcome,
            completion,
        } => completion::complete(record, outcome, completion),
        super::schema::Rule::Deliver => Ok(lifecycle::deliver(record)),
        super::schema::Rule::Retire(reason) => Ok(lifecycle::retire(record, reason)),
        super::schema::Rule::ObserveStop => Ok(lifecycle::native_stop(record)),
        super::schema::Rule::SettlePins(pins) => cleanup::pins(record, pins),
        super::schema::Rule::Cleanup(obligations) => cleanup::settle(record, obligations),
        super::schema::Rule::Wake => Ok(lifecycle::wake(record)),
        super::schema::Rule::TakeWake => lifecycle::take_wake(record),
        super::schema::Rule::Finish => cleanup::finish(record),
        super::schema::Rule::Duplicate => {
            Ok(super::Decision::unchanged(record, super::Action::Duplicate))
        }
    }
}
