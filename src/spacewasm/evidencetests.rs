#[test]
fn local_integrity_cannot_replace_published_identity() {
    let tool = super::Tool {
        path: "/nix/store/local/tool".into(),
        blake3: "1".repeat(64),
    };
    let trust = super::Trust {
        mantle: tool.clone(),
        octet: tool.clone(),
        chaos: tool,
        bundle_identity: String::new(),
        octet_profile_identity: "2".repeat(64),
        cohort_identity: "3".repeat(64),
        probe_blake3: "4".repeat(64),
    };
    assert_eq!(super::validate_pins(&trust), Err("missing authority-pinned external identity".into()));
}

#[test]
fn valid_hostless_member_cannot_stand_for_named_abi_fixture() {
    let manifest = serde_json::json!({"members":[{
        "path":"fixtures/wasm/hostless41.wasm", "digest_blake3":"same-bytes-digest"
    }]});
    assert!(super::require_bundle_member(&manifest, "same-bytes-digest").is_err());
}

#[test]
fn distinguishes_completed_output_guest_failure_and_unknown_traps() {
    assert!(super::validate_row(&serde_json::json!({"outcome":"unsupported"})).is_err());
    let mut row = serde_json::json!({"outcome":"completed", "trap_class":null, "output_hex":"",
        "hostcalls":[], "globals":[], "memory_blake3":"observed-memory"});
    assert!(super::validate_row(&row).is_err());
    row["outcome"] = serde_json::json!("invalid-output");
    assert!(super::validate_row(&row).is_ok());
    row["outcome"] = serde_json::json!("trap");
    row["trap_class"] = serde_json::json!("generic");
    assert!(super::validate_row(&row).is_err());
}

#[test]
fn stale_static_artifact_or_error_finding_cannot_be_admitted() {
    let tool = super::Tool {
        path: "/nix/store/local/tool".into(),
        blake3: "1".repeat(64),
    };
    let trust = super::Trust {
        mantle: tool.clone(),
        octet: tool.clone(),
        chaos: tool,
        bundle_identity: "2".repeat(64),
        octet_profile_identity: "3".repeat(64),
        cohort_identity: "4".repeat(64),
        probe_blake3: "5".repeat(64),
    };
    let mut report = serde_json::json!({"schema":"octet-evidence-rail-receipt/v1", "status":"passed",
        "rail":"wasm-artifact", "wasm_artifact":{"profile_id":"spacewasm-molten-abi-pilot",
        "profile_identity":trust.octet_profile_identity, "exact_artifact_identity":"b3:stale", "findings":[],
        "target_runtime":{"source_revision":super::super::profile::REVISION,
        "cohort_identity":"b3:7c537eef7bff66d0a90508715de08de79c9aaac23352e1007a6d47e7fb4674a2"}}});
    assert!(super::verify_octet(&report, &trust, "actual").is_err());
    report["wasm_artifact"]["exact_artifact_identity"] = serde_json::json!("b3:actual");
    report["wasm_artifact"]["findings"] = serde_json::json!([{"severity":"error"}]);
    assert!(super::verify_octet(&report, &trust, "actual").is_err());
}
