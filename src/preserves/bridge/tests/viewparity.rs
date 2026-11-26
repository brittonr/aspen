//! Reader parity: every rail `IOValue` parser and its `molten_preserves_core::view` mirror accept
//! the same values, return the same results, and deny with the same `Failure` (class and message)
//! once the kernel's `ViewError` is converted.

const FIELD: &str = "field";
const CONTEXT: &str = "gate";
const MAX_ARITY: usize = 3;
const MAX_BOUND: usize = 4;

type Built = super::builder_parity::Built;
type Failure = crate::error::Failure;
type PackedValue = molten_preserves_core::model::PackedValue;
type ViewError = molten_preserves_core::view::ViewError;

fn agree<T: PartialEq + std::fmt::Debug>(rail: Result<T, Failure>, kernel: Result<T, ViewError>) {
    assert_eq!(rail, kernel.map_err(Failure::from));
}

fn packed(value: &super::Value<super::IoValue>) -> PackedValue {
    super::packed_from_iovalue(&super::value_to_iovalue(value)).expect("bridge conversion")
}

fn check_record_readers(built: &Built, label: &'static str, arity: u64) {
    let rail = super::simple_record_fields(&built.rail, label, arity)
        .map(|record| record.fields_iter().map(packed).collect::<Vec<_>>());
    let kernel = molten_preserves_core::view::simple_record_fields(&built.kernel, label, arity).map(<[_]>::to_vec);
    agree(rail, kernel);
    let value = super::Value::from(built.rail.clone());
    agree(
        super::record_string_field(&value, label, FIELD),
        molten_preserves_core::view::record_string_field(&built.kernel, label, FIELD),
    );
    agree(
        super::record_content_ref_string(&value, label, FIELD),
        molten_preserves_core::view::record_content_ref_string(&built.kernel, label, FIELD),
    );
}

fn check_field_readers(built: &Built, maximum: u64) {
    let value = super::Value::from(built.rail.clone());
    agree(
        super::required_string_field(&value, FIELD),
        molten_preserves_core::view::required_string_field(&built.kernel, FIELD),
    );
    agree(
        super::required_content_ref_string(&value, FIELD),
        molten_preserves_core::view::required_content_ref_string(&built.kernel, FIELD),
    );
    agree(
        super::optional_content_ref_string(&value, FIELD),
        molten_preserves_core::view::optional_content_ref_string(&built.kernel, FIELD),
    );
    let rail_sequence =
        super::required_sequence_field(&value, FIELD).map(|items| items.iter().map(packed).collect::<Vec<_>>());
    let kernel_sequence = molten_preserves_core::view::required_sequence_field(&built.kernel, FIELD);
    agree(rail_sequence, kernel_sequence.map(<[_]>::to_vec));
    for label in ["refs", "pins"] {
        agree(
            super::record_content_ref_strings(&value, label, FIELD, maximum),
            molten_preserves_core::view::record_content_ref_strings(&built.kernel, label, FIELD, maximum),
        );
    }
}

fn check_set_readers(built: &Built, maximum: u64) {
    let value = super::Value::from(built.rail.clone());
    let rail = super::parse_checks_record(&value, maximum, CONTEXT);
    let kernel = molten_preserves_core::view::parse_checks_record(&built.kernel, maximum, CONTEXT);
    let expected = ["shape", "policy"];
    if let (Ok(rail_checks), Ok(kernel_checks)) = (&rail, &kernel) {
        agree(
            super::require_checks_present(rail_checks, &expected, CONTEXT),
            molten_preserves_core::view::require_checks_present(kernel_checks, &expected, CONTEXT),
        );
    }
    agree(rail, kernel);
}

/// Accept/deny, results and messages agree for every reader over values built both ways, so the
/// two traversals cannot drift apart while the rail keeps its own `IOValue` walk.
// r[verify molten.preserves_rail_toolkit.parser_builders]
// r[verify molten.preserves_rail_toolkit.negative_shapes]
// r[verify molten.preserves_rail_toolkit.check_sets]
#[hegel::test(test_cases = 300)]
fn hegel_rail_parsers_and_kernel_views_agree(tc: hegel::TestCase) {
    let built = super::builder_parity::draw_built(&tc);
    let labels = super::builder_parity::LABELS;
    let label = labels[super::builder_parity::draw_index(&tc, labels.len() - 1)];
    let arity = u64::try_from(super::builder_parity::draw_index(&tc, MAX_ARITY)).expect("small arity");
    let maximum = u64::try_from(super::builder_parity::draw_index(&tc, MAX_BOUND)).expect("small bound");
    check_record_readers(&built, label, arity);
    check_field_readers(&built, maximum);
    check_set_readers(&built, maximum);
}

/// The parity checks also hold on inputs every reader accepts.
#[test]
fn readers_agree_on_accepting_inputs() {
    let refs = vec![super::content_ref_from_bytes(b"parity")];
    let wrapped = Built {
        rail: super::record("refs", vec![super::refs_sequence(&refs)]),
        kernel: molten_preserves_core::build::record("refs", vec![molten_preserves_core::build::refs_sequence(&refs)]),
    };
    let read = molten_preserves_core::view::record_content_ref_strings(&wrapped.kernel, "refs", FIELD, 1);
    assert_eq!(read, Ok(refs));
    check_record_readers(&wrapped, "refs", 1);
    check_field_readers(&wrapped, 1);
    let pairs = [("shape", "pass"), ("policy", "deny")];
    let checks = Built {
        rail: super::checks_value(&pairs),
        kernel: molten_preserves_core::build::checks_value(&pairs),
    };
    let parsed = molten_preserves_core::view::parse_checks_record(&checks.kernel, 2, CONTEXT).expect("checks parse");
    let present = molten_preserves_core::view::require_checks_present(&parsed, &["shape", "policy"], CONTEXT);
    assert_eq!(present, Ok(()));
    check_set_readers(&checks, 2);
}
