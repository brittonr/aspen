    #[test]
    fn cli_rewrite_commands_work() {
        let dir = temp_dir("rewrite-cli");
        let registry = dir.join("registry");
        install_rewrite_doc(&dir, &registry);
        let plan_out = preview_rewrite(&dir, &registry);
        apply_rewrite(&dir, &registry, plan_out);
        assert_rewritten_doc(registry);
    }

    // r[verify molten.local_rewrite.upgrade_hook_authority]
    // r[verify molten.upgrades.authenticated_source_gate]
    #[test]
    fn cli_rewrite_upgrade_plan_fails_closed_without_authenticated_source_or_refs() {
        let dir = temp_dir("rewrite-cli-upgrade-denied");
        let registry = dir.join("registry");
        install_rewrite_doc(&dir, &registry);
        let upgrade_plan = dir.join("rewrite-upgrade-plan.preserves");
        let empty_root = dir.join("empty-source-root");
        fs::create_dir_all(&empty_root).expect("create empty source root");
        let forged_root = forged_source_root(&dir);
        let initiator = Some(cli_synthetic_ref("rewrite-initiator").expect("initiator"));
        let capabilities = vec![cli_synthetic_ref("rewrite-upgrade-capability").expect("capability")];
        let policies = vec![cli_synthetic_ref("rewrite-upgrade-policy").expect("policy")];
        let cases = [
            ("no source evidence", None, initiator.clone(), capabilities.clone(), policies.clone()),
            ("no trust selection", Some(empty_root), initiator.clone(), capabilities.clone(), policies.clone()),
            ("forged gate and trust", Some(forged_root.clone()), initiator.clone(), capabilities.clone(), policies.clone()),
            ("no initiator", Some(forged_root.clone()), None, capabilities.clone(), policies.clone()),
            ("label initiator", Some(forged_root.clone()), Some("rewrite-session".to_string()), capabilities.clone(), policies.clone()),
            ("no capability", Some(forged_root.clone()), initiator.clone(), Vec::new(), policies.clone()),
            ("no policy", Some(forged_root), initiator, capabilities, Vec::new()),
        ];
        for (case, source_state_root, initiator_ref, capability_refs, policy_refs) in cases {
            let mut command = apply_command(&registry);
            if let RewriteCommand::Apply {
                upgrade_plan_out: plan,
                source_state_root: root,
                initiator_ref: initiator,
                capability_refs: capabilities,
                policy_refs: policies,
                ..
            } = &mut command
            {
                *plan = Some(upgrade_plan.clone());
                *root = source_state_root;
                *initiator = initiator_ref;
                *capabilities = capability_refs;
                *policies = policy_refs;
            }
            run_rewrite_command(command).expect_err(case);
            assert!(!upgrade_plan.exists(), "{case}: no unadmitted upgrade plan is written");
            let docs = molten::artifacts::list_artifacts(&registry, Some("doc")).expect("list docs");
            assert_eq!(docs.len(), 1, "{case}: the rewrite is not applied when the upgrade plan is refused");
        }
    }

    /// A caller-written passing gate receipt plus a trust file naming an unusable verifier: not evidence.
    fn forged_source_root(dir: &Path) -> PathBuf {
        let root = dir.join("forged-source-root");
        fs::create_dir_all(&root).expect("create forged source root");
        let gate = molten::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("synthetic gate");
        fs::write(root.join("gate-receipt.preserves"), to_text(&gate).expect("gate text")).expect("write gate");
        let zero = format!("blake3:{}", "0".repeat(64));
        let trust = format!(
            r#"{{"schema":"molten-node-source-trust-v1","nix":"/nonexistent/molten-test/nix","output":"/nix/store/00000000000000000000000000000000-forged","public_key":"forged:AAAA","selection":{{"binding_ref":"{zero}","policy_ref":"{zero}","profile_ref":"{zero}"}}}}"#
        );
        fs::write(root.join("source-gate-trust.json"), trust).expect("write trust");
        root
    }

    fn apply_command(registry: &Path) -> RewriteCommand {
        RewriteCommand::Apply {
            registry: registry.to_path_buf(),
            from: "old".to_string(),
            to: "new".to_string(),
            artifact_kinds: vec!["doc".to_string()],
            root_refs: Vec::new(),
            dependency_inclusion_enabled: true,
            hidden_refs: Vec::new(),
            plan_out: None,
            receipt_out: None,
            upgrade_plan_out: None,
            session_id: "rewrite-cli-session".to_string(),
            source_state_root: None,
            initiator_ref: None,
            capability_refs: Vec::new(),
            policy_refs: Vec::new(),
        }
    }

    fn install_rewrite_doc(dir: &Path, registry: &Path) {
        let payload = dir.join("doc.preserves");
        let artifact_out = dir.join("doc-artifact.preserves");
        write_file(&payload, r#"<doc "old" ["old" "keep"]>"#).expect("write rewrite payload");
        crate::cli_artifact::run(crate::cli_artifact::Command::Install {
            payload,
            registry: registry.to_path_buf(),
            kind: "doc".to_string(),
            dependencies: Vec::new(),
            schema_refs: Vec::new(),
            effect_manifest_ref: None,
            artifact_out: Some(artifact_out),
            receipt_out: Some(dir.join("doc-install-receipt.preserves")),
        })
        .expect("install rewrite artifact");
    }

    fn preview_rewrite(dir: &Path, registry: &Path) -> PathBuf {
        let matches_out = dir.join("rewrite-matches.preserves");
        let plan_out = dir.join("rewrite-plan.preserves");
        run_rewrite_command(RewriteCommand::Find {
            registry: registry.to_path_buf(),
            pattern_kind: "string-equals".to_string(),
            pattern: "old".to_string(),
            artifact_kinds: vec!["doc".to_string()],
            root_refs: Vec::new(),
            dependency_inclusion_enabled: true,
            hidden_refs: Vec::new(),
            matches_out: Some(matches_out.clone()),
            receipt_out: Some(dir.join("rewrite-find-receipt.preserves")),
        })
        .expect("rewrite find");
        assert!(fs::read_to_string(&matches_out).expect("read matches").contains("rewrite-match-v1"));
        run_rewrite_command(RewriteCommand::Preview {
            registry: registry.to_path_buf(),
            from: "old".to_string(),
            to: "new".to_string(),
            artifact_kinds: vec!["doc".to_string()],
            root_refs: Vec::new(),
            dependency_inclusion_enabled: true,
            hidden_refs: Vec::new(),
            plan_out: Some(plan_out.clone()),
            receipt_out: Some(dir.join("rewrite-preview-receipt.preserves")),
        })
        .expect("rewrite preview");
        run_rewrite_command(RewriteCommand::Show {
            artifact: plan_out.clone(),
        })
        .expect("rewrite show plan");
        plan_out
    }

    fn apply_rewrite(dir: &Path, registry: &Path, _plan_out: PathBuf) {
        let apply_receipt = dir.join("rewrite-apply-receipt.preserves");
        let mut command = apply_command(registry);
        if let RewriteCommand::Apply { receipt_out, .. } = &mut command {
            *receipt_out = Some(apply_receipt.clone());
        }
        run_rewrite_command(command).expect("rewrite apply");
        run_rewrite_command(RewriteCommand::Show {
            artifact: apply_receipt,
        })
        .expect("rewrite show receipt");
    }

    fn assert_rewritten_doc(registry: PathBuf) {
        let docs = molten::artifacts::list_artifacts(&registry, Some("doc")).expect("list rewritten docs");
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().any(|artifact| {
            molten::artifacts::read_payload(&registry, &artifact.artifact_ref)
                .and_then(|value| to_text(&value))
                .is_ok_and(|text| text.contains("new"))
        }));
    }
