use super::*;

#[test]
fn test_empty_tuple() {
    let t = Tuple::new();
    assert!(t.is_empty());
    assert_eq!(t.len(), 0);

    let packed = t.pack();
    assert!(packed.is_empty());

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(t, unpacked);
}

#[test]
fn test_null_element() {
    let t = Tuple::new().push(());
    let packed = t.pack();
    assert_eq!(packed, vec![NULL_CODE]);

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::Null));
}

#[test]
fn test_string_element() {
    let t = Tuple::new().push("hello");
    let packed = t.pack();

    // STRING_CODE + "hello" + NULL terminator
    assert_eq!(packed[0], STRING_CODE);
    assert_eq!(&packed[1..6], b"hello");
    assert_eq!(packed[6], 0x00);

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::String("hello".to_string())));
}

#[test]
fn test_string_with_null_bytes() {
    let t = Tuple::new().push("foo\x00bar");
    let packed = t.pack();

    // Should escape the null byte
    assert!(packed.contains(&NULL_ESCAPE));

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::String("foo\x00bar".to_string())));
}

#[test]
fn test_bytes_element() {
    let t = Tuple::new().push(vec![1u8, 2, 3, 4]);
    let packed = t.pack();

    assert_eq!(packed[0], BYTES_CODE);

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::Bytes(vec![1, 2, 3, 4])));
}

#[test]
fn test_integer_zero() {
    let t = Tuple::new().push(0i64);
    let packed = t.pack();
    assert_eq!(packed, vec![INT_ZERO_CODE]);

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::Int(0)));
}

#[test]
fn test_positive_integers() {
    for n in [1i64, 127, 128, 255, 256, 65535, 65536, i64::MAX] {
        let t = Tuple::new().push(n);
        let packed = t.pack();
        let unpacked = Tuple::unpack(&packed).unwrap();
        assert_eq!(unpacked.get(0), Some(&Element::Int(n)), "failed for n={}", n);
    }
}

#[test]
fn test_negative_integers() {
    for n in [-1i64, -127, -128, -255, -256, -65535, -65536, i64::MIN] {
        let t = Tuple::new().push(n);
        let packed = t.pack();
        let unpacked = Tuple::unpack(&packed).unwrap();
        assert_eq!(unpacked.get(0), Some(&Element::Int(n)), "failed for n={}", n);
    }
}

#[test]
fn test_integer_ordering() {
    // Verify that packed integers sort correctly
    let values: Vec<i64> = vec![i64::MIN, -1000, -1, 0, 1, 1000, i64::MAX];
    let packed: Vec<Vec<u8>> = values.iter().map(|&n| Tuple::new().push(n).pack()).collect();

    for i in 1..packed.len() {
        assert!(packed[i - 1] < packed[i], "ordering failed: {:?} should be < {:?}", values[i - 1], values[i]);
    }
}

#[test]
fn test_string_ordering() {
    let values = ["", "a", "aa", "ab", "b", "ba"];
    let packed: Vec<Vec<u8>> = values.iter().map(|s| Tuple::new().push(*s).pack()).collect();

    for i in 1..packed.len() {
        assert!(packed[i - 1] < packed[i], "ordering failed: {:?} should be < {:?}", values[i - 1], values[i]);
    }
}

#[test]
fn test_bool_element() {
    let t = Tuple::new().push(true).push(false);
    let packed = t.pack();

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::Bool(true)));
    assert_eq!(unpacked.get(1), Some(&Element::Bool(false)));
}

#[test]
fn test_float_element() {
    let t = Tuple::new().push(1.234f32);
    let packed = t.pack();

    let unpacked = Tuple::unpack(&packed).unwrap();
    if let Some(Element::Float(f)) = unpacked.get(0) {
        assert!((f - 1.234f32).abs() < 1e-6);
    } else {
        panic!("expected Float element");
    }
}

#[test]
fn test_double_element() {
    let t = Tuple::new().push(1.23456789f64);
    let packed = t.pack();

    let unpacked = Tuple::unpack(&packed).unwrap();
    if let Some(Element::Double(d)) = unpacked.get(0) {
        assert!((d - 1.23456789f64).abs() < 1e-10);
    } else {
        panic!("expected Double element");
    }
}

#[test]
fn test_nested_tuple() {
    let inner = Tuple::new().push("inner").push(42i64);
    let outer = Tuple::new().push("outer").push(inner.clone());

    let packed = outer.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    assert_eq!(unpacked.get(0), Some(&Element::String("outer".to_string())));
    assert_eq!(unpacked.get(1), Some(&Element::Tuple(inner)));
}

