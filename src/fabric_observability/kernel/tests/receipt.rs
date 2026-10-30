use super::*;

fn fixture() -> Vec<u8> {
    br#"{"runtime":{"schema_version":1,"machine_id":"fixture-machine","active_generation":"fixture-generation","generations":[{"generation":"fixture-generation","state":"active","pack_identity":"fixture-pack","kernel_build_identity":"fixture-kernel","btf_blake3":"fixture-btf","module_blake3":"fixture-module","object_blake3":"fixture-object","loader_blake3":"fixture-loader","attach_target":"fixture-device","verifier_class":"accepted","observation_loss":0,"cleanup_class":"retained"}],"read_only":true,"mutable_handles_exposed":false,"credentials_exposed":false}}"#.to_vec()
}

fn expected() -> OnixBpfStatusExpectation {
    OnixBpfStatusExpectation {
        machine: "fixture-machine".into(),
        generation: "fixture-generation".into(),
        pack: "fixture-pack".into(),
        kernel: "fixture-kernel".into(),
        btf: "fixture-btf".into(),
        object: "fixture-object".into(),
        loader: "fixture-loader".into(),
        attachment: "fixture-device".into(),
    }
}

// r[verify linux_ebpf_observability.boundary]
// r[verify linux_ebpf_observability.failure]
#[test]
fn actual_file_reader_preserves_decoder_parity_and_closes_on_rollover() {
    let workspace = crate::test_support::process_workspace("ebpf-status-parity").expect("owned workspace");
    let root = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Ledger, &workspace).expect("namespace");
    let path = crate::node_state::RelativePath::parse("receipt.json").expect("receipt leaf");
    let bytes = fixture();
    root.write(&path, &bytes).expect("synthetic provider status receipt");
    let mut reader = OnixBpfReader::open(&root, path.clone(), expected());
    assert_eq!(reader.poll(false), decode_onix_bpf_status(&bytes));
    let mut drift: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    drift["runtime"]["generations"][0]["btf_blake3"] = "changed-btf".into();
    root.write(&path, &serde_json::to_vec(&drift).unwrap()).expect("replace fixture receipt");
    assert_eq!(reader.poll(false), Err(EbpfReadFailure::Binding));
    assert_eq!(reader.poll(false), Err(EbpfReadFailure::Closed));
    // Replacement belongs to OnixOS; reader never writes or removes its source.
    let crate::node_state::FileObservation::Regular(file) = root.observe_file(&path).unwrap() else {
        panic!("reader removed or changed receipt type");
    };
    assert_eq!(file.read_bounded(ONIX_BPF_STATUS_MAX_BYTES).unwrap(), serde_json::to_vec(&drift).unwrap());
}

// r[verify linux_ebpf_observability.failure]
#[test]
fn missing_cancelled_and_closed_readers_do_not_claim_observations() {
    let workspace = crate::test_support::process_workspace("ebpf-status-unavailable").expect("owned workspace");
    let root = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Ledger, &workspace).expect("namespace");
    let path = crate::node_state::RelativePath::parse("missing.json").expect("receipt leaf");
    let mut reader = OnixBpfReader::open(&root, path, expected());
    assert_eq!(reader.poll(false), Err(EbpfReadFailure::Missing));
    assert_eq!(reader.poll(true), Err(EbpfReadFailure::Cancelled));
    assert_eq!(reader.poll(false), Err(EbpfReadFailure::Closed));
}

// r[verify linux_ebpf_observability.confidentiality]
#[test]
fn mutable_credentials_unknown_fields_and_ambiguous_json_are_denied() {
    let bytes = fixture();
    for field in ["credentials_exposed", "mutable_handles_exposed"] {
        let mut changed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        changed["runtime"][field] = true.into();
        assert_eq!(decode_onix_bpf_status(&serde_json::to_vec(&changed).unwrap()), Err(EbpfReadFailure::Binding));
    }
    for field in [
        "command_line",
        "payload",
        "packet",
        "private_path",
        "socket",
        "peer",
        "secret",
        "kernel_pointer",
    ] {
        let mut changed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        changed["runtime"]["generations"][0][field] = "prohibited".into();
        assert_eq!(decode_onix_bpf_status(&serde_json::to_vec(&changed).unwrap()), Err(EbpfReadFailure::Malformed));
    }
    let duplicate = String::from_utf8(bytes)
        .unwrap()
        .replace("\"read_only\":true", "\"read_only\":false,\"read_only\":true");
    assert_eq!(decode_onix_bpf_status(duplicate.as_bytes()), Err(EbpfReadFailure::Malformed));
}
