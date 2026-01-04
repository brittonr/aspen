//! Property-based tests for the tuple encoding layer.
//!
//! These tests verify key invariants of the FoundationDB-compatible tuple encoding:
//!
//! 1. **Roundtrip**: pack(unpack(x)) == x for all valid tuples
//! 2. **Ordering**: pack(a) < pack(b) iff a < b (lexicographic preservation)
//! 3. **Prefix stability**: prefix(pack(a, b)) == pack(a) for any b
//! 4. **Type consistency**: Same input always produces same output
//!
//! # References
//!
//! - [FoundationDB Tuple Layer](https://github.com/apple/foundationdb/blob/main/design/tuple.md)
//! - [Proptest Guide](https://proptest-rs.github.io/proptest/proptest/index.html)

use proptest::prelude::*;

use crate::Element;
use crate::Subspace;
use crate::Tuple;

// =============================================================================
// Strategies for generating test data
// =============================================================================

/// Strategy for generating arbitrary Element values.
fn arb_element() -> impl Strategy<Value = Element> {
    prop_oneof![
        // Null
        Just(Element::Null),
        // Small strings (most common case)
        "[a-zA-Z0-9_]{0,20}".prop_map(Element::String),
        // Strings with special characters
        ".*".prop_map(|s: String| {
            // Filter to printable ASCII to avoid UTF-8 issues in tests
            Element::String(s.chars().filter(|c| c.is_ascii()).collect())
        }),
        // Bytes
        prop::collection::vec(any::<u8>(), 0..50).prop_map(Element::Bytes),
        // Integers across the full range
        any::<i64>().prop_map(Element::Int),
        // Small integers (common case)
        (-1000i64..1000i64).prop_map(Element::Int),
        // Booleans
        any::<bool>().prop_map(Element::Bool),
        // Floats (avoid NaN for equality testing)
        (-1e10f32..1e10f32).prop_map(Element::Float),
        // Doubles (avoid NaN for equality testing)
        (-1e100f64..1e100f64).prop_map(Element::Double),
    ]
}

/// Strategy for generating tuples with 0-5 elements.
fn arb_tuple() -> impl Strategy<Value = Tuple> {
    prop::collection::vec(arb_element(), 0..5).prop_map(|elements| {
        let mut tuple = Tuple::new();
        for elem in elements {
            tuple.push_mut(elem);
        }
        tuple
    })
}

/// Strategy for generating simple string tuples (for ordering tests).
fn arb_string_tuple() -> impl Strategy<Value = Tuple> {
    prop::collection::vec("[a-z]{1,5}", 1..4).prop_map(|strings| {
        let mut tuple = Tuple::new();
        for s in strings {
            tuple.push_mut(Element::String(s));
        }
        tuple
    })
}

/// Strategy for generating integer tuples (for ordering tests).
#[allow(dead_code)]
fn arb_int_tuple() -> impl Strategy<Value = Tuple> {
    prop::collection::vec(any::<i64>(), 1..4).prop_map(|ints| {
        let mut tuple = Tuple::new();
        for n in ints {
            tuple.push_mut(Element::Int(n));
        }
        tuple
    })
}

// =============================================================================
// Property Tests
// =============================================================================