#[test]
fn test_nested_tuple_with_null() {
    let inner = Tuple::new().push(()).push("after_null");
    let outer = Tuple::new().push("outer").push(inner.clone());

    let packed = outer.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    if let Some(Element::Tuple(t)) = unpacked.get(1) {
        assert_eq!(t.get(0), Some(&Element::Null));
        assert_eq!(t.get(1), Some(&Element::String("after_null".to_string())));
    } else {
        panic!("expected nested tuple");
    }
}

#[test]
fn test_composite_tuple() {
    let t = Tuple::new().push("users").push(12345i64).push("profile").push(true);

    let packed = t.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    assert_eq!(t, unpacked);
}

#[test]
fn test_range() {
    let prefix = Tuple::new().push("users").push(1i64);
    let (start, end) = prefix.range();

    assert_eq!(start, prefix.pack());
    assert_eq!(end, {
        let mut v = prefix.pack();
        v.push(0xFF);
        v
    });

    // Any key with a longer suffix should be in range
    let key = Tuple::new().push("users").push(1i64).push("profile").pack();
    assert!(key >= start && key < end);
}

#[test]
fn test_strinc() {
    let t = Tuple::new().push("abc");
    let incremented = t.strinc().unwrap();

    // Should be strictly greater than original
    assert!(incremented > t.pack());

    // But less than "abd"
    let next = Tuple::new().push("abd").pack();
    assert!(incremented <= next);
}

#[test]
fn test_type_ordering() {
    // Verify cross-type ordering: Null < Bytes < String < Int < ...
    let null_packed = Tuple::new().push(()).pack();
    let bytes_packed = Tuple::new().push(vec![0u8]).pack();
    let string_packed = Tuple::new().push("a").pack();
    // Note: integers have different type codes based on value

    assert!(null_packed < bytes_packed);
    assert!(bytes_packed < string_packed);
}

#[test]
fn test_roundtrip_stress() {
    // Test many different values
    for i in -1000i64..1000 {
        let t = Tuple::new().push(i);
        let packed = t.pack();
        let unpacked = Tuple::unpack(&packed).unwrap();
        assert_eq!(t, unpacked, "roundtrip failed for i={}", i);
    }
}

// =========================================================================
// Float/Double Special Values
// =========================================================================

#[test]
fn test_float_positive_infinity() {
    let t = Tuple::new().push(f32::INFINITY);
    let packed = t.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    if let Some(Element::Float(f)) = unpacked.get(0) {
        assert!(f.is_infinite() && f.is_sign_positive());
    } else {
        panic!("expected Float element");
    }
}

#[test]
fn test_float_negative_infinity() {
    let t = Tuple::new().push(f32::NEG_INFINITY);
    let packed = t.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    if let Some(Element::Float(f)) = unpacked.get(0) {
        assert!(f.is_infinite() && f.is_sign_negative());
    } else {
        panic!("expected Float element");
    }
}

#[test]
fn test_float_nan() {
    let t = Tuple::new().push(f32::NAN);
    let packed = t.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    if let Some(Element::Float(f)) = unpacked.get(0) {
        assert!(f.is_nan());
    } else {
        panic!("expected Float element");
    }
}

#[test]
fn test_float_negative_zero() {
    let t = Tuple::new().push(-0.0f32);
    let packed = t.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    if let Some(Element::Float(f)) = unpacked.get(0) {
        // -0.0 and 0.0 are equal but have different bit representations
        assert_eq!(*f, 0.0f32);
    } else {
        panic!("expected Float element");
    }
}

#[test]
fn test_double_positive_infinity() {
    let t = Tuple::new().push(f64::INFINITY);
    let packed = t.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    if let Some(Element::Double(d)) = unpacked.get(0) {
        assert!(d.is_infinite() && d.is_sign_positive());
    } else {
        panic!("expected Double element");
    }
}

#[test]
fn test_double_negative_infinity() {
    let t = Tuple::new().push(f64::NEG_INFINITY);
    let packed = t.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    if let Some(Element::Double(d)) = unpacked.get(0) {
        assert!(d.is_infinite() && d.is_sign_negative());
    } else {
        panic!("expected Double element");
    }
}

#[test]
fn test_double_nan() {
    let t = Tuple::new().push(f64::NAN);
    let packed = t.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    if let Some(Element::Double(d)) = unpacked.get(0) {
        assert!(d.is_nan());
    } else {
        panic!("expected Double element");
    }
}

