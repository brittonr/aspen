use super::super::choregraph::digest_from_reference;
use super::super::*;
use super::support::*;

// r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
#[test]
fn choregraph_digests_accept_only_lowercase_hex_pairs() {
    let lowercase = reference("choregraph-digest");
    let hex = lowercase.strip_prefix("blake3:").expect("canonical reference");
    let uppercase = format!("blake3:{}", hex.to_ascii_uppercase());
    assert_ne!(uppercase, lowercase, "fixture digest must contain hex letters");
    assert!(digest_from_reference(&lowercase).is_ok());
    assert_eq!(digest_from_reference(&uppercase).err(), Some(WorldHeadIssue::ChoregraphDenied));

    // `u8::from_str_radix` alone admits a leading `+`, so "+f" would decode as a hex pair.
    let signed = format!("blake3:+{}", &hex[1..]);
    assert_eq!(digest_from_reference(&signed).err(), Some(WorldHeadIssue::ChoregraphDenied));
}

// r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
#[test]
fn world_head_issue_codes_are_closed_kebab_tags() {
    let issues = [
        WorldHeadIssue::InvalidBounds(WorldHeadBound::MaxSigners),
        WorldHeadIssue::InvalidBounds(WorldHeadBound::PolicyMaxConflicts),
        WorldHeadIssue::StaleExpectedHead,
        WorldHeadIssue::SignerThresholdMiss,
        WorldHeadIssue::ChoregraphDenied,
        WorldHeadIssue::ConflictStateMismatch,
    ];
    let codes = issues.iter().map(WorldHeadIssue::code).collect::<Vec<_>>();
    assert_eq!(codes, vec![
        "invalid-bounds:max-signers",
        "invalid-bounds:policy-max-conflicts",
        "stale-expected-head",
        "signer-threshold-miss",
        "choregraph-denied",
        "conflict-state-mismatch",
    ]);
    assert!(codes.iter().all(|code| code.bytes().all(|byte| byte.is_ascii_lowercase() || b"-:".contains(&byte))));
}
