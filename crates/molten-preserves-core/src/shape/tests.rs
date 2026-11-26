use alloc::string::ToString;

use crate::fault::Classified;

type String = alloc::string::String;
type ViewError = super::ViewError;
type PackedValue = crate::model::PackedValue;

fn reference(seed: &[u8]) -> String {
    crate::build::content_ref_from_bytes(seed)
}

fn text(value: &str) -> PackedValue {
    crate::build::string(value)
}

fn wrap(label: &str, field: PackedValue) -> PackedValue {
    crate::build::record(label, alloc::vec![field])
}

fn message(result: Result<impl core::fmt::Debug, ViewError>) -> String {
    let error = result.expect_err("reader denies");
    assert_eq!(error.failure_kind(), crate::fault::FailureKind::InvalidHarness);
    error.to_string()
}

// r[verify molten.preserves_rail_toolkit.negative_shapes]
#[test]
fn simple_record_fields_admit_only_the_label_and_arity() {
    let value = crate::build::record("pair", alloc::vec![text("a"), crate::build::u64_value(1)]);
    let fields = super::simple_record_fields(&value, "pair", 2).expect("matching record");
    assert_eq!(fields, &[text("a"), crate::build::u64_value(1)]);
    assert_eq!(message(super::simple_record_fields(&value, "pair", 1)), "expected <pair ...> with arity 1");
    assert_eq!(message(super::simple_record_fields(&value, "other", 2)), "expected <other ...> with arity 2");
    let string_label = PackedValue::record(text("pair"), alloc::vec::Vec::new());
    assert!(super::simple_record_fields(&string_label, "pair", 0).is_err());
}

#[test]
fn string_and_sequence_fields_require_their_class() {
    assert_eq!(super::required_string_field(&text("x"), "name"), Ok(String::from("x")));
    assert_eq!(
        message(super::required_string_field(&crate::build::symbol("x"), "name")),
        "expected string for name"
    );
    let items = alloc::vec![crate::build::bool_value(true)];
    let sequence = crate::build::sequence(items.clone());
    assert_eq!(super::required_sequence_field(&sequence, "items"), Ok(items.as_slice()));
    assert_eq!(message(super::required_sequence_field(&text("x"), "items")), "expected sequence for items");
}

#[test]
fn content_ref_fields_nest_the_rail_reference_diagnostic() {
    let valid = reference(b"view");
    let parsed = super::required_content_ref(&text(&valid), "artifact").expect("canonical ref");
    assert_eq!(parsed.as_str(), valid);
    assert_eq!(super::required_content_ref_string(&text(&valid), "artifact"), Ok(valid.clone()));
    assert_eq!(
        message(super::required_content_ref(&text("sha256:00"), "artifact")),
        "artifact must be a canonical content ref: invalid harness artifact: content ref must start with blake3:, got \
         sha256:00"
    );
    assert_eq!(
        message(super::required_content_ref(&text("blake3:00"), "artifact")),
        "artifact must be a canonical content ref: invalid harness artifact: content ref must be blake3:<64 lowercase \
         hex chars>, got blake3:00"
    );
    let uppercase = valid.to_ascii_uppercase().replacen("BLAKE3:", "blake3:", 1);
    assert!(
        message(super::required_content_ref(&text(&uppercase), "artifact"))
            .ends_with(&alloc::format!("content ref must use lowercase hex chars, got {uppercase}"))
    );
}

// r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
#[test]
fn optional_content_refs_have_one_encoding() {
    let valid = reference(b"optional");
    let some = crate::build::optional_ref_value(Some(&valid));
    assert_eq!(super::optional_content_ref_string(&some, "head"), Ok(Some(valid.clone())));
    assert_eq!(super::optional_content_ref(&crate::build::optional_ref_value(None), "head"), Ok(None));
    assert_eq!(
        message(super::optional_content_ref(&text(&valid), "head")),
        "head must be <none> or <some \"blake3:...\">"
    );
    let not_string = wrap("some", crate::build::symbol("x"));
    assert_eq!(message(super::optional_content_ref(&not_string, "head")), "expected string for head");
    let invalid = crate::build::optional_ref_value(Some("blake3:zz"));
    assert_eq!(
        message(super::optional_content_ref(&invalid, "head")),
        "head must be a canonical content ref: invalid harness artifact: content ref must be blake3:<64 lowercase hex \
         chars>, got blake3:zz"
    );
}

#[test]
fn record_fields_read_one_wrapped_value() {
    let valid = reference(b"record");
    let named = wrap("name", text("alpha"));
    assert_eq!(super::record_string_field(&named, "name", "node name"), Ok(String::from("alpha")));
    assert_eq!(
        message(super::record_string_field(&named, "label", "node name")),
        "expected <label ...> with arity 1"
    );
    let wrapped = wrap("object", text(&valid));
    let parsed = super::record_content_ref(&wrapped, "object", "object").map(crate::identity::ContentRef::into_string);
    assert_eq!(parsed, Ok(valid.clone()));
    assert_eq!(super::record_content_ref_string(&wrapped, "object", "object"), Ok(valid));
}

