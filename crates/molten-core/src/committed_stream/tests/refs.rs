use super::*;

const LOWERCASE_REF: &str = "blake3:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd";
const UPPERCASE_REF: &str = "blake3:ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD";
const MIXED_CASE_REF: &str = "blake3:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcD";
const PREFIX_ONLY_REF: &str = "blake3:payload";

fn input(payload_ref: &str, state_ref: &str) -> CommitInput {
    CommitInput {
        payload_ref: payload_ref.to_string(),
        state_ref: state_ref.to_string(),
        committed_at_tick: 7,
        event_count: 1,
    }
}

// r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
#[test]
fn only_lowercase_canonical_digests_commit() {
    let mut accepted = new_stream("service:orchestrator", 1).expect("new");
    commit_transition(&mut accepted, &input(LOWERCASE_REF, LOWERCASE_REF)).expect("lowercase refs commit");
    assert_eq!(accepted.revision, 1);

    for spelling in [UPPERCASE_REF, MIXED_CASE_REF, PREFIX_ONLY_REF] {
        let mut denied = new_stream("service:orchestrator", 1).expect("new");
        assert_eq!(
            commit_transition(&mut denied, &input(spelling, LOWERCASE_REF)).expect_err("payload spelling denied"),
            CommittedStreamIssue::InvalidPayloadRef,
            "{spelling}"
        );
        assert_eq!(
            commit_transition(&mut denied, &input(LOWERCASE_REF, spelling)).expect_err("state spelling denied"),
            CommittedStreamIssue::InvalidStateRef,
            "{spelling}"
        );
        assert_eq!(denied.revision, 0);
    }
}
