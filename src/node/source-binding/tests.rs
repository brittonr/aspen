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
