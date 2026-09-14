const FIRST_NONSQUARE: u128 = 2;
const SMALL_ROOT: u128 = 3;
const SMALL_SQUARE: u128 = SMALL_ROOT * SMALL_ROOT;

fn is_floor_root(value: u128, root: u128) -> bool {
    if root > u128::from(u64::MAX) {
        return false;
    }
    let Some(lower) = root.checked_mul(root) else {
        return false;
    };
    let Some(next) = root.checked_add(1) else {
        return false;
    };
    match next.checked_mul(next) {
        Some(upper) => lower <= value && value < upper,
        None => lower <= value,
    }
}

#[test]
fn small_values_keep_the_exact_floor() {
    for (value, expected) in [
        (0, 0),
        (1, 1),
        (FIRST_NONSQUARE, 1),
        (SMALL_SQUARE - 1, SMALL_ROOT - 1),
        (SMALL_SQUARE, SMALL_ROOT),
        (SMALL_SQUARE + 1, SMALL_ROOT),
    ] {
        assert_eq!(super::integer_sqrt(value), expected);
        assert!(is_floor_root(value, expected));
    }
}

#[test]
fn full_width_values_keep_the_exact_floor() {
    let largest_root = u128::from(u64::MAX);
    let largest_square = largest_root * largest_root;
    for (value, expected) in [
        (largest_square - 1, largest_root - 1),
        (largest_square, largest_root),
        (largest_square + 1, largest_root),
        (u128::MAX - 1, largest_root),
        (u128::MAX, largest_root),
    ] {
        assert_eq!(super::integer_sqrt(value), expected);
        assert!(is_floor_root(value, expected));
    }
}

#[test]
fn bit_boundaries_satisfy_independent_square_bounds() {
    for shift in 1..u128::BITS {
        let boundary = 1_u128 << shift;
        for value in [boundary - 1, boundary, boundary + 1] {
            assert!(is_floor_root(value, super::integer_sqrt(value)), "value={value}");
        }
    }
}

#[test]
fn square_bound_oracle_rejects_wrong_and_overflowing_candidates() {
    let largest_root = u128::from(u64::MAX);
    for (value, wrong_root) in [
        (0, 1),
        (1, 0),
        (FIRST_NONSQUARE, FIRST_NONSQUARE),
        (SMALL_SQUARE, SMALL_ROOT - 1),
        (SMALL_SQUARE, SMALL_ROOT + 1),
        (u128::MAX, largest_root - 1),
        (u128::MAX, largest_root + 1),
        (u128::MAX, u128::MAX),
    ] {
        assert!(!is_floor_root(value, wrong_root));
    }
}