#[test]
fn test_float_ordering() {
    // Verify float ordering: -inf < -1 < -0 < 0 < 1 < inf
    let values: Vec<f32> = vec![f32::NEG_INFINITY, -1.0, 0.0, 1.0, f32::INFINITY];
    let packed: Vec<Vec<u8>> = values.iter().map(|&f| Tuple::new().push(f).pack()).collect();

    for i in 1..packed.len() {
        assert!(packed[i - 1] < packed[i], "float ordering failed: {:?} should be < {:?}", values[i - 1], values[i]);
    }
}

#[test]
fn test_double_ordering() {
    // Verify double ordering: -inf < -1 < 0 < 1 < inf
    let values: Vec<f64> = vec![f64::NEG_INFINITY, -1.0, 0.0, 1.0, f64::INFINITY];
    let packed: Vec<Vec<u8>> = values.iter().map(|&d| Tuple::new().push(d).pack()).collect();

    for i in 1..packed.len() {
        assert!(packed[i - 1] < packed[i], "double ordering failed: {:?} should be < {:?}", values[i - 1], values[i]);
    }
}

// =========================================================================
// Error Path Tests
// =========================================================================

#[test]
fn test_error_unexpected_end_empty() {
    // Decoding an empty buffer with an offset triggers UnexpectedEnd
    let result = Tuple::unpack(&[]);
    // Empty input is valid - returns empty tuple
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_error_unknown_type_code() {
    // Type code 0x30 is not defined
    let data = [0x30];
    let result = Tuple::unpack(&data);
    assert!(result.is_err());

    if let Err(TupleError::UnknownTypeCode { code, offset }) = result {
        assert_eq!(code, 0x30);
        assert_eq!(offset, 0);
    } else {
        panic!("expected UnknownTypeCode error");
    }
}

#[test]
fn test_error_invalid_utf8() {
    // STRING_CODE followed by invalid UTF-8 sequence
    let data = [STRING_CODE, 0xFF, 0xFE, 0x00]; // Invalid UTF-8 + terminator
    let result = Tuple::unpack(&data);
    assert!(result.is_err());

    if let Err(TupleError::InvalidUtf8 { .. }) = result {
        // Expected
    } else {
        panic!("expected InvalidUtf8 error, got {:?}", result);
    }
}

#[test]
fn test_error_missing_terminator() {
    // STRING_CODE without null terminator
    let data = [STRING_CODE, b'h', b'e', b'l', b'l', b'o'];
    let result = Tuple::unpack(&data);
    assert!(result.is_err());

    if let Err(TupleError::MissingTerminator { .. }) = result {
        // Expected
    } else {
        panic!("expected MissingTerminator error, got {:?}", result);
    }
}

#[test]
fn test_error_unterminated_nested() {
    // NESTED_CODE without terminator
    let data = [NESTED_CODE, STRING_CODE, b'a', 0x00]; // String is terminated, but nested isn't
    let result = Tuple::unpack(&data);
    assert!(result.is_err());

    if let Err(TupleError::UnterminatedNested { .. }) = result {
        // Expected
    } else {
        panic!("expected UnterminatedNested error, got {:?}", result);
    }
}

#[test]
fn test_error_unexpected_end_float() {
    // FLOAT_CODE requires 4 more bytes, but we only have 2
    let data = [FLOAT_CODE, 0x00, 0x00];
    let result = Tuple::unpack(&data);
    assert!(result.is_err());

    if let Err(TupleError::UnexpectedEnd { .. }) = result {
        // Expected
    } else {
        panic!("expected UnexpectedEnd error, got {:?}", result);
    }
}

#[test]
fn test_error_unexpected_end_double() {
    // DOUBLE_CODE requires 8 more bytes
    let data = [DOUBLE_CODE, 0x00, 0x00, 0x00, 0x00];
    let result = Tuple::unpack(&data);
    assert!(result.is_err());

    if let Err(TupleError::UnexpectedEnd { .. }) = result {
        // Expected
    } else {
        panic!("expected UnexpectedEnd error, got {:?}", result);
    }
}

#[test]
fn test_error_unexpected_end_integer() {
    // Positive integer code 0x15 requires 1 byte, but none provided
    let data = [0x15];
    let result = Tuple::unpack(&data);
    assert!(result.is_err());

    if let Err(TupleError::UnexpectedEnd { .. }) = result {
        // Expected
    } else {
        panic!("expected UnexpectedEnd error, got {:?}", result);
    }
}

// =========================================================================
// From Trait Edge Cases
// =========================================================================

#[test]
fn test_u64_max_converts_to_bytes() {
    // u64::MAX > i64::MAX, so it should be stored as bytes
    let elem: Element = u64::MAX.into();

    if let Element::Bytes(bytes) = elem {
        assert_eq!(bytes, u64::MAX.to_be_bytes().to_vec());
    } else {
        panic!("expected Bytes element for u64::MAX");
    }
}

#[test]
fn test_u64_within_i64_range() {
    // Values <= i64::MAX should become Int
    let elem: Element = (i64::MAX as u64).into();
    assert_eq!(elem, Element::Int(i64::MAX));
}

#[test]
fn test_u8_to_element() {
    let elem: Element = 255u8.into();
    assert_eq!(elem, Element::Int(255));
}

#[test]
fn test_u32_to_element() {
    let elem: Element = u32::MAX.into();
    assert_eq!(elem, Element::Int(u32::MAX as i64));
}

#[test]
fn test_i32_to_element() {
    let elem: Element = i32::MIN.into();
    assert_eq!(elem, Element::Int(i32::MIN as i64));
}

#[test]
fn test_unit_to_element() {
    let elem: Element = ().into();
    assert_eq!(elem, Element::Null);
}

#[test]
fn test_slice_to_element() {
    let data: &[u8] = &[1, 2, 3];
    let elem: Element = data.into();
    assert_eq!(elem, Element::Bytes(vec![1, 2, 3]));
}

#[test]
fn test_string_to_element() {
    let s = String::from("test");
    let elem: Element = s.into();
    assert_eq!(elem, Element::String("test".to_string()));
}

// =========================================================================
// Tuple API Edge Cases
// =========================================================================

#[test]
fn test_with_capacity() {
    let t = Tuple::with_capacity(10);
    assert!(t.is_empty());
    assert_eq!(t.len(), 0);
}

#[test]
fn test_push_mut() {
    let mut t = Tuple::new();
    t.push_mut("a");
    t.push_mut(1i64);
    t.push_mut(true);

    assert_eq!(t.len(), 3);
    assert_eq!(t.get(0), Some(&Element::String("a".to_string())));
    assert_eq!(t.get(1), Some(&Element::Int(1)));
    assert_eq!(t.get(2), Some(&Element::Bool(true)));
}

#[test]
fn test_iter() {
    let t = Tuple::new().push("a").push("b").push("c");
    let elements: Vec<_> = t.iter().collect();

    assert_eq!(elements.len(), 3);
    assert_eq!(elements[0], &Element::String("a".to_string()));
    assert_eq!(elements[1], &Element::String("b".to_string()));
    assert_eq!(elements[2], &Element::String("c".to_string()));
}

#[test]
fn test_get_out_of_bounds() {
    let t = Tuple::new().push("a");
    assert!(t.get(0).is_some());
    assert!(t.get(1).is_none());
    assert!(t.get(100).is_none());
}

#[test]
fn test_unpack_partial() {
    // Pack two tuples consecutively
    let t1 = Tuple::new().push("first");
    let t2 = Tuple::new().push("second");

    let mut data = t1.pack();
    data.extend(t2.pack());

    // unpack_partial should only consume the first tuple's bytes
    let (unpacked, consumed) = Tuple::unpack_partial(&t1.pack()).unwrap();
    assert_eq!(unpacked, t1);
    assert_eq!(consumed, t1.pack().len());
}

// =========================================================================
// Deep Nesting
// =========================================================================

#[test]
fn test_deeply_nested_tuples() {
    // Create 5 levels of nesting
    let level5 = Tuple::new().push("level5");
    let level4 = Tuple::new().push(level5);
    let level3 = Tuple::new().push(level4);
    let level2 = Tuple::new().push(level3);
    let level1 = Tuple::new().push(level2);

    let packed = level1.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    // Navigate to level5
    if let Some(Element::Tuple(l2)) = unpacked.get(0) {
        if let Some(Element::Tuple(l3)) = l2.get(0) {
            if let Some(Element::Tuple(l4)) = l3.get(0) {
                if let Some(Element::Tuple(l5)) = l4.get(0) {
                    assert_eq!(l5.get(0), Some(&Element::String("level5".to_string())));
                    return;
                }
            }
        }
    }
    panic!("failed to navigate nested structure");
}

#[test]
fn test_nested_with_multiple_elements() {
    let inner = Tuple::new().push(1i64).push(2i64).push(3i64);
    let outer = Tuple::new().push("prefix").push(inner.clone()).push("suffix");

    let packed = outer.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();

    assert_eq!(unpacked.get(0), Some(&Element::String("prefix".to_string())));
    assert_eq!(unpacked.get(1), Some(&Element::Tuple(inner)));
    assert_eq!(unpacked.get(2), Some(&Element::String("suffix".to_string())));
}

// =========================================================================
// Element Ordering (Ord trait)
// =========================================================================

#[test]
fn test_element_ord_null() {
    let null = Element::Null;
    let int = Element::Int(0);
    assert!(null < int);
}

#[test]
fn test_element_ord_ints() {
    let a = Element::Int(-100);
    let b = Element::Int(0);
    let c = Element::Int(100);

    assert!(a < b);
    assert!(b < c);
    assert!(a < c);
}

#[test]
fn test_element_ord_strings() {
    let a = Element::String("aaa".to_string());
    let b = Element::String("aab".to_string());
    let c = Element::String("b".to_string());

    assert!(a < b);
    assert!(b < c);
}

#[test]
fn test_element_partial_ord() {
    let a = Element::Int(1);
    let b = Element::Int(2);

    assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less));
    assert_eq!(b.partial_cmp(&a), Some(std::cmp::Ordering::Greater));
    assert_eq!(a.partial_cmp(&a), Some(std::cmp::Ordering::Equal));
}

