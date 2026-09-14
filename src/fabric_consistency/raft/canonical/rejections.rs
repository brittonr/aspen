#[test]
fn vote_encoding_rejects_other_families_without_panicking() {
    assert_denial(super::super::compatibility::Family::Vote);
}

#[test]
fn append_encoding_rejects_other_families_without_panicking() {
    assert_denial(super::super::compatibility::Family::Append);
}

#[test]
fn read_encoding_rejects_other_families_without_panicking() {
    assert_denial(super::super::compatibility::Family::Read);
}

#[test]
fn snapshot_encoding_rejects_other_families_without_panicking() {
    assert_denial(super::super::compatibility::Family::Snapshot);
}

fn assert_denial(family: super::super::compatibility::Family) {
    let diagnostic = match family {
        super::super::compatibility::Family::Vote => "vote encoding admitted a non-vote message",
        super::super::compatibility::Family::Append => "append encoding admitted a non-append message",
        super::super::compatibility::Family::Read => "read encoding admitted a non-read message",
        super::super::compatibility::Family::Snapshot => "snapshot encoding admitted a non-snapshot message",
    };
    let mut panic_count = 0;
    for case in super::super::compatibility::cases() {
        if case.family == family {
            continue;
        }
        let before = case.envelope.clone();
        let observed = std::panic::catch_unwind(|| encode(family, &case.envelope.message));
        assert_eq!(case.envelope, before, "{}", case.name);
        match observed {
            Ok(result) => assert_eq!(result, Err(crate::error::MoltenError::InvalidHarness(diagnostic.to_string()))),
            Err(_) => panic_count += 1,
        }
    }
    assert_eq!(panic_count, 0, "wrong-family encoding must return an error, not panic");
}

fn encode(
    family: super::super::compatibility::Family,
    message: &super::RaftMessage,
) -> crate::error::Result<preserves::IOValue> {
    match family {
        super::super::compatibility::Family::Vote => super::vote_message_value(message),
        super::super::compatibility::Family::Append => super::append_message_value(message),
        super::super::compatibility::Family::Read => super::read_message_value(message),
        super::super::compatibility::Family::Snapshot => super::snapshot_message_value(message),
    }
}

#[test]
fn matching_family_encoding_preserves_the_public_message_value() {
    const MESSAGE_FIELD: usize = 5;
    for case in super::super::compatibility::cases() {
        let encoded = encode(case.family, &case.envelope.message).expect("matching private encoder");
        let canonical = super::canonical_replica_message(&case.envelope).expect("public encoder");
        let fields = super::required_record(&canonical.value, "raft-message-envelope-v1", super::ENVELOPE_ARITY)
            .expect("canonical envelope fields");
        let expected: &preserves::IOValue = (&fields[MESSAGE_FIELD]).into();
        assert_eq!(&encoded, expected, "{}", case.name);
    }
}