#[test]
fn record_ref_sequences_are_bounded_and_canonical() {
    let refs = [reference(b"a"), reference(b"b")];
    let value = wrap("pins", crate::build::refs_sequence(&refs));
    assert_eq!(super::record_content_ref_strings(&value, "pins", "pins", 2), Ok(refs.to_vec()));
    assert_eq!(super::record_content_ref_sequence(&value, "pins", "pins", 2).map(|refs| refs.len()), Ok(2));
    assert_eq!(
        message(super::record_content_ref_sequence(&value, "pins", "pins", 1)),
        "pins count 2 exceeds maximum 1"
    );
    let mixed = wrap("pins", crate::build::sequence(alloc::vec![text(&refs[0]), crate::build::u64_value(3)]));
    assert_eq!(message(super::record_content_ref_strings(&mixed, "pins", "pins", 4)), "expected string for pins");
}

// r[verify molten.preserves_rail_toolkit.check_sets]
#[test]
fn check_sets_admit_unique_known_statuses() {
    let value = crate::build::checks_value(&[("shape", "pass"), ("policy", "deny"), ("budget", "fail")]);
    let checks = super::parse_checks_record(&value, 3, "gate").expect("valid checks");
    let names: alloc::vec::Vec<&str> = checks.iter().map(|check| check.name.as_str()).collect();
    assert_eq!(names, ["shape", "policy", "budget"]);
    assert_eq!(super::require_checks_present(&checks, &["policy", "shape"], "gate"), Ok(()));
    assert_eq!(message(super::require_checks_present(&checks, &["absent"], "gate")), "missing gate check absent");
    assert_eq!(message(super::parse_checks_record(&value, 2, "gate")), "checks count 3 exceeds maximum 2");
    let diagnostic = crate::build::checks_value(&[("shape", "diagnostic")]);
    assert_eq!(
        message(super::parse_checks_record(&diagnostic, 3, "gate")),
        "unsupported gate check status diagnostic"
    );
    let duplicate = crate::build::checks_value(&[("shape", "pass"), ("shape", "fail")]);
    assert_eq!(message(super::parse_checks_record(&duplicate, 3, "gate")), "duplicate gate check shape");
    let short = wrap("checks", crate::build::sequence(alloc::vec![wrap("check", text("x"))]));
    assert_eq!(message(super::parse_checks_record(&short, 3, "gate")), "expected <check ...> with arity 2");
}

#[test]
fn stable_ids_admit_only_the_stable_alphabet() {
    assert_eq!(super::validate_stable_id("node:alpha-1_v.2", "node"), Ok(()));
    assert_eq!(message(super::validate_stable_id("", "node")), "node cannot be empty");
    assert_eq!(
        message(super::validate_stable_id("bad/id", "node")),
        "node must contain only ASCII letters, digits, '_', '.', ':', or '-', got bad/id"
    );
}

#[test]
fn typed_ids_parse_format_and_reject_invalid_values() {
    let stable = super::StableId::parse("node:alpha-1").expect("stable id");
    assert_eq!(stable.as_str(), "node:alpha-1");
    assert_eq!(stable.into_string(), "node:alpha-1");
    let schema = super::SchemaId::parse("molten.schema.v1").map(|id| id.as_str().to_string());
    assert_eq!(schema, Ok("molten.schema.v1".into()));
    assert_eq!(super::ProfileId::parse("production").map(|id| id.as_str().to_string()), Ok("production".into()));
    let operation = super::OperationId::parse("storage.read").map(|id| id.as_str().to_string());
    assert_eq!(operation, Ok("storage.read".into()));
    assert_eq!(message(super::SchemaId::parse("")), "schema id cannot be empty");
    assert_eq!(
        message(super::ProfileId::parse("a b")).split(',').next(),
        Some("profile id must contain only ASCII letters")
    );
    assert_eq!(
        message(super::OperationId::parse("Storage.read")),
        "operation id first character must be lowercase ascii, got Storage.read"
    );
    assert_eq!(message(super::OperationId::parse("")), "operation id cannot be empty");
}

#[test]
fn closed_vocabularies_round_trip_and_reject_unknown_words() {
    for word in ["pass", "deny"] {
        assert_eq!(super::Decision::parse(word).map(super::Decision::as_str), Ok(word));
    }
    for word in ["pass", "fail", "deny", "diagnostic"] {
        assert_eq!(super::CheckStatus::parse(word).map(super::CheckStatus::as_str), Ok(word));
    }
    for word in ["idempotent", "deterministic", "effectful"] {
        assert_eq!(super::ReplayClass::parse(word).map(super::ReplayClass::as_str), Ok(word));
    }
    assert_eq!(message(super::Decision::parse("maybe")), "unsupported decision maybe");
    assert_eq!(message(super::CheckStatus::parse("unknown")), "unsupported check status unknown");
    assert_eq!(message(super::ReplayClass::parse("nondeterministic")), "unsupported replay class nondeterministic");
}
