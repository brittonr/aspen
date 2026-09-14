const MESSAGE_VARIANT_COUNT: usize = 8;

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn all_message_variants_preserve_pre_change_bytes_values_references_and_readback() {
    let mut variants = Vec::with_capacity(MESSAGE_VARIANT_COUNT);
    for case in super::cases() {
        let before = case.envelope.clone();
        let (bytes, reference) = fixture(case.name);
        let canonical = super::super::canonical_replica_message(&case.envelope).expect("canonical message");
        let expected_value = crate::preserves_rail::strict_canonical_decode(bytes).expect("fixed canonical value");
        assert_eq!(canonical.bytes, bytes, "{}", case.name);
        assert_eq!(canonical.value, expected_value.value, "{}", case.name);
        assert_eq!(canonical.envelope_ref, reference, "{}", case.name);
        assert_eq!(crate::preserves_rail::content_ref_from_bytes(bytes), reference, "{}", case.name);
        assert_eq!(super::super::parse_canonical_replica_message(bytes).expect("fixed envelope"), case.envelope);
        assert_eq!(case.envelope, before, "{}", case.name);
        let variant = std::mem::discriminant(&case.envelope.message);
        if !variants.contains(&variant) {
            variants.push(variant);
        }
    }
    assert_eq!(variants.len(), MESSAGE_VARIANT_COUNT);
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn truncation_rejects_without_changing_the_fixture() {
    for case in super::cases() {
        let (bytes, reference) = fixture(case.name);
        let truncated_length = bytes.len().checked_sub(1).expect("nonempty fixed message");
        assert!(super::super::parse_canonical_replica_message(&bytes[..truncated_length]).is_err(), "{}", case.name);
        assert_eq!(crate::preserves_rail::content_ref_from_bytes(bytes), reference, "{}", case.name);
    }
}

fn fixture(name: &str) -> (&'static [u8], &'static str) {
    match name {
        "request-vote" => (
            include_bytes!("fixtures/request-vote.preserves"),
            "blake3:76864e9bf80c880fa66e0982281f35a52c2afe02a34d7f52cc03f4759e4cea42",
        ),
        "vote-response" => (
            include_bytes!("fixtures/vote-response.preserves"),
            "blake3:ba5502acf43edda9cbd7e2f14aa60336dfaeaec0574bde8bc8af802209603924",
        ),
        "append-empty" => (
            include_bytes!("fixtures/append-empty.preserves"),
            "blake3:2e273a2a4eff97022e2bc088aec6f41af441603187093315156a7c5a79c8b4cd",
        ),
        "append-entries" => (
            include_bytes!("fixtures/append-entries.preserves"),
            "blake3:df2d7f764076b28c1355584a256817ca739212af6c5d5fc20ec2e3f431bc4b3c",
        ),
        "append-response" => (
            include_bytes!("fixtures/append-response.preserves"),
            "blake3:6f2dc7281dba6587a2625c6707e51ae17985e8dcc5874da4f9c70d330e87690f",
        ),
        "read-probe" => (
            include_bytes!("fixtures/read-probe.preserves"),
            "blake3:e81092cbeb180420dd6982b234a5b7db779edc3df78c7b4660f63e3efedfbbbc",
        ),
        "read-acknowledgement" => (
            include_bytes!("fixtures/read-acknowledgement.preserves"),
            "blake3:0df14ae491e97ae885a054e62199b9fdcf6c0f2ab6c333ef61d01c653dac1b74",
        ),
        "install-snapshot" => (
            include_bytes!("fixtures/install-snapshot.preserves"),
            "blake3:70494cc1eac0ccddf319c66e82b2c7685233fbc5dab8b46e712c4e4d045ee667",
        ),
        "snapshot-response" => (
            include_bytes!("fixtures/snapshot-response.preserves"),
            "blake3:5e4244a5d2a247fe987476413fd92064462c068974bf6eaaa791a6169c126765",
        ),
        "snapshot-empty" => (
            include_bytes!("fixtures/snapshot-empty.preserves"),
            "blake3:6cd2ef8d31fd4a0feaddd23034b579825622fffa4a76343438b4eac72a956bf5",
        ),
        _ => panic!("unknown fixed Raft message fixture: {name}"),
    }
}
