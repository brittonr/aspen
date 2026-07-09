
#[cfg(test)]
mod tests {
    use super::*;

    type Case = hegel::TestCase;

    const TEST_LOGICAL_TIME_STEP: u64 = 7;

    fn test_ref(label: &str) -> String {
        local_ref("transcript-test", label).expect("test ref")
    }

    fn admitted_parse_input(label: &str) -> TranscriptParseInput {
        TranscriptParseInput {
            schema_refs: vec![test_ref(&format!("schema-{label}"))],
            handler_profile_ref: Some(test_ref(&format!("handler-profile-{label}"))),
            policy_refs: vec![test_ref(&format!("policy-{label}"))],
            capability_refs: vec![test_ref(&format!("capability-{label}"))],
            resource_refs: vec![test_ref(&format!("resource-{label}"))],
            effect_manifest_refs: vec![test_ref(&format!("effect-{label}"))],
            seed_ref: Some(test_ref(&format!("seed-{label}"))),
            logical_time: Some(TEST_LOGICAL_TIME_STEP),
            ..TranscriptParseInput::default()
        }
    }

    #[test]
    fn parse_markdown_preserves_order_modifiers_and_stable_refs() {
        let source = "# Demo\n\n```preserves:hide\n<value 1>\n```\n\n```expect\n<expect-output <value 1>>\n```\n";
        let first = parse_markdown(source, &TranscriptParseInput::default()).expect("parse first");
        let second = parse_markdown(source, &TranscriptParseInput::default()).expect("parse second");
        assert_eq!(first.transcript_ref, second.transcript_ref);
        assert_eq!(first.stanzas.len(), 3);
        assert_eq!(first.stanzas[1].kind, KIND_PRESERVES);
        assert!(first.stanzas[1].has_modifier("hide"));
        assert_eq!(first.stanzas[2].kind, KIND_EXPECT);
    }

    #[test]
    fn fresh_runs_are_deterministic_across_temp_roots_and_render_hides_output() {
        let source = "```preserves:hide\n<value \"stable\">\n```\n```expect\n<expect-output <value \"stable\">>\n```\n";
        let transcript = parse_markdown(source, &TranscriptParseInput::default()).expect("parse");
        let first = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run first");
        let second = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run second");
        assert_eq!(first.decision, DECISION_PASS);
        assert_eq!(second.decision, DECISION_PASS);
        assert_eq!(
            canonical_hash(&first.receipt_value).expect("first hash"),
            canonical_hash(&second.receipt_value).expect("second hash")
        );
        let rendered = render_transcript(&transcript, Some(&first)).expect("render");
        assert!(rendered.contains("output hidden"));
        assert!(!rendered.contains("stable\">\n```preserves-output"));
    }

    #[test]
    fn restricted_cli_installs_artifact_and_matches_receipt_expectations() {
        let source = "```preserves\n<payload \"doc\">\n```\n```molten-cli\ntest artifact install --kind transcript-example\n```\n```expect\n<expect-decision \"pass\">\n```\n```molten-cli\ntest artifact list\n```\n";
        let transcript = parse_markdown(source, &admitted_parse_input("install")).expect("parse");
        let run = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run");
        assert_eq!(run.decision, DECISION_PASS);
        assert!(run.stanza_outcomes.iter().any(|outcome| {
            outcome
                .output
                .as_ref()
                .is_some_and(|output| output.collect_simple_record("artifact-list", Some(1)).is_some())
        }));
    }

    #[test]
    fn expected_error_known_bug_and_ambient_shell_denials_are_canonical() {
        let source =
            "```molten-cli:error\ntest unsupported command\n```\n```molten-cli:bug\ntest artifact closure\n```\n";
        let transcript = parse_markdown(source, &TranscriptParseInput::default()).expect("parse");
        let run = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run");
        assert_eq!(run.decision, DECISION_KNOWN_BUG);
        assert_eq!(run.stanza_outcomes[0].decision, DECISION_PASS);
        assert_eq!(run.stanza_outcomes[1].decision, DECISION_KNOWN_BUG);
        let shell = parse_markdown("```shell\necho ambient\n```", &TranscriptParseInput::default())
            .expect_err("ambient shell denied");
        assert!(shell.to_string().contains("ambient shell"), "{shell}");
    }

