#[test]
fn exact_bundle_reference_rejects_tampering() {
    let bundle = super::Bundle {
        services: vec![],
        replacements: vec![],
    };
    assert!(super::encode(&bundle).is_err());
    let expected = crate::preserves_rail::content_ref_from_bytes(b"previous admitted bundle");
    let other = crate::preserves_rail::record("node-service-host-bundle-v1", vec![
        crate::preserves_rail::sequence(vec![]),
        crate::preserves_rail::sequence(vec![crate::preserves_rail::string("tampered")]),
    ]);
    let changed = crate::preserves_rail::canonical_bytes(&other).expect("tampered canonical bytes");
    assert!(super::decode(&changed, &expected).is_err());
}

#[test]
fn recomputing_reference_does_not_admit_wrong_schema_or_trailing_bytes() {
    let value = crate::preserves_rail::record("node-service-host-bundle-v2", vec![
        crate::preserves_rail::sequence(vec![]),
        crate::preserves_rail::sequence(vec![]),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value).expect("canonical wrong schema");
    let reference = crate::preserves_rail::content_ref_from_bytes(&bytes);
    assert!(super::decode(&bytes, &reference).is_err());
    let bundle = super::Bundle {
        services: vec![],
        replacements: vec![],
    };
    let value = super::wire::Wire::encode(&bundle).expect("raw empty wire bundle");
    let mut bytes = crate::preserves_rail::canonical_bytes(&value).expect("raw bytes");
    let reference = crate::preserves_rail::content_ref_from_bytes(&bytes);
    assert!(super::decode(&bytes, &reference).is_err());
    bytes.extend(crate::preserves_rail::canonical_bytes(&value).expect("second value"));
    let reference = crate::preserves_rail::content_ref_from_bytes(&bytes);
    assert!(super::decode(&bytes, &reference).is_err());
}

#[test]
fn bounded_collections_and_unknown_enum_cannot_materialize() {
    let sequence =
        crate::preserves_rail::sequence((0..257).map(|_| crate::preserves_rail::string("argument")).collect());
    assert!(<Vec<String> as super::wire::Wire>::decode(&sequence).is_err());
    let unknown = crate::preserves_rail::string("not-a-native-host-policy");
    assert!(<crate::system_extension::OverloadPolicy as super::wire::Wire>::decode(&unknown).is_err());
}

#[test]
fn references_are_exact_and_canonical_not_display_names() {
    assert!(super::wire::field(&crate::preserves_rail::string("latest"), "source_gate_ref").is_err());
    assert!(super::wire::field(&crate::preserves_rail::string("blake3:ABCDEF"), "manifest_ref").is_err());
    let reference = crate::preserves_rail::content_ref_from_bytes(b"actual materialized input");
    assert!(super::wire::field(&crate::preserves_rail::string(&reference), "manifest_ref").is_ok());
}