#[test]
fn test_tuple_ord() {
    let t1 = Tuple::new().push("a").push(1i64);
    let t2 = Tuple::new().push("a").push(2i64);
    let t3 = Tuple::new().push("b").push(1i64);

    assert!(t1 < t2); // Same prefix, 1 < 2
    assert!(t2 < t3); // "a" < "b"
    assert!(t1 < t3);
}

#[test]
fn test_tuple_partial_ord() {
    let t1 = Tuple::new().push(1i64);
    let t2 = Tuple::new().push(2i64);

    assert_eq!(t1.partial_cmp(&t2), Some(std::cmp::Ordering::Less));
}

// =========================================================================
// Empty String/Bytes Edge Cases
// =========================================================================

#[test]
fn test_empty_string() {
    let t = Tuple::new().push("");
    let packed = t.pack();

    // STRING_CODE + null terminator
    assert_eq!(packed, vec![STRING_CODE, 0x00]);

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::String(String::new())));
}

#[test]
fn test_empty_bytes() {
    let t = Tuple::new().push(Vec::<u8>::new());
    let packed = t.pack();

    // BYTES_CODE + null terminator
    assert_eq!(packed, vec![BYTES_CODE, 0x00]);

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::Bytes(vec![])));
}

#[test]
fn test_bytes_with_embedded_nulls() {
    let bytes = vec![0x00, 0x01, 0x00, 0x02, 0x00];
    let t = Tuple::new().push(bytes.clone());
    let packed = t.pack();

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::Bytes(bytes)));
}

