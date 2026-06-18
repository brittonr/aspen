    #[test]
    fn cli_transcript_commands_work() {
        let dir = temp_dir("transcript-cli");
        let markdown = dir.join("example.md");
        let transcript_out = dir.join("transcript.preserves");
        let run_receipt = dir.join("transcript-run-receipt.preserves");
        let rendered = dir.join("rendered.md");
        write_file(
            &markdown,
            "```preserves:hide\n<value \"cli\">\n```\n```expect\n<expect-output <value \"cli\">>\n```\n",
        )
        .expect("write transcript markdown");
        run_transcript_command(TranscriptCommand::Parse {
            markdown: markdown.clone(),
            out: transcript_out.clone(),
            dependency_refs: Vec::new(),
            dependency_closure_hash: None,
            handler_profile_ref: None,
            policy_refs: Vec::new(),
            capability_refs: Vec::new(),
            revocation_refs: Vec::new(),
            seed_ref: None,
            expected_refs: Vec::new(),
        })
        .expect("transcript parse");
        run_transcript_command(TranscriptCommand::Run {
            transcript: transcript_out.clone(),
            cache: Some(dir.join("transcript-cache")),
            state: "fresh".to_string(),
            save_root: None,
            out: Some(rendered.clone()),
            receipt_out: Some(run_receipt.clone()),
            failure_out: Some(dir.join("transcript.failure.preserves")),
        })
        .expect("transcript run");
        assert!(fs::read_to_string(&rendered).expect("read rendered").contains("output hidden"));
        run_transcript_command(TranscriptCommand::Show {
            transcript: transcript_out.clone(),
        })
        .expect("transcript show");
        run_transcript_command(TranscriptCommand::Render {
            transcript: transcript_out,
            receipt: Some(run_receipt),
            out: dir.join("rendered-again.md"),
        })
        .expect("transcript render");
    }

    #[test]
    fn cli_eval_cache_commands_work() {
        let dir = temp_dir("cache-cli");
        let cache = dir.join("eval-cache");
        let input = dir.join("input.preserves");
        let output = dir.join("output.preserves");
        let key_out = dir.join("key.preserves");
        let value_out = dir.join("value.preserves");
        let hit_out = dir.join("hit.preserves");
        let dependency_ref = test_ref("cache-cli-dependency");
        let policy_ref = test_ref("cache-cli-policy");
        write_file(&input, "<schema-shape <record \"x\">>").expect("write cache input");
        write_file(&output, "<fingerprint \"ok\">").expect("write cache output");
        run_cache_command(CacheCommand::Put {
            input,
            cache: cache.clone(),
            output: Some(output),
            operation: "schema-fingerprint".to_string(),
            version: "v1".to_string(),
            dependencies: vec![dependency_ref.clone()],
            dependency_closure_hash: None,
            handler_profile_ref: None,
            policy_refs: vec![policy_ref.clone()],
            capability_refs: Vec::new(),
            revocation_refs: Vec::new(),
            tool_ref: None,
            tool_version: "cli-test".to_string(),
            assumption_refs: Vec::new(),
            tier: eval_cache::TIER_PURE.to_string(),
            status: eval_cache::STATUS_PASS.to_string(),
            evidence_refs: Vec::new(),
            diagnostics: Vec::new(),
            key_out: Some(key_out.clone()),
            value_out: Some(value_out.clone()),
            receipt_out: Some(dir.join("put-receipt.preserves")),
        })
        .expect("cache put");
        let key = eval_cache::parse_eval_cache_key(&read_preserves_file(&key_out).expect("read key"))
            .expect("parse cache key");
        run_cache_command(CacheCommand::Get {
            key_ref: key.key_ref.clone(),
            cache: cache.clone(),
            current_policy_refs: Vec::new(),
            current_capability_refs: Vec::new(),
            current_revocation_refs: Vec::new(),
            semantic_enabled: true,
            out: Some(hit_out.clone()),
            receipt_out: Some(dir.join("hit-receipt.preserves")),
        })
        .expect("cache get");
        assert_eq!(fs::read_to_string(&hit_out).expect("read hit"), "<fingerprint \"ok\">");
        run_cache_command(CacheCommand::Status { cache: cache.clone() }).expect("cache status");
        run_cache_command(CacheCommand::List {
            cache: cache.clone(),
            operation: Some("schema-fingerprint".to_string()),
            tier: Some(eval_cache::TIER_PURE.to_string()),
            status: Some(eval_cache::STATUS_PASS.to_string()),
            dependency_ref: Some(dependency_ref.clone()),
            policy_ref: Some(policy_ref),
            capability_ref: None,
            revocation_ref: None,
            evidence_ref: None,
        })
        .expect("cache list");
        run_cache_command(CacheCommand::Show {
            reference: key.key_ref.clone(),
            cache: cache.clone(),
        })
        .expect("cache show key");
        run_cache_command(CacheCommand::Show {
            reference: eval_cache::parse_eval_cache_value(&read_preserves_file(&value_out).expect("read value"))
                .expect("parse cache value")
                .value_ref,
            cache: cache.clone(),
        })
        .expect("cache show value");
        let cache_retention_object = RetentionCliObject {
            root: &cache,
            label: "cache-invalidate",
            object_ref: &key.key_ref,
            object_kind: "eval-cache-key",
            retention_class: retention::CLASS_EPHEMERAL_CACHE,
            action: retention::ACTION_TOMBSTONE,
        };
        let cache_retention = retention_cli_args_for_object(cache_retention_object);
        let cache_apply_refs = vec![retention_apply_ref(
            cache_retention_object,
            "eval-cache-invalidate",
            &cache_retention,
        )];
        let invalidate_receipt = dir.join("invalidate-receipt.preserves");
        run_cache_command(CacheCommand::Invalidate {
            cache: cache.clone(),
            key_ref: None,
            dependency_ref: Some(dependency_ref),
            policy_ref: None,
            capability_ref: None,
            revocation_ref: None,
            operation: None,
            reason: "cli-test".to_string(),
            apply_refs: cache_apply_refs,
            retention: cache_retention,
            receipt_out: Some(invalidate_receipt.clone()),
        })
        .expect("cache invalidate");
        let invalidate_text = fs::read_to_string(&invalidate_receipt).expect("read invalidate receipt");
        assert!(invalidate_text.contains("retention-execution"));
        let error = run_cache_command(CacheCommand::Get {
            key_ref: key.key_ref,
            cache,
            current_policy_refs: Vec::new(),
            current_capability_refs: Vec::new(),
            current_revocation_refs: Vec::new(),
            semantic_enabled: true,
            out: None,
            receipt_out: None,
        })
        .expect_err("invalidated key should miss");
        assert!(error.to_string().contains("tombstoned"), "{error}");
    }
