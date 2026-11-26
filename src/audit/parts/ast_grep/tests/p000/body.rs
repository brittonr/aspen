    use super::*;

    const AST_GREP_VERSION_FIXTURE: &str = "ast-grep 0.39.5";
    const RUN_REF_FIXTURE: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
    /// The scanned inventory measured for fixture receipts; every fixture finding path is part of it.
    const SCANNED_SOURCES: &[ContentSource<'static>] = &[
        ContentSource { path: "src/cli/evidence/gate/io.rs", bytes: b"fn gate_io() {}\n" },
        ContentSource { path: "src/chunk/store/local/ingest.rs", bytes: b"fn store() {}\n" },
        ContentSource { path: "src/cli/runtime/repro/bundle.rs", bytes: b"fn bundle() {}\n" },
        ContentSource { path: "src/node/daemon/live/inbound.rs", bytes: b"fn daemon() {}\n" },
        ContentSource { path: "src/plugin/host.rs", bytes: b"fn host() {}\n" },
    ];
    const STALE_RULE_BUNDLE_REF: &str = "blake3:3333333333333333333333333333333333333333333333333333333333333333";

    macro_rules! rule_file {
        ($id:literal) => {
            (
                concat!("tools/ast-grep/runtime-authority/rules/", $id, ".yml"),
                include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tools/ast-grep/runtime-authority/rules/", $id, ".yml"))
                    .as_slice(),
            )
        };
    }

    /// The checked-in YAML bytes of every rule the default Rust profile declares.
    const DEFAULT_PROFILE_RULE_FILES: &[(&str, &[u8])] = &[
        rule_file!("ambient-filesystem-call"),
        rule_file!("ambient-process-command"),
        rule_file!("ambient-network-bind"),
        rule_file!("ambient-clock-now"),
        rule_file!("ambient-random-thread-rng"),
        rule_file!("credential-env-var"),
        rule_file!("plugin-dynamic-load"),
        rule_file!("unsafe-block"),
        rule_file!("panic-bypass"),
        rule_file!("direct-authority-bypass"),
        rule_file!("store-ambient-filesystem-call"),
        rule_file!("test-ambient-temp-workspace"),
        rule_file!("materialization-ambient-output"),
        rule_file!("node-state-ambient-filesystem-call"),
        rule_file!("node-state-root-reacquisition"),
    ];

    fn rule_sources(files: &[(&'static str, &'static [u8])]) -> Vec<ContentSource<'static>> {
        files.iter().map(|(path, bytes)| ContentSource { path, bytes }).collect()
    }

    fn default_rule_bundle_hash(profile: &AuditProfile) -> String {
        rule_bundle_hash(profile, &rule_sources(DEFAULT_PROFILE_RULE_FILES)).expect("default rule bundle hash")
    }

    fn scan_input(profile: AuditProfile, rule_bundle_hash: String, findings: Vec<Finding>) -> ScanInput {
        ScanInput {
            profile,
            ast_grep_version: AST_GREP_VERSION_FIXTURE.to_string(),
            rule_bundle_hash,
            scan_scope_hash: scan_scope_hash(SCANNED_SOURCES).expect("fixture scan scope"),
            evidence_gate_run_ref: RUN_REF_FIXTURE.to_string(),
            findings,
        }
    }

    /// Builds a fixture receipt against the checked-in rule YAML bytes and the fixture scanned inventory.
    /// Deliberately shadows the glob-imported builder so every receipt test runs the measured path.
    fn build_audit_receipt(input: ScanInput) -> AuditReceipt {
        let rule_sources = rule_sources(DEFAULT_PROFILE_RULE_FILES);
        super::build_audit_receipt(input, &ScanMeasurement {
            rule_sources: &rule_sources,
            scanned_sources: SCANNED_SOURCES,
        })
    }

    fn finding(rule_id: &str, surface: &str, path: &str) -> Finding {
        Finding {
            rule_id: rule_id.to_string(),
            surface: surface.to_string(),
            path: path.to_string(),
            message: format!("{rule_id} structural candidate"),
        }
    }

    fn failed_check<'a>(receipt: &'a AuditReceipt, name: &str) -> &'a ReceiptCheck {
        let check = receipt.checks.iter().find(|check| check.name == name).expect("named check");
        assert!(!check.passed, "{name} unexpectedly passed");
        check
    }

    #[test]
    fn profile_declares_required_structural_surfaces_and_inventory_rules() {
        // r[verify aspen.ast_grep_runtime_authority_audits.profile]
        // r[verify aspen.ast_grep_runtime_authority_audits.inventory]
        let profile = profile();
        let validation = validate_profile(&profile);

        assert!(validation.valid, "{:?}", validation.diagnostics);
        assert!(
            profile
                .rules
                .iter()
                .any(|rule| { rule.id == "store-ambient-filesystem-call" && rule.posture == RulePosture::Blocking })
        );
        assert!(profile.rules.iter().any(|rule| rule.posture == RulePosture::Inventory));
        assert!(REQUIRED_NON_CLAIMS.iter().all(|required| profile.non_claims.iter().any(|claim| claim == required)));
    }

    #[test]
    fn default_profile_declares_node_state_surface_and_active_blocking_rules() {
        // r[verify molten.evidence_gates.static_audit_content_bound_hashes]
        // r[verify molten.node.cap_std_regression_gate]
        let profile = profile();
        let node_state = profile
            .surfaces
            .iter()
            .find(|surface| surface.id == "node-state-adapters")
            .expect("node-state-adapters surface");
        assert!(node_state.scan_scopes.iter().any(|scope| scope == "src/node/daemon/**/*.rs"));
        for scope in profile.surfaces.iter().flat_map(|surface| &surface.scan_scopes) {
            assert!(!scope.contains(' '), "scan scope {scope:?} must be a path or glob, not prose");
        }
        for rule_id in ["node-state-ambient-filesystem-call", "node-state-root-reacquisition"] {
            let rule = profile.rules.iter().find(|rule| rule.id == rule_id).expect("node-state rule");
            assert_eq!(rule.posture, RulePosture::Blocking);
            assert_eq!(rule.rule_path, format!("tools/ast-grep/runtime-authority/rules/{rule_id}.yml"));
        }

        let mut without_surface = profile.clone();
        without_surface.surfaces.retain(|surface| surface.id != "node-state-adapters");
        let validation = validate_profile(&without_surface);
        assert!(!validation.valid);
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic.contains("node-state-adapters")));
    }

    #[test]
    fn node_state_root_reacquisition_finding_is_blocking_and_fails_the_receipt() {
        // r[verify molten.evidence_gates.static_audit_content_bound_hashes]
        // r[verify molten.node.cap_std_regression_gate]
        let profile = profile();
        let rule_bundle_hash = default_rule_bundle_hash(&profile);
        let receipt = build_audit_receipt(scan_input(profile, rule_bundle_hash, vec![finding(
            "node-state-root-reacquisition",
            "node-state-adapters",
            "src/node/daemon/live/inbound.rs",
        )]));

        assert!(!receipt.valid());
        assert_eq!(receipt.decision, RECEIPT_DECISION_INVALID);
        let blocking = failed_check(&receipt, "blocking-findings-absent");
        assert!(blocking.diagnostic.contains("node-state-root-reacquisition"), "{}", blocking.diagnostic);
        assert!(receipt.checks.iter().any(|check| check.name == "findings-reference-known-rules" && check.passed));
    }

    #[test]
    fn default_profile_matches_checked_in_rule_yaml() {
        // r[verify molten.evidence_gates.static_audit_content_bound_hashes]
        let profile = profile();
        let first = default_rule_bundle_hash(&profile);
        assert!(is_content_ref(&first));
        let mut reversed = rule_sources(DEFAULT_PROFILE_RULE_FILES);
        reversed.reverse();
        assert_eq!(rule_bundle_hash(&profile, &reversed).expect("reordered rule sources"), first);
    }

    #[test]
    fn editing_a_rule_yaml_pattern_changes_rule_bundle_hash_and_requires_a_fresh_scan() {
        // r[verify aspen.ast_grep_runtime_authority_audits.identity]
        // r[verify molten.evidence_gates.static_audit_content_bound_hashes]
        let profile = profile();
        let original = default_rule_bundle_hash(&profile);
        let receipt =
            build_audit_receipt(scan_input(profile.clone(), original.clone(), Vec::new()));

        let rule_path = "tools/ast-grep/runtime-authority/rules/node-state-root-reacquisition.yml";
        let (_, original_bytes) =
            DEFAULT_PROFILE_RULE_FILES.iter().find(|(path, _)| *path == rule_path).expect("node-state rule bytes");
        let mut edited_bytes = original_bytes.to_vec();
        edited_bytes.extend_from_slice(b"    - pattern: Dir::open_ambient_dir_with($$$ARGS)\n");
        let edited_sources = DEFAULT_PROFILE_RULE_FILES
            .iter()
            .map(|(path, bytes)| ContentSource {
                path,
                bytes: if *path == rule_path { edited_bytes.as_slice() } else { bytes },
            })
            .collect::<Vec<_>>();
        let edited = rule_bundle_hash(&profile, &edited_sources).expect("edited rule bundle hash");

        assert_ne!(edited, original);
        assert!(!requires_fresh_scan(&receipt, &original));
        assert!(requires_fresh_scan(&receipt, &edited));
    }

    #[test]
    fn rule_bundle_hash_rejects_missing_undeclared_duplicate_or_mismatched_rule_sources() {
        // r[verify molten.evidence_gates.static_audit_content_bound_hashes]
        let profile = profile();
        let mut missing = rule_sources(DEFAULT_PROFILE_RULE_FILES);
        missing.retain(|source| !source.path.ends_with("node-state-root-reacquisition.yml"));
        let error = rule_bundle_hash(&profile, &missing).unwrap_err();
        assert!(error.to_string().contains("node-state-root-reacquisition"), "{error}");
        assert_eq!(error.class(), Some(crate::error::FailureClass::MalformedObservation));

        let mut undeclared = rule_sources(DEFAULT_PROFILE_RULE_FILES);
        undeclared.push(ContentSource {
            path: "tools/ast-grep/runtime-authority/rules/molten-bpf-loader-library.yml",
            bytes: b"id: molten-bpf-loader-library\nmetadata:\n  posture: blocking\n",
        });
        assert!(rule_bundle_hash(&profile, &undeclared).unwrap_err().to_string().contains("not declared"));

        let mut duplicate = rule_sources(DEFAULT_PROFILE_RULE_FILES);
        duplicate.push(duplicate[0]);
        assert!(rule_bundle_hash(&profile, &duplicate).unwrap_err().to_string().contains("twice"));

        let demoted = DEFAULT_PROFILE_RULE_FILES
            .iter()
            .map(|(path, bytes)| ContentSource {
                path,
                bytes: if path.ends_with("node-state-root-reacquisition.yml") {
                    b"id: node-state-root-reacquisition\nmetadata:\n  posture: inventory\n".as_slice()
                } else {
                    bytes
                },
            })
            .collect::<Vec<_>>();
        assert!(rule_bundle_hash(&profile, &demoted).unwrap_err().to_string().contains("posture blocking"));
        assert!(rule_bundle_hash(&profile, &[]).is_err());
    }

