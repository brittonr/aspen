//! Content references, value hashing, and optional-reference encoding.

use alloc::string::ToString;

const VALID: &str = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn content_refs_require_lowercase_blake3_digests() {
    let parsed = crate::identity::ContentRef::parse(VALID).expect("valid ref");
    assert_eq!(parsed.as_str(), VALID);
    assert_eq!(parsed.hex(), DIGEST);
    assert_eq!(crate::identity::ContentRef::from_hex(DIGEST), Ok(parsed.clone()));
    assert_eq!(crate::identity::ref_digest(VALID), Ok(DIGEST));
    assert!(crate::identity::has_ref_prefix("blake3:anything"));

    let uppercase = alloc::format!("blake3:{}", DIGEST.to_ascii_uppercase());
    let cases = [
        ("", crate::identity::RefError::MissingPrefix),
        ("sha256:0123", crate::identity::RefError::MissingPrefix),
        ("blake3:", crate::identity::RefError::DigestLength),
        ("blake3:0123", crate::identity::RefError::DigestLength),
        (uppercase.as_str(), crate::identity::RefError::DigestCharacter),
    ];
    for (input, expected) in cases {
        assert_eq!(crate::identity::validate_ref(input), Err(expected), "input {input}");
    }
    assert_eq!(
        crate::identity::ContentRef::from_hex(&DIGEST.to_ascii_uppercase()),
        Err(crate::identity::RefError::DigestCharacter)
    );
}

#[test]
fn value_refs_hash_canonical_bytes_with_blake3() {
    let value = super::support::example_record();
    let bytes = super::support::encode(&value);
    let expected = alloc::format!("blake3:{}", blake3::hash(&bytes).to_hex());
    assert_eq!(crate::identity::ContentRef::of_value(&value).as_str(), expected);
    assert_eq!(crate::identity::ContentRef::of_bytes(&bytes).to_string(), expected);
}

#[test]
fn domain_refs_differ_from_plain_value_refs() {
    let value = super::support::example_record();
    let bytes = super::support::encode(&value);
    let plain = crate::identity::ContentRef::of_value(&value);
    let first = crate::identity::domain_value_ref("aspen.test.first.v1", &value);
    let second = crate::identity::domain_ref("aspen.test.second.v1", &bytes);
    assert_ne!(first, plain);
    assert_ne!(first, second);
    assert_eq!(first, crate::identity::domain_ref("aspen.test.first.v1", &bytes));
    let mut hasher = blake3::Hasher::new_derive_key("aspen.test.first.v1");
    hasher.update(&bytes);
    assert_eq!(first, crate::identity::ContentRef::from_hash(hasher.finalize()));
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn optional_refs_have_one_canonical_encoding() {
    let reference = super::support::fixture_ref("optional");
    let some = crate::identity::optional_ref_value(Some(reference.as_str()));
    let none = crate::identity::optional_ref_value(None);
    assert_eq!(super::support::encode(&none), b"\xb4\xb3\x04none\x84");
    assert_eq!(crate::identity::decode_optional_ref(&some), Ok(Some(reference.clone())));
    assert_eq!(crate::identity::decode_optional_ref(&none), Ok(None));

    let bare = crate::model::PackedValue::string(reference.as_str());
    let shape = Err(crate::identity::OptionalRefError::Shape);
    assert_eq!(crate::identity::decode_optional_ref(&bare), shape);
    let extra = crate::model::PackedValue::symbol_record("none", alloc::vec![bare.clone()]);
    assert_eq!(crate::identity::decode_optional_ref(&extra), shape);
    let symbol = crate::model::PackedValue::symbol_record("some", alloc::vec![crate::model::PackedValue::symbol("x")]);
    assert_eq!(crate::identity::decode_optional_ref(&symbol), Err(crate::identity::OptionalRefError::NotString));
    let uppercase = alloc::format!("blake3:{}", reference.hex().to_ascii_uppercase());
    let wrapped = crate::identity::optional_ref_value(Some(&uppercase));
    assert_eq!(
        crate::identity::decode_optional_ref(&wrapped),
        Err(crate::identity::OptionalRefError::InvalidRef(crate::identity::RefError::DigestCharacter))
    );
}
