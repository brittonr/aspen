// r[verify molten.audit_f02.validation]
// r[verify molten.audit_f02.candidate_binding]
// r[verify molten.audit_f02.real_evidence]
#[test]
fn normal_startup_evidence_errors_preserve_active_lock_and_clean_shutdown() {
    for case in ["missing", "read-error", "synthetic"] {
        let workspace =
            crate::test_support::process_workspace(&format!("source-startup-{case}")).expect("isolated node root");
        std::fs::create_dir(workspace.join("control")).expect("control directory");
        let protected = [
            "control/node.lock.preserves",
            "shutdown-receipt.preserves",
            "startup-receipt.preserves",
        ];
        for path in protected {
            std::fs::write(workspace.join(path), format!("prior-{path}")).expect("existing lifecycle evidence");
        }
        let trust = workspace.join("source-gate-trust.json");
        match case {
            "read-error" => std::fs::create_dir(&trust).expect("unreadable regular-file input"),
            "synthetic" => {
                std::fs::write(&trust, br#"{"schema":"explicit-test-fixture"}"#).expect("synthetic trust carrier")
            }
            _ => {}
        }
        let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("root capability");
        super::super::run_local_with_root(&root).expect_err("normal startup fails closed");
        for path in protected {
            assert_eq!(
                std::fs::read(workspace.join(path)).expect("preserved evidence"),
                format!("prior-{path}").as_bytes()
            );
        }
        assert!(!workspace.join("receipts").exists(), "source denial cannot prepare adapter effects");
        assert!(!workspace.join("source-binding-validation.preserves").exists());
    }
}

// r[verify molten.startup_binding.authentication]
#[test]
fn verifier_process_executes_the_selected_binary_under_valid_bounds() {
    use std::os::unix::fs::PermissionsExt;
    let workspace = crate::test_support::process_workspace("source-verifier-process").expect("isolated workspace");
    let verifier = workspace.join("verifier");
    std::fs::write(&verifier, "#!/bin/sh\nprintf 'verified %s' \"$#\"\n").expect("stand-in verifier executable");
    std::fs::set_permissions(&verifier, std::fs::Permissions::from_mode(0o755)).expect("executable mode");
    let trust = super::Trust {
        schema: "molten-node-source-trust-v1".to_owned(),
        nix: verifier,
        output: workspace.join("output"),
        public_key: "experimental:key".to_owned(),
        selection: molten_core::startup_binding::Selection {
            binding_ref: String::new(),
            policy_ref: String::new(),
            profile_ref: String::new(),
        },
    };
    // The real process boundary must accept its own limits; only the program's exit decides.
    let stdout = super::process::run(&trust, &["path-info", "--json"]).expect("verifier process executes");
    assert!(stdout.starts_with(b"verified "), "{}", String::from_utf8_lossy(&stdout));
}

// r[verify molten.startup_binding.authentication]
#[test]
fn closure_cache_rejects_stale_and_malformed_entries() {
    let workspace = crate::test_support::process_workspace("source-closure-cache").expect("isolated node root");
    let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("root capability");
    assert!(!super::closure_cache::is_valid(&root, "key-a"));
    super::closure_cache::record(&root, "key-a");
    assert!(super::closure_cache::is_valid(&root, "key-a"));
    assert!(!super::closure_cache::is_valid(&root, "key-b"));
    std::fs::write(workspace.join("cache/source-closure-validation.json"), b"{").expect("malformed cache");
    assert!(!super::closure_cache::is_valid(&root, "key-a"));
}

fn trust_for(output: &std::path::Path, public_key: &str) -> super::Trust {
    super::Trust {
        schema: "molten-node-source-trust-v1".to_owned(),
        nix: output.join("nix"),
        output: output.to_path_buf(),
        public_key: public_key.to_owned(),
        selection: molten_core::startup_binding::Selection {
            binding_ref: String::new(),
            policy_ref: String::new(),
            profile_ref: String::new(),
        },
    }
}

#[test]
fn closure_cache_key_binds_every_measurement_input() {
    let workspace = crate::test_support::process_workspace("source-closure-cache-key").expect("isolated workspace");
    let output_path = workspace.join("output");
    std::fs::create_dir(&output_path).expect("output directory");
    std::fs::write(output_path.join("build-closure"), b"closure").expect("closure input");
    for role in molten_core::startup_binding::closure::ROLES {
        std::fs::write(output_path.join(format!("{role}-closure")), role.as_bytes()).expect("graph input");
    }
    let output = cap_std::fs::Dir::open_ambient_dir(&output_path, cap_std::ambient_authority()).expect("output dir");
    let trust = trust_for(&output_path, "key:a");
    let base = super::closure_cache::key(&output, &trust, "binding-a", "executing-a").expect("key");
    assert_ne!(base, super::closure_cache::key(&output, &trust, "binding-b", "executing-a").expect("key"));
    assert_ne!(base, super::closure_cache::key(&output, &trust, "binding-a", "executing-b").expect("key"));
    assert_ne!(
        base,
        super::closure_cache::key(&output, &trust_for(&output_path, "key:b"), "binding-a", "executing-a").expect("key")
    );
    std::fs::write(output_path.join("build-closure"), b"changed").expect("changed closure input");
    assert_ne!(base, super::closure_cache::key(&output, &trust, "binding-a", "executing-a").expect("key"));
}

/// Test-only accepted source over the synthetic gate receipt; no producer is authenticated.
pub(in super::super) fn fixture() -> crate::error::Result<super::Accepted> {
    let gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let gate_ref = crate::preserves_rail::canonical_hash(&gate_value)?;
    let validation_value = crate::preserves_rail::record("node-source-fixture-validation-v1", vec![
        crate::preserves_rail::string("explicit-test-only-no-producer-authentication"),
        crate::preserves_rail::string(&gate_ref),
    ]);
    let validation_ref = crate::preserves_rail::canonical_hash(&validation_value)?;
    Ok(super::Accepted {
        binding_ref: validation_ref.clone(),
        gate_ref,
        gate_value,
        validation_ref,
        validation_value,
    })
}
