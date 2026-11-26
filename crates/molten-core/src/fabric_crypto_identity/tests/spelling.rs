use super::*;

const LOWERCASE_REF: &str = "blake3:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd";
const UPPERCASE_REF: &str = "blake3:ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD";
const MIXED_CASE_REF: &str = "blake3:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcD";
const PREFIX_ONLY_REF: &str = "blake3:key";

// r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
#[test]
fn artifact_key_identity_admits_only_lowercase_canonical_digests() {
    let observed = standalone_observation(LOWERCASE_REF, true).expect("lowercase key ref admitted");
    assert_eq!(observed.key_identity.digest_hex, &LOWERCASE_REF["blake3:".len()..]);
    for spelling in [UPPERCASE_REF, MIXED_CASE_REF, PREFIX_ONLY_REF] {
        let denied = standalone_observation(spelling, true).expect_err("non-canonical key ref denied");
        assert!(denied.contains("key_ref:expected-blake3-ref"), "{spelling}: {denied}");
    }
}