proptest! {
    /// Property: pack followed by unpack is identity (roundtrip).
    ///
    /// This is the fundamental correctness property of any serialization format.
    #[test]
    fn prop_roundtrip(tuple in arb_tuple()) {
        let packed = tuple.pack();
        let unpacked = Tuple::unpack(&packed).expect("unpack should succeed");
        prop_assert_eq!(tuple, unpacked, "roundtrip failed");
    }

    /// Property: Integer encoding preserves ordering.
    ///
    /// For any two integers a < b, their packed representations should
    /// maintain the same ordering: pack(a) < pack(b).
    #[test]
    fn prop_int_ordering(a in any::<i64>(), b in any::<i64>()) {
        let tuple_a = Tuple::new().push(a);
        let tuple_b = Tuple::new().push(b);

        let packed_a = tuple_a.pack();
        let packed_b = tuple_b.pack();

        match a.cmp(&b) {
            std::cmp::Ordering::Less => prop_assert!(packed_a < packed_b, "ordering failed: {} < {} but {:?} >= {:?}", a, b, packed_a, packed_b),
            std::cmp::Ordering::Greater => prop_assert!(packed_a > packed_b, "ordering failed: {} > {} but {:?} <= {:?}", a, b, packed_a, packed_b),
            std::cmp::Ordering::Equal => prop_assert_eq!(packed_a, packed_b),
        }
    }

    /// Property: String encoding preserves ordering.
    ///
    /// For any two strings a < b (lexicographically), their packed
    /// representations should maintain the same ordering.
    #[test]
    fn prop_string_ordering(a in "[a-z]{0,10}", b in "[a-z]{0,10}") {
        let tuple_a = Tuple::new().push(&a as &str);
        let tuple_b = Tuple::new().push(&b as &str);

        let packed_a = tuple_a.pack();
        let packed_b = tuple_b.pack();

        match a.cmp(&b) {
            std::cmp::Ordering::Less => prop_assert!(packed_a < packed_b, "ordering failed: {:?} < {:?} but packed {:?} >= {:?}", a, b, packed_a, packed_b),
            std::cmp::Ordering::Greater => prop_assert!(packed_a > packed_b),
            std::cmp::Ordering::Equal => prop_assert_eq!(packed_a, packed_b),
        }
    }

    /// Property: Tuple ordering is lexicographic by element.
    ///
    /// Tuples should sort element-by-element, left to right.
    #[test]
    fn prop_tuple_ordering(a in arb_string_tuple(), b in arb_string_tuple()) {
        let packed_a = a.pack();
        let packed_b = b.pack();

        // Compare tuples element by element
        let mut expected = std::cmp::Ordering::Equal;
        for (elem_a, elem_b) in a.iter().zip(b.iter()) {
            match elem_a.cmp(elem_b) {
                std::cmp::Ordering::Equal => continue,
                ord => {
                    expected = ord;
                    break;
                }
            }
        }

        // If all compared elements are equal, shorter tuple comes first
        if expected == std::cmp::Ordering::Equal {
            expected = a.len().cmp(&b.len());
        }

        match expected {
            std::cmp::Ordering::Less => prop_assert!(packed_a < packed_b, "tuple ordering failed"),
            std::cmp::Ordering::Greater => prop_assert!(packed_a > packed_b),
            std::cmp::Ordering::Equal => prop_assert_eq!(packed_a, packed_b),
        }
    }

    /// Property: Prefix stability for tuples.
    ///
    /// If tuple A is a prefix of tuple B (A ++ C = B), then
    /// pack(A) is a prefix of pack(B).
    #[test]
    fn prop_prefix_stability(prefix in arb_string_tuple(), suffix in arb_string_tuple()) {
        let packed_prefix = prefix.pack();

        // Create the combined tuple
        let mut combined = prefix.clone();
        for elem in suffix.iter() {
            combined.push_mut(elem.clone());
        }
        let packed_combined = combined.pack();

        // The packed prefix should be a prefix of the packed combined tuple
        prop_assert!(
            packed_combined.starts_with(&packed_prefix),
            "prefix stability violated: packed prefix {:?} is not a prefix of packed combined {:?}",
            packed_prefix,
            packed_combined
        );
    }

    /// Property: Range queries capture all matching keys.
    ///
    /// For a tuple prefix P, the range (P.pack(), P.pack() + 0xFF) should
    /// contain all tuples that start with P.
    #[test]
    fn prop_range_captures_prefix(prefix in arb_string_tuple(), suffix in arb_string_tuple()) {
        let (start, end) = prefix.range();

        // Create a key that extends the prefix
        let mut key_tuple = prefix.clone();
        for elem in suffix.iter() {
            key_tuple.push_mut(elem.clone());
        }
        let key = key_tuple.pack();

        // The key should be in range
        prop_assert!(
            key >= start && key < end,
            "range query failed: key {:?} not in range [{:?}, {:?})",
            key,
            start,
            end
        );
    }

    /// Property: Subspace pack/unpack roundtrip.
    ///
    /// For any subspace S and key K, unpack(S, pack(S, K)) == K.
    #[test]
    fn prop_subspace_roundtrip(
        prefix in arb_string_tuple(),
        key in arb_string_tuple()
    ) {
        let subspace = Subspace::new(prefix);
        let packed = subspace.pack(&key);
        let unpacked = subspace.unpack(&packed).expect("unpack should succeed");
        prop_assert_eq!(key, unpacked, "subspace roundtrip failed");
    }

    /// Property: Subspace contains its own keys.
    ///
    /// For any subspace S and key K, S.contains(pack(S, K)) is true.
    #[test]
    fn prop_subspace_contains_own_keys(
        prefix in arb_string_tuple(),
        key in arb_string_tuple()
    ) {
        let subspace = Subspace::new(prefix);
        let packed = subspace.pack(&key);
        prop_assert!(
            subspace.contains(&packed),
            "subspace should contain its own keys"
        );
    }

    /// Property: Different subspaces don't contain each other's keys.
    ///
    /// For subspaces S1 and S2 with different prefixes, keys from S1
    /// should not be contained in S2 (unless S1's prefix is a prefix of S2's).
    #[test]
    fn prop_subspace_isolation(
        prefix1 in "[a-m]{1,3}",
        prefix2 in "[n-z]{1,3}",
        key in arb_string_tuple()
    ) {
        // Ensure prefixes are different (a-m vs n-z guarantees this)
        let sub1 = Subspace::new(Tuple::new().push(&prefix1 as &str));
        let sub2 = Subspace::new(Tuple::new().push(&prefix2 as &str));

        let key1 = sub1.pack(&key);
        let key2 = sub2.pack(&key);

        // Each subspace should contain only its own keys
        prop_assert!(sub1.contains(&key1));
        prop_assert!(!sub1.contains(&key2));
        prop_assert!(sub2.contains(&key2));
        prop_assert!(!sub2.contains(&key1));
    }

    /// Property: Nested subspaces maintain hierarchy.
    ///
    /// If S2 = S1.subspace(X), then S1.contains(key) for all keys in S2.
    #[test]
    fn prop_nested_subspace_hierarchy(
        outer_prefix in arb_string_tuple(),
        inner_suffix in arb_string_tuple(),
        key in arb_string_tuple()
    ) {
        let outer = Subspace::new(outer_prefix);
        let inner = outer.subspace(&inner_suffix);

        let inner_key = inner.pack(&key);

        // Key should be in both inner and outer subspace
        prop_assert!(inner.contains(&inner_key), "key not in inner subspace");
        prop_assert!(outer.contains(&inner_key), "key not in outer subspace");
    }

    /// Property: Integer special cases.
    ///
    /// Test boundary conditions for integer encoding.
    #[test]
    fn prop_int_boundaries(n in prop::sample::select(vec![
        i64::MIN,
        i64::MIN + 1,
        -1_000_000_000_000i64,
        -1_000_000_000i64,
        -1_000_000i64,
        -1_000i64,
        -256i64,
        -255i64,
        -128i64,
        -127i64,
        -1i64,
        0i64,
        1i64,
        127i64,
        128i64,
        255i64,
        256i64,
        1_000i64,
        1_000_000i64,
        1_000_000_000i64,
        1_000_000_000_000i64,
        i64::MAX - 1,
        i64::MAX,
    ])) {
        let tuple = Tuple::new().push(n);
        let packed = tuple.pack();
        let unpacked = Tuple::unpack(&packed).expect("unpack should succeed");
        prop_assert_eq!(tuple, unpacked, "boundary roundtrip failed for {}", n);
    }

    /// Property: Empty string and null byte handling.
    #[test]
    fn prop_special_strings(s in prop::sample::select(vec![
        String::new(),
        String::from("\x00"),
        String::from("\x00\x00"),
        String::from("a\x00b"),
        String::from("\x00a\x00"),
        String::from("test"),
    ])) {
        let tuple = Tuple::new().push(&s as &str);
        let packed = tuple.pack();
        let unpacked = Tuple::unpack(&packed).expect("unpack should succeed");
        prop_assert_eq!(tuple, unpacked, "special string roundtrip failed for {:?}", s);
    }

    /// Property: Deterministic encoding.
    ///
    /// The same tuple should always produce the same packed bytes.
    #[test]
    fn prop_deterministic(tuple in arb_tuple()) {
        let packed1 = tuple.pack();
        let packed2 = tuple.pack();
        prop_assert_eq!(packed1, packed2, "encoding should be deterministic");
    }
}

