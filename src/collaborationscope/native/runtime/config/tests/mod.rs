#[test]
fn configuration_rejects_duplicate_keys_instead_of_overwriting_authority_fields() {
    let duplicate = crate::preserves_rail::record("object", vec![
        crate::preserves_rail::record("entry", vec![
            crate::preserves_rail::string("policy_ref"),
            crate::preserves_rail::record("text", vec![crate::preserves_rail::string("first")]),
        ]),
        crate::preserves_rail::record("entry", vec![
            crate::preserves_rail::string("policy_ref"),
            crate::preserves_rail::record("text", vec![crate::preserves_rail::string("second")]),
        ]),
    ]);
    assert!(super::codec::parse(&duplicate, 0).is_err());
}

#[test]
fn configuration_rejects_signed_and_fractional_numeric_resources() {
    for number in [serde_json::json!(-1), serde_json::json!(1.5)] {
        assert!(super::codec::value(&number, 0).is_err());
    }
    let maximum = serde_json::json!(u64::MAX);
    let value = super::codec::value(&maximum, 0).expect("unsigned maximum");
    assert_eq!(super::codec::parse(&value, 0).expect("decoded maximum"), maximum);
}

#[test]
fn configuration_rejects_excessive_nesting_and_unknown_record_shapes() {
    let mut value = crate::preserves_rail::record("null", Vec::new());
    for _ in 0..17 {
        value = crate::preserves_rail::record("array", vec![value]);
    }
    assert!(super::codec::parse(&value, 0).is_err());
    assert!(super::codec::parse(&crate::preserves_rail::record("callback-success", Vec::new()), 0).is_err());
}

#[test]
fn configuration_round_trips_nested_containers_in_source_order() {
    let nested = serde_json::json!({
        "alpha": [1, {"inner": [null, true, false, "text"]}, []],
        "beta": {"gamma": {"delta": [2, 3]}, "empty": {}},
    });
    let value = super::codec::value(&nested, 0).expect("encoded nested configuration");
    assert_eq!(super::codec::parse(&value, 0).expect("decoded nested configuration"), nested);
    let mut deep = serde_json::json!(null);
    for _ in 0..17 {
        deep = serde_json::json!([deep]);
    }
    assert!(super::codec::value(&deep, 0).is_err());
}