    #[test]
    fn eval_cache_hit_reuses_deterministic_transcript_receipt() {
        let source = "```preserves\n<value \"cache\">\n```\n```expect\n<expect-output <value \"cache\">>\n```\n";
        let dependency_ref = local_ref("transcript-dependency", "cache").expect("dependency ref");
        let artifact_ref = local_ref("transcript-artifact", "cache").expect("artifact ref");
        let schema_ref = local_ref("transcript-schema", "cache").expect("schema ref");
        let handler_profile_ref = local_ref("transcript-handler-profile", "deterministic").expect("profile ref");
        let policy_ref = local_ref("transcript-policy", "cache").expect("policy ref");
        let capability_ref = local_ref("transcript-capability", "cache").expect("capability ref");
        let resource_ref = local_ref("transcript-resource", "cache").expect("resource ref");
        let effect_ref = local_ref("transcript-effect", "cache").expect("effect ref");
        let initial_state_ref = local_ref("transcript-initial-state", "cache").expect("initial state ref");
        let seed_ref = local_ref("transcript-seed", "cache").expect("seed ref");
        let expected_ref = local_ref("transcript-expected-output", "cache").expect("expected ref");
        let transcript = parse_markdown(source, &TranscriptParseInput {
            dependency_refs: vec![dependency_ref.clone()],
            dependency_closure_hash: Some(initial_state_ref.clone()),
            artifact_refs: vec![artifact_ref.clone()],
            schema_refs: vec![schema_ref.clone()],
            handler_profile_ref: Some(handler_profile_ref.clone()),
            policy_refs: vec![policy_ref.clone()],
            capability_refs: vec![capability_ref.clone()],
            resource_refs: vec![resource_ref.clone()],
            effect_manifest_refs: vec![effect_ref.clone()],
            seed_ref: Some(seed_ref.clone()),
            logical_time: Some(TEST_LOGICAL_TIME_STEP),
            expected_refs: vec![expected_ref.clone()],
            ..TranscriptParseInput::default()
        })
        .expect("parse");
        let cache_key = crate::eval_cache::parse_key(
            &crate::eval_cache::key_value(&transcript_cache_key(&transcript).expect("transcript cache key"))
                .expect("cache key value"),
        )
        .expect("parse cache key");
        assert_eq!(cache_key.dependency_closure_hash, initial_state_ref);
        assert!(cache_key.dependency_refs.contains(&dependency_ref));
        assert!(cache_key.dependency_refs.contains(&artifact_ref));
        assert!(cache_key.dependency_refs.contains(&schema_ref));
        assert!(cache_key.dependency_refs.contains(&resource_ref));
        assert!(cache_key.dependency_refs.contains(&effect_ref));
        assert_eq!(cache_key.handler_profile_ref.as_deref(), Some(handler_profile_ref.as_str()));
        assert_eq!(cache_key.policy_refs, vec![policy_ref]);
        assert_eq!(cache_key.capability_refs, vec![capability_ref]);
        assert!(cache_key.assumption_refs.contains(&seed_ref));
        assert!(cache_key.assumption_refs.contains(&expected_ref));
        assert!(cache_key.assumption_refs.contains(
            &transcript_logical_time_ref(Some(TEST_LOGICAL_TIME_STEP)).expect("logical time ref").expect("logical time set")
        ));

        let cache_root = temp_state_root("cache-test").expect("cache root");
        let input = TranscriptRunInput {
            cache_root: Some(cache_root),
            ..TranscriptRunInput::default()
        };
        let first = run_transcript(&transcript, &input).expect("first run");
        assert!(first.cache_receipt_value.is_some());
        let second = run_transcript(&transcript, &input).expect("second cached run");
        assert!(second.cache_receipt_value.is_some());
        assert_eq!(second.stanza_outcomes.len(), 0);
        assert_eq!(first.receipt_ref, second.receipt_ref);
    }

