use molten::content_store_adapter::NODE_CONTENT_SCHEMA;
use molten::content_store_adapter::NodeContentConfig;
use molten::node_daemon::ControlServeInput;
use molten::node_daemon::serve_control_content;
use serde_json::json;

// r[verify molten.node_content.lifecycle]
#[test]
fn unknown_json_authority_is_rejected() {
    let wire = json!({"schema":NODE_CONTENT_SCHEMA,"manifest_ref":format!("blake3:{}", "a".repeat(64)),
        "readers":["b".repeat(64)],"bind_addr":"192.0.2.1:17888","tick_ms":250,"allow_any_reader":true});
    assert!(serde_json::from_value::<NodeContentConfig>(wire).is_err());
}

// r[verify molten.node_content.lifecycle]
#[test]
fn denied_policy_precedes_root_and_listener_effects() {
    let root = std::env::temp_dir().join(format!("molten-content-denied-{}", std::process::id()));
    assert!(!root.exists());
    let request = ControlServeInput {
        state_root: &root,
        topic: "node-control",
        max_ticks: 1,
        max_requests_per_tick: 1,
        supervisor_policy_value: None,
    };
    let config = NodeContentConfig {
        schema: NODE_CONTENT_SCHEMA.into(),
        manifest_ref: format!("blake3:{}", "a".repeat(64)),
        readers: vec![],
        bind_addr: "192.0.2.1:17888".parse().unwrap(),
        tick_ms: 250,
    };
    let error = serve_control_content(&request, config, format!("blake3:{}", "c".repeat(64)), None).unwrap_err();
    assert!(error.to_string().contains("read grant denied"));
    assert!(!root.exists());
}

#[test]
fn valid_content_policy_cannot_bypass_missing_real_startup_evidence() {
    let root = std::env::temp_dir().join(format!("molten-content-no-gate-{}", std::process::id()));
    assert!(!root.exists());
    let request = ControlServeInput {
        state_root: &root,
        topic: "node-control",
        max_ticks: 1,
        max_requests_per_tick: 1,
        supervisor_policy_value: None,
    };
    let config = NodeContentConfig {
        schema: NODE_CONTENT_SCHEMA.into(),
        manifest_ref: format!("blake3:{}", "a".repeat(64)),
        readers: vec!["b".repeat(64)],
        bind_addr: "192.0.2.1:17888".parse().unwrap(),
        tick_ms: 250,
    };
    let error = serve_control_content(&request, config, format!("blake3:{}", "c".repeat(64)), None).unwrap_err();
    assert!(error.to_string().contains("node-startup-source-gate-required"));
    assert!(!root.exists());
}

#[test]
fn production_cli_never_manufactures_a_clean_startup_gate() {
    let temporary = std::env::temp_dir();
    // Include the real workspace: Cargo metadata must not enable the old synthetic bypass.
    for (index, cwd) in [std::path::Path::new(env!("CARGO_MANIFEST_DIR")), temporary.as_path()].iter().enumerate() {
        let root = temporary.join(format!("molten-startup-no-gate-{}-{index}", std::process::id()));
        assert!(!root.exists());
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_molten"))
            .current_dir(cwd)
            .args(["node", "run", "--state-root"])
            .arg(&root)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("node-startup-source-gate-required"));
        assert!(!root.exists(), "startup modified state before real source evidence");
    }
}
