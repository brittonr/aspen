//! Deterministic bridge vectors: canonical bytes from text, non-canonical and over-budget denials
//! with their established failure classes, and `Embedded` values that cannot bypass the scans.

const EXAMPLE_TEXT: &str = "<example \"a\" [1 2 3]>";
const EXAMPLE_PACKED: &[u8] = b"\xb4\xb3\x07example\xb1\x01a\xb5\xb0\x01\x01\xb0\x01\x02\xb0\x01\x03\x84\x84";

fn strict_error(bytes: &[u8]) -> super::MoltenError {
    super::strict_canonical_decode(bytes).expect_err("strict decode denies")
}

#[test]
fn text_values_encode_to_the_kernel_wire_vector() {
    let value = super::parse_text(EXAMPLE_TEXT).expect("parse example");
    assert_eq!(super::canonical_bytes(&value).expect("canonical bytes"), EXAMPLE_PACKED);
    let decoded = super::strict_canonical_decode(EXAMPLE_PACKED).expect("strict decode");
    assert_eq!(decoded.value, value);
    assert_eq!(decoded.value_ref.as_str(), super::content_ref_from_bytes(EXAMPLE_PACKED));
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn non_canonical_bytes_are_denied_as_noncanonical() {
    for bytes in [
        &b"\xb6\xb0\x01\x02\xb0\x01\x01\x84"[..],
        b"\xb7\xb3\x01b\xb0\x01\x01\xb3\x01a\xb0\x01\x02\x84",
        b"\xb0\x02\x00\x01",
        b"\xb1\x80\x00",
        b"\x85\xb0\x01\x02\xb0\x01\x01",
        b"\x80\x80",
    ] {
        let error = strict_error(bytes);
        assert!(matches!(error, super::MoltenError::InvalidHarness(_)), "input {bytes:02x?}: {error}");
        assert!(error.to_string().contains("strict canonical Preserves decode failed"));
    }
}

#[test]
fn malformed_bytes_keep_the_preserves_failure_class() {
    for bytes in [
        &b"\x82"[..],
        b"\xb1\x05a",
        b"\xb1\x01\xff",
        b"\xb6\xb0\x01\x01\xb0\x01\x01\x84",
        b"\xb4\x84",
        b"\x87\x04\x3f\x80\x00\x00",
    ] {
        assert!(matches!(strict_error(bytes), super::MoltenError::Preserves(_)), "input {bytes:02x?}");
    }
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn overflowing_lengths_and_nesting_fail_closed() {
    let huge_length = b"\xb2\xff\xff\xff\xff\xff\xff\xff\xff\xff\x01";
    assert!(matches!(strict_error(huge_length), super::MoltenError::Preserves(_)));
    assert!(super::packed_bounds::preflight(huge_length, super::packed_bounds::MAX_VALUE_TOKENS).is_err());

    let depth = usize::try_from(molten_preserves_core::reader::IN_MEMORY_MAX_DEPTH).expect("depth fits") + 1;
    let nested = [vec![0xb5; depth], vec![0x84; depth]].concat();
    let error = strict_error(&nested);
    assert!(error.to_string().contains("nesting depth"), "{error}");
    assert!(super::packed_bounds::preflight(&nested, super::packed_bounds::MAX_VALUE_TOKENS).is_err());
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn embedded_payloads_decoded_from_bytes_cannot_bypass_scans() {
    let target = super::content_ref_from_bytes(b"embedded-bypass-target");
    let inner = super::record("outer", vec![
        super::IoValue::embedded(super::record("secret", Vec::new())),
        super::IoValue::embedded(super::record("command", Vec::new())),
        super::IoValue::embedded(super::IoValue::embedded(super::string(&target))),
    ]);
    let bytes = super::canonical_bytes(&inner).expect("canonical bytes");
    let decoded = super::strict_canonical_decode(&bytes).expect("strict decode").value;
    let secret = super::find_sensitive_structural_marker(&decoded).expect("scan").expect("secret");
    assert_eq!(secret.path, ["$", "field[0]", "embedded"]);
    let ambient = super::find_ambient_job_token(&decoded).expect("scan").expect("command");
    assert_eq!(ambient.path, ["$", "field[1]", "embedded"]);
    let found = super::find_structural_content_ref(&decoded, &target).expect("scan").expect("ref");
    assert_eq!(found.path, ["$", "field[2]", "embedded", "embedded"]);

    let uppercase = format!("blake3:{}", target.trim_start_matches("blake3:").to_ascii_uppercase());
    assert!(super::find_structural_content_ref(&decoded, &uppercase).is_err());
    let wrapped_some = super::IoValue::embedded(super::optional_ref_value(Some(&target)));
    assert!(super::optional_content_ref(&preserves::Value::from(wrapped_some), "optional ref").is_err());
}

#[test]
fn shell_scan_defaults_match_the_kernel() {
    assert_eq!(
        u64::try_from(super::DEFAULT_STRUCTURAL_SCAN_MAX_NODES).ok(),
        Some(molten_preserves_core::inspect::DEFAULT_MAX_NODES)
    );
    assert_eq!(
        u64::try_from(super::DEFAULT_STRUCTURAL_SCAN_MAX_DEPTH).ok(),
        Some(molten_preserves_core::inspect::DEFAULT_MAX_DEPTH)
    );
}
