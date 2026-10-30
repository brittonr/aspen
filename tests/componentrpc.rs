#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![cfg(feature = "component-rpc")]

use molten::component_rpc;

// r[verify molten.component_rpc.bridge]
#[test]
fn exact_hello_string_wire_is_not_an_envelope_or_transport_echo() {
    let golden = b"\x0fhello from Rust";
    assert_eq!(component_rpc::bridge::decode_result(golden).expect("WIT string"), "hello from Rust");
    assert_eq!(component_rpc::bridge::encode_result("hello from Rust").expect("encode"), golden);
    for malformed in [
        b"hello from Rust".as_slice(),
        b"\x8f\x00hello from Rust",
        b"\x01\xff",
        b"\x00extra",
        b"\x80",
        b"\xff\xff\xff\xff\x10",
    ] {
        assert!(component_rpc::bridge::decode_result(malformed).is_err());
    }
}

#[test]
fn multibyte_string_lengths_are_minimal_and_bounded() {
    let text = "x".repeat(128);
    let encoded = component_rpc::bridge::encode_result(&text).expect("bounded result");
    assert_eq!(&encoded[..2], &[0x80, 0x01]);
    assert_eq!(component_rpc::bridge::decode_result(&encoded).expect("decode"), text);
    let mut truncated = encoded;
    truncated.pop();
    assert!(component_rpc::bridge::decode_result(&truncated).is_err());
    assert!(component_rpc::bridge::encode_result(&"x".repeat(component_rpc::MAX_BYTES)).is_err());
}

// r[verify molten.component_rpc.async_scope]
#[test]
fn world_function_and_all_async_paths_are_closed() {
    component_rpc::bridge::admit_shape(component_rpc::INTERFACE, component_rpc::FUNCTION, 0).expect("unary");
    for (world, function, paths) in [
        ("other:world/handler", "hello", 0),
        (component_rpc::INTERFACE, "other", 0),
        (component_rpc::INTERFACE, "hello", 1),
    ] {
        assert!(component_rpc::bridge::admit_shape(world, function, paths).is_err());
    }
}

// r[verify molten.component_rpc.telemetry]
#[test]
fn diagnostic_projection_excludes_payloads_and_has_a_finite_queue() {
    let receipt = format!("blake3:{}", blake3::hash(b"canonical receipt"));
    let mut diagnostics = component_rpc::diagnostics::Diagnostics::default();
    assert!(diagnostics.observe("raw secret credential", component_rpc::diagnostics::Status::Returned).is_err());
    for _ in 0..32 {
        diagnostics.observe(&receipt, component_rpc::diagnostics::Status::Returned).expect("capacity");
    }
    assert!(diagnostics.observe(&receipt, component_rpc::diagnostics::Status::Returned).is_err());
    for value in diagnostics.drain() {
        let fields =
            molten::codec::simple_record_fields(&value, "component-rpc-diagnostic-v1", 4).expect("closed fields");
        assert_eq!(molten::codec::required_string_field(&fields[0], "role").expect("role"), "diagnostic-only");
        assert_eq!(molten::codec::required_string_field(&fields[1], "receipt").expect("receipt"), receipt);
    }
    diagnostics
        .observe(&receipt, component_rpc::diagnostics::Status::Denied)
        .expect("capacity released");
}