// =============================================================================
// Additional Non-Proptest Tests
// =============================================================================

#[test]
fn test_negative_integer_ordering() {
    // Specific test for negative integer ordering edge cases
    let values: Vec<i64> = vec![
        i64::MIN,
        i64::MIN + 1,
        -1_000_000_000_000,
        -1000,
        -256,
        -255,
        -128,
        -127,
        -2,
        -1,
        0,
        1,
        2,
        127,
        128,
        255,
        256,
        1000,
        1_000_000_000_000,
        i64::MAX - 1,
        i64::MAX,
    ];

    let packed: Vec<Vec<u8>> = values.iter().map(|&n| Tuple::new().push(n).pack()).collect();

    for i in 1..packed.len() {
        assert!(
            packed[i - 1] < packed[i],
            "ordering failed: {} < {} but {:?} >= {:?}",
            values[i - 1],
            values[i],
            packed[i - 1],
            packed[i]
        );
    }
}

#[test]
fn test_mixed_type_tuples() {
    // Test tuples with mixed types
    let tuple = Tuple::new().push("string").push(42i64).push(vec![1u8, 2, 3]).push(true).push(1.23456789f64);

    let packed = tuple.pack();
    let unpacked = Tuple::unpack(&packed).expect("unpack should succeed");

    assert_eq!(tuple.len(), unpacked.len());
    assert_eq!(tuple.get(0), unpacked.get(0));
    assert_eq!(tuple.get(1), unpacked.get(1));
    assert_eq!(tuple.get(2), unpacked.get(2));
    assert_eq!(tuple.get(3), unpacked.get(3));

    // Float comparison needs epsilon
    if let (Some(Element::Double(a)), Some(Element::Double(b))) = (tuple.get(4), unpacked.get(4)) {
        assert!((a - b).abs() < 1e-10);
    } else {
        panic!("expected Double elements");
    }
}

#[test]
fn test_subspace_range_disjoint() {
    // Test that sibling subspaces have disjoint ranges
    let users = Subspace::new(Tuple::new().push("users"));
    let orders = Subspace::new(Tuple::new().push("orders"));

    let (users_start, users_end) = users.range();
    let (orders_start, orders_end) = orders.range();

    // Since "orders" < "users" lexicographically, orders range should be before users
    assert!(orders_end <= users_start || users_end <= orders_start);
}

#[test]
fn test_nested_tuple_encoding() {
    // Test nested tuple encoding and decoding
    let inner = Tuple::new().push("inner").push(42i64);
    let outer = Tuple::new().push("outer").push(inner.clone());

    let packed = outer.pack();
    let unpacked = Tuple::unpack(&packed).expect("unpack should succeed");

    assert_eq!(unpacked.len(), 2);
    assert_eq!(unpacked.get(0), Some(&Element::String("outer".to_string())));
    assert_eq!(unpacked.get(1), Some(&Element::Tuple(inner)));
}