    #[test]
    fn canonical_receipt_oracle_matches_artifact_closure() {
        let source = "```preserves\n<payload \"doc\">\n```\n```molten-cli\ntest artifact install --kind transcript-example\n```\n```molten-cli\ntest artifact closure\n```\n```expect\n<expect-receipt \"artifact-receipt-v1\" \"pass\">\n```\n";
        let transcript = parse_markdown(source, &admitted_parse_input("receipt-oracle")).expect("parse");
        let run = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run");
        assert_eq!(run.decision, DECISION_PASS);
        assert!(run.receipt_value.collect_simple_record("transcript-run-receipt-v1", Some(TRANSCRIPT_RUN_RECEIPT_FIELD_COUNT)).is_some());
        let rendered = to_text(&run.receipt_value).expect("receipt text");
        assert!(rendered.contains("profile-seed-effect-resource-bound"));
    }

    #[test]
    fn stale_ref_and_raw_output_expectations_deny_normative_pass() {
        let missing_ref = test_ref("stale-artifact");
        let stale_source = format!(
            "```molten-cli\ntest artifact closure {}\n```\n```expect\n<expect-receipt \"artifact-receipt-v1\" \"pass\">\n```\n",
            missing_ref
        );
        let stale = parse_markdown(&stale_source, &TranscriptParseInput::default()).expect("parse stale");
        let stale_run = run_transcript(&stale, &TranscriptRunInput::default()).expect("run stale");
        assert_eq!(stale_run.decision, DECISION_DENY);
        assert!(stale_run
            .stanza_outcomes
            .iter()
            .any(|outcome| outcome.diagnostics.iter().any(|diagnostic| diagnostic.contains("decision mismatch"))));

        let raw_source = "```preserves:hide\n<value \"hidden\">\n```\n```expect\n<expect-stdout \"hidden\">\n```\n";
        let raw = parse_markdown(raw_source, &TranscriptParseInput::default()).expect("parse raw");
        let raw_run = run_transcript(&raw, &TranscriptRunInput::default()).expect("run raw");
        assert_eq!(raw_run.decision, DECISION_DENY);
        assert!(raw_run
            .stanza_outcomes
            .iter()
            .any(|outcome| outcome.diagnostics.iter().any(|diagnostic| diagnostic.contains("diagnostic-only"))));
    }