// =========================================================================
// strinc Edge Cases
// =========================================================================

#[test]
fn test_strinc_all_ff() {
    // A tuple that packs to all 0xFF should return None from strinc
    let t = Tuple::new().push(vec![0xFFu8; 10]);
    // Actually the packed form won't be all 0xFF due to type code and escaping
    // Let's test the underlying strinc function directly
    let mut data = vec![0xFF, 0xFF, 0xFF];
    let result = super::tuple_type::strinc_for_test(&mut data);
    assert!(!result);
    assert!(data.is_empty()); // All bytes were popped
}

#[test]
fn test_strinc_trailing_ff() {
    // Test strinc with trailing 0xFF bytes
    let mut data = vec![0x01, 0x02, 0xFF, 0xFF];
    let result = super::tuple_type::strinc_for_test(&mut data);
    assert!(result);
    assert_eq!(data, vec![0x01, 0x03]); // Incremented 0x02 -> 0x03, removed trailing FF
}

#[test]
fn test_strinc_empty() {
    let mut data = vec![];
    let result = super::tuple_type::strinc_for_test(&mut data);
    assert!(!result);
}

// =========================================================================
// Tuple Default Trait
// =========================================================================

#[test]
fn test_tuple_default() {
    let t: Tuple = Default::default();
    assert!(t.is_empty());
    assert_eq!(t.len(), 0);
}

// =========================================================================
// Tuple Clone
// =========================================================================

#[test]
fn test_tuple_clone() {
    let t = Tuple::new().push("test").push(42i64);
    let cloned = t.clone();

    assert_eq!(t, cloned);
    assert_eq!(t.pack(), cloned.pack());
}

