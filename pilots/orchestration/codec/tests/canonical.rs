// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeMap;

use molten_orchestrator_codec::*;
#[test]
fn projection_matches_an_independently_written_preserves_value() {
    let input = BTreeMap::from([("name", "example")]);
    let encoded = encode("fixture.v1", &input).unwrap();
    let expected = preserves::read_iovalue_text(
        "<molten.orchestration.document.v1 \"fixture.v1\" <o.map [<o.field \"name\" <o.string \"example\">>]>>",
        false,
    )
    .unwrap();
    assert_eq!(encoded.canonical, preserves::write_iovalue_packed(&expected, false).unwrap());
    assert_eq!(encoded.id, digest(&encoded.canonical));
}
#[test]
fn schema_domain_separation_changes_identity() {
    assert_ne!(identity("one.v1", &123_u64).unwrap(), identity("two.v1", &123_u64).unwrap());
}
#[test]
fn roundtrip_rejects_modified_canonical_and_projection() {
    let input = BTreeMap::from([("name".to_string(), "x".to_string())]);
    let encoded = encode("test.v1", &input).unwrap();
    let out: BTreeMap<String, String> = decode("test.v1", &encoded.canonical, &encoded.projection).unwrap();
    assert_eq!(out, input);
    let mut corrupt = encoded.canonical.clone();
    corrupt.push(0);
    assert!(decode::<BTreeMap<String, String>>("test.v1", &corrupt, &encoded.projection).is_err());
    assert!(
        decode::<BTreeMap<String, String>>("test.v1", &encoded.canonical, b"{\"name\":\"x\",\"name\":\"x\"}").is_err()
    );
}
#[test]
fn wire_rejects_wrong_schema_trailing_data_and_duplicate_keys() {
    let bytes = frame("request.v1", &42_u64).unwrap();
    assert_eq!(unframe::<u64>("request.v1", &bytes).unwrap(), 42);
    assert!(unframe::<u64>("other.v1", &bytes).is_err());
    let mut extra = bytes.clone();
    extra.push(b' ');
    assert!(unframe::<u64>("request.v1", &extra).is_err());
    let text = String::from_utf8(bytes).unwrap().replace("\"body\":42", "\"body\":42,\"body\":42");
    assert!(unframe::<u64>("request.v1", text.as_bytes()).is_err());
}
#[test]
fn floats_and_excessive_nesting_are_not_admitted() {
    assert!(encode("float.v1", &1.5_f64).is_err());
    let nested = format!("{}0{}", "[".repeat(200), "]".repeat(200));
    assert!(unframe::<serde_json::Value>("nested.v1", nested.as_bytes()).is_err());
}