    #[test]
    fn missing_capability_and_nondeterministic_output_are_denied_before_pass() {
        let missing_capability_input = TranscriptParseInput {
            schema_refs: vec![test_ref("schema-missing-capability")],
            policy_refs: vec![test_ref("policy-missing-capability")],
            resource_refs: vec![test_ref("resource-missing-capability")],
            effect_manifest_refs: vec![test_ref("effect-missing-capability")],
            ..TranscriptParseInput::default()
        };
        let source = "```preserves\n<payload \"doc\">\n```\n```molten-cli\ntest artifact install --kind transcript-example\n```\n";
        let transcript = parse_markdown(source, &missing_capability_input).expect("parse missing capability");
        let run = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run missing capability");
        assert_eq!(run.decision, DECISION_DENY);
        assert!(run
            .stanza_outcomes
            .iter()
            .any(|outcome| outcome.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing capability ref"))));

        let nondeterministic = parse_markdown(
            "```molten-cli\ntest nondeterministic\n```\n",
            &TranscriptParseInput::default(),
        )
        .expect("parse nondeterministic");
        let nondeterministic_run = run_transcript(&nondeterministic, &TranscriptRunInput::default())
            .expect("run nondeterministic");
        assert_eq!(nondeterministic_run.decision, DECISION_DENY);
        assert!(nondeterministic_run
            .stanza_outcomes
            .iter()
            .any(|outcome| outcome.diagnostics.iter().any(|diagnostic| diagnostic.contains("nondeterministic"))));
    }

    #[test]
    fn stanza_modifier_refs_admit_effects_and_profile_mismatch_misses_cache() {
        let policy_ref = test_ref("modifier-policy");
        let capability_ref = test_ref("modifier-capability");
        let resource_ref = test_ref("modifier-resource");
        let effect_ref = test_ref("modifier-effect");
        let schema_ref = test_ref("modifier-schema");
        let source = format!(
            "```preserves\n<payload \"doc\">\n```\n```molten-cli policy-ref={} capability-ref={} resource-ref={} effect-ref={} schema-ref={}\ntest artifact install --kind transcript-example\n```\n```expect\n<expect-decision \"pass\">\n```\n",
            policy_ref, capability_ref, resource_ref, effect_ref, schema_ref
        );
        let transcript = parse_markdown(&source, &TranscriptParseInput::default()).expect("parse modifier refs");
        assert!(transcript.stanzas[1].declared_refs.contains(&capability_ref));
        let run = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run modifier refs");
        assert_eq!(run.decision, DECISION_PASS);

        let cache_root = temp_state_root("profile-mismatch-cache").expect("cache root");
        let profile_one = parse_markdown("```preserves\n<value \"profile\">\n```\n", &TranscriptParseInput {
            handler_profile_ref: Some(test_ref("profile-one")),
            ..TranscriptParseInput::default()
        })
        .expect("profile one parse");
        let profile_two = parse_markdown("```preserves\n<value \"profile\">\n```\n", &TranscriptParseInput {
            handler_profile_ref: Some(test_ref("profile-two")),
            ..TranscriptParseInput::default()
        })
        .expect("profile two parse");
        let input = TranscriptRunInput {
            cache_root: Some(cache_root),
            ..TranscriptRunInput::default()
        };
        let first = run_transcript(&profile_one, &input).expect("profile one run");
        let second = run_transcript(&profile_two, &input).expect("profile two run");
        assert!(first.cache_receipt_value.is_some());
        assert!(!second.stanza_outcomes.is_empty());
        assert_ne!(first.receipt_ref, second.receipt_ref);
    }

    #[test]
    fn ucm_compatibility_claim_is_denied() {
        let error = parse_markdown("```ucm\n.> load scratch.u\n```\n", &TranscriptParseInput::default())
            .expect_err("ucm syntax denied");
        assert!(error.to_string().contains("prior art only"), "{error}");
    }

    #[test]
    fn ledger_classifies_transcript_artifacts_and_receipts() {
        let transcript =
            parse_markdown("```preserves\n<value 1>\n```", &TranscriptParseInput::default()).expect("parse");
        assert_eq!(crate::ledger::artifact_kind(&transcript.value), "transcript-artifact");
        let run = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run");
        assert_eq!(crate::ledger::artifact_kind(&run.receipt_value), "transcript-run-receipt");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_stanza_order_identity_and_denied_ambient_properties(tc: Case) {
        let n = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1000));
        let source = format!("```preserves\n<value {}>\n```\n```expect\n<expect-output <value {}>>\n```\n", n, n);
        let transcript = parse_markdown(&source, &TranscriptParseInput::default()).expect("parse");
        let reparsed = parse_transcript_artifact(&transcript.value).expect("reparse");
        assert_eq!(transcript.transcript_ref, reparsed.transcript_ref);
        assert_eq!(transcript.stanzas[0].index, 0);
        assert_eq!(transcript.stanzas[1].index, 1);
        let run = run_transcript(&transcript, &TranscriptRunInput::default()).expect("run");
        assert_eq!(run.decision, DECISION_PASS);
        let bad = parse_markdown(&format!("```shell\necho {}\n```", n), &TranscriptParseInput::default());
        assert!(bad.is_err());
        let value = parse_text(&format!("<value {}>", n)).expect("value");
        assert_eq!(
            canonical_hash(run.stanza_outcomes[0].output.as_ref().expect("output")).expect("output ref"),
            canonical_hash(&value).expect("value ref")
        );
    }
}
