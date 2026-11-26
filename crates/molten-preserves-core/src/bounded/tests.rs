use alloc::string::ToString;

use super::Count;
use super::PushLimited;
use crate::fault::Classified;

const TEST_LIMIT: usize = 2;
const TEST_OVERFLOW_LEFT: usize = usize::MAX;
const TEST_ONE: usize = 1;

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn push_allows_exact_limit_and_denies_one_past_without_mutation() {
    let mut values = alloc::vec::Vec::new();
    super::push(&mut values, "first", TEST_LIMIT, "test values").expect("first push");
    super::push(&mut values, "second", TEST_LIMIT, "test values").expect("second push");
    assert_eq!(values, ["first", "second"]);

    let before = values.clone();
    let error = super::push(&mut values, "third", TEST_LIMIT, "test values").expect_err("one-past-limit push denies");
    assert_eq!(error.to_string(), "test values count 3 exceeds maximum 2");
    assert_eq!(values, before);
}

#[test]
fn push_limited_matches_push_and_takes_wire_bounds() {
    let mut values = alloc::vec![1_u8];
    values.push_limited(2, 2_u64, "limited").expect("push within limit");
    let error = values.push_limited(3, 2_u64, "limited").expect_err("push past limit denies");
    assert_eq!(error, super::BoundError::CountExceeded {
        label: "limited".to_string(),
        count: 3,
        maximum: 2,
    });
    assert_eq!(values, [1, 2]);
}

#[test]
fn checked_count_sum_denies_arithmetic_overflow() {
    let error = super::checked_count_sum(TEST_OVERFLOW_LEFT, TEST_ONE, TEST_OVERFLOW_LEFT, "overflow values")
        .expect_err("overflow denies");
    assert_eq!(error.to_string(), "overflow values count overflow");
    let wire_overflow = super::checked_count_sum(u64::MAX, 1_u64, u64::MAX, "wire").expect_err("u64 overflow denies");
    assert_eq!(wire_overflow.to_string(), "wire count overflow");
    assert_eq!(super::checked_count_sum(TEST_ONE, TEST_ONE, TEST_LIMIT, "sum"), Ok(2));
}

#[test]
fn ensure_count_at_most_allows_the_maximum_and_denies_past_it() {
    assert_eq!(super::ensure_count_at_most(TEST_LIMIT, TEST_LIMIT, "items"), Ok(()));
    assert_eq!(super::ensure_count_at_most(2_u64, TEST_LIMIT, "items"), Ok(()));
    let error = super::ensure_count_at_most(TEST_LIMIT + 1, 2_u64, "items").expect_err("past maximum");
    assert_eq!(error.to_string(), "items count 3 exceeds maximum 2");
}

#[test]
fn extend_denies_before_appending_any_item() {
    let mut values = alloc::vec!["existing".to_string()];
    let incoming = alloc::vec!["first".to_string(), "second".to_string()];
    let before = values.clone();
    let error =
        super::extend(&mut values, &incoming, TEST_LIMIT, "extend values").expect_err("extend over limit denies");
    assert_eq!(error.to_string(), "extend values count 3 exceeds maximum 2");
    assert_eq!(values, before);
    super::extend(&mut values, &incoming[..1], TEST_LIMIT, "extend values").expect("extend within limit");
    assert_eq!(values, ["existing", "first"]);
}

#[test]
fn diagnostic_sink_uses_push_behavior() {
    let mut diagnostics = alloc::vec::Vec::new();
    {
        let mut sink = super::DiagnosticSink::new(&mut diagnostics, TEST_LIMIT, "diagnostics");
        sink.push("first").expect("first diagnostic");
        sink.push("second").expect("second diagnostic");
        let error = sink.push("third").expect_err("diagnostic overflow denies");
        assert_eq!(error.to_string(), "diagnostics count 3 exceeds maximum 2");
    }
    assert_eq!(diagnostics, ["first", "second"]);
}

#[test]
fn count_conversions_accept_fitting_values() {
    assert_eq!(u64::from(u32::MAX).to_usize("bound"), Ok(usize::try_from(u32::MAX).expect("fits")));
    assert_eq!(TEST_LIMIT.to_u64("count"), Ok(2));
    assert_eq!(TEST_LIMIT.to_usize("count"), Ok(TEST_LIMIT));
    assert_eq!(7_u64.to_u64("count"), Ok(7));
}

#[test]
fn conversion_denials_render_the_shell_messages() {
    let usize_overflow = super::BoundError::UsizeOverflow {
        label: "record arity".to_string(),
        value: u64::MAX,
    };
    assert_eq!(usize_overflow.to_string(), "record arity 18446744073709551615 does not fit usize");
    let u64_overflow = super::BoundError::U64Overflow {
        label: "boundary schema arity".to_string(),
        value: "7".to_string(),
    };
    assert_eq!(u64_overflow.to_string(), "boundary schema arity 7 does not fit u64");
    assert_eq!(usize_overflow.failure_kind(), crate::fault::FailureKind::InvalidHarness);
}
