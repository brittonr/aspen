
#[cfg(test)]
mod tests {
    use super::*;

    type Case = hegel::TestCase;

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
        let transcript = parse_markdown(source, &TranscriptParseInput::default()).expect("parse");
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
        let handler_profile_ref = local_ref("transcript-handler-profile", "deterministic").expect("profile ref");
        let policy_ref = local_ref("transcript-policy", "cache").expect("policy ref");
        let initial_state_ref = local_ref("transcript-initial-state", "cache").expect("initial state ref");
        let seed_ref = local_ref("transcript-seed", "cache").expect("seed ref");
        let expected_ref = local_ref("transcript-expected-output", "cache").expect("expected ref");
        let transcript = parse_markdown(source, &TranscriptParseInput {
            dependency_refs: vec![dependency_ref.clone()],
            dependency_closure_hash: Some(initial_state_ref.clone()),
            handler_profile_ref: Some(handler_profile_ref.clone()),
            policy_refs: vec![policy_ref.clone()],
            seed_ref: Some(seed_ref.clone()),
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
        assert_eq!(cache_key.dependency_refs, vec![dependency_ref]);
        assert_eq!(cache_key.handler_profile_ref.as_deref(), Some(handler_profile_ref.as_str()));
        assert_eq!(cache_key.policy_refs, vec![policy_ref]);
        assert!(cache_key.assumption_refs.contains(&seed_ref));
        assert!(cache_key.assumption_refs.contains(&expected_ref));

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
