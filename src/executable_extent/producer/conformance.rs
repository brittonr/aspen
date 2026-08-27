//! Producer and consumer conformance receipt parity.

pub(super) fn validate(receipt: &super::model::Receipt) -> Result<(), super::Error> {
    let layouts = executable_extent_conformance::standard_vectors();
    let producer_layout = executable_extent_conformance::run(
        executable_extent_conformance::AdapterRole::Producer,
        &receipt.producer_implementation_id,
        layouts,
    )
    .map_err(|_error| super::Error::Conformance)?;
    let consumer_layout = executable_extent_conformance::run(
        executable_extent_conformance::AdapterRole::Consumer,
        "molten-executable-extent-consumer@v1",
        layouts,
    )
    .map_err(|_error| super::Error::Conformance)?;
    executable_extent_conformance::check_parity(&producer_layout, &consumer_layout)
        .map_err(|_error| super::Error::Conformance)?;

    let transitions = executable_extent_conformance::standard_transition_vectors();
    let producer_transition = executable_extent_conformance::run_transitions(
        executable_extent_conformance::AdapterRole::Producer,
        &receipt.producer_implementation_id,
        transitions,
    )
    .map_err(|_error| super::Error::Conformance)?;
    let consumer_transition = executable_extent_conformance::run_transitions(
        executable_extent_conformance::AdapterRole::Consumer,
        "molten-executable-extent-consumer@v1",
        transitions,
    )
    .map_err(|_error| super::Error::Conformance)?;
    executable_extent_conformance::check_transition_parity(&producer_transition, &consumer_transition)
        .map_err(|_error| super::Error::Conformance)?;

    let expected = [
        (&receipt.layout_corpus_identity_blake3, producer_layout.corpus_identity),
        (&receipt.layout_receipt_identity_blake3, producer_layout.receipt_identity),
        (&receipt.transition_corpus_identity_blake3, producer_transition.corpus_identity),
        (&receipt.transition_receipt_identity_blake3, producer_transition.receipt_identity),
    ];
    if expected
        .iter()
        .any(|(text, identity)| super::admission::decode_digest(text).ok().as_ref() != Some(identity))
    {
        return Err(super::Error::Conformance);
    }
    Ok(())
}