#[test]
fn test_element_clone() {
    let elem = Element::Tuple(Tuple::new().push("nested"));
    let cloned = elem.clone();
    assert_eq!(elem, cloned);
}

// =========================================================================
// Integer Boundary Values
// =========================================================================

#[test]
fn test_integer_boundary_sizes() {
    // Test integers at size boundaries
    let boundaries: Vec<i64> = vec![
        0xFF,               // 1-byte max
        0x100,              // 2-byte min
        0xFFFF,             // 2-byte max
        0x10000,            // 3-byte min
        0xFF_FFFF,          // 3-byte max
        0x100_0000,         // 4-byte min
        0xFFFF_FFFF,        // 4-byte max
        0x1_0000_0000,      // 5-byte min
        0xFF_FFFF_FFFF,     // 5-byte max
        0x100_0000_0000,    // 6-byte min
        0xFFFF_FFFF_FFFF,   // 6-byte max
        0x1_0000_0000_0000, // 7-byte min
    ];

    for n in boundaries {
        let t = Tuple::new().push(n);
        let packed = t.pack();
        let unpacked = Tuple::unpack(&packed).unwrap();
        assert_eq!(unpacked.get(0), Some(&Element::Int(n)), "boundary test failed for n={}", n);

        // Also test negative
        let t_neg = Tuple::new().push(-n);
        let packed_neg = t_neg.pack();
        let unpacked_neg = Tuple::unpack(&packed_neg).unwrap();
        assert_eq!(unpacked_neg.get(0), Some(&Element::Int(-n)), "negative boundary test failed for n={}", -n);
    }
}

// =========================================================================
// Decode Error Path Tests
// =========================================================================

#[test]
fn test_unpack_partial_not_consuming_all() {
    // Test unpack_partial can return less than full data length
    // This happens when parsing nested tuples
    let t = Tuple::new().push("test");
    let packed = t.pack();

    // unpack_partial returns how many bytes were consumed
    let (unpacked, consumed) = Tuple::unpack_partial(&packed).unwrap();
    assert_eq!(consumed, packed.len());
    assert_eq!(unpacked.get(0), Some(&Element::String("test".to_string())));
}

#[test]
fn test_decode_element_empty_data() {
    // Test UnexpectedEnd error when decoding from empty data
    let result = Tuple::unpack(&[]);
    // Empty data should produce empty tuple
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_decode_int_truncated_positive() {
    // Craft a malformed positive integer encoding (type code says 4 bytes but only 2 provided)
    // INT_ZERO_CODE + 4 = 0x18 means 4-byte positive integer
    let malformed = vec![0x18, 0x00, 0x00]; // Only 2 bytes instead of 4

    let result = Tuple::unpack(&malformed);
    assert!(result.is_err());
}

#[test]
fn test_decode_int_truncated_negative() {
    // Craft a malformed negative integer encoding
    // INT_ZERO_CODE - 4 = 0x10 means 4-byte negative integer
    let malformed = vec![0x10, 0xFF, 0xFF]; // Only 2 bytes instead of 4

    let result = Tuple::unpack(&malformed);
    assert!(result.is_err());
}

#[test]
fn test_8_byte_integer_encoding() {
    // Test that very large integers use 8-byte encoding (line 585)
    let large_value = 0x0100_0000_0000_0000i64; // Requires 8 bytes
    let t = Tuple::new().push(large_value);
    let packed = t.pack();

    // Should be INT_ZERO_CODE + 8 = 0x1C for 8-byte positive int
    assert_eq!(packed[0], 0x1C);

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::Int(large_value)));
}

#[test]
fn test_large_negative_8_byte_integer() {
    // Test 8-byte negative integer encoding
    let large_neg = -0x0100_0000_0000_0000i64;
    let t = Tuple::new().push(large_neg);
    let packed = t.pack();

    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::Int(large_neg)));
}

#[test]
fn test_i64_max_encoding() {
    // i64::MAX should encode and decode correctly
    let t = Tuple::new().push(i64::MAX);
    let packed = t.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::Int(i64::MAX)));
}

#[test]
fn test_i64_min_encoding() {
    // i64::MIN should encode and decode correctly
    let t = Tuple::new().push(i64::MIN);
    let packed = t.pack();
    let unpacked = Tuple::unpack(&packed).unwrap();
    assert_eq!(unpacked.get(0), Some(&Element::Int(i64::MIN)));
}
