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
    let run = {
        let state = crate::test_support::process_workspace("transcript-state").expect("state workspace");
        run_transcript(&transcript, &TranscriptRunInput::default(), |_| Ok(state.to_path_buf()))
    }
    .expect("run modifier refs");
    assert_eq!(run.decision, DECISION_PASS);

    let cache_root = crate::test_support::process_workspace("profile-mismatch-cache").expect("cache root");
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
        cache_root: Some(cache_root.to_path_buf()),
        ..TranscriptRunInput::default()
    };
    let first = {
        let state = crate::test_support::process_workspace("transcript-state").expect("state workspace");
        run_transcript(&profile_one, &input, |_| Ok(state.to_path_buf()))
    }
    .expect("profile one run");
    let second = {
        let state = crate::test_support::process_workspace("transcript-state").expect("state workspace");
        run_transcript(&profile_two, &input, |_| Ok(state.to_path_buf()))
    }
    .expect("profile two run");
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
    let transcript = parse_markdown("```preserves\n<value 1>\n```", &TranscriptParseInput::default()).expect("parse");
    assert_eq!(crate::ledger::artifact_kind(&transcript.value), "transcript-artifact");
    let run = {
        let state = crate::test_support::process_workspace("transcript-state").expect("state workspace");
        run_transcript(&transcript, &TranscriptRunInput::default(), |_| Ok(state.to_path_buf()))
    }
    .expect("run");
    assert_eq!(crate::ledger::artifact_kind(&run.receipt_value), "transcript-run-receipt");
}

#[test]
fn explicit_save_and_denied_modes_do_not_require_temporary_authority() {
    let transcript = parse_markdown(
        "```preserves\n<value 7>\n```\n```expect\n<expect-output <value 7>>\n```\n",
        &TranscriptParseInput::default(),
    )
    .expect("parse");
    let saved = crate::test_support::process_workspace("transcript-saved-state").expect("saved state");
    let save_root = saved.join("saved");
    let run = run_transcript(
        &transcript,
        &TranscriptRunInput {
            mode: TranscriptRunMode::Save,
            save_root: Some(save_root.clone()),
            cache_root: None,
        },
        |_| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into()),
    )
    .expect("explicit save");
    assert_eq!(run.decision, DECISION_PASS);
    assert_eq!(run.state_root.as_ref(), Some(&save_root));
    let expected = parse_text("<value 7>").expect("expected output");
    assert_eq!(run.stanza_outcomes[0].output.as_ref(), Some(&expected));
    for mode in [TranscriptRunMode::ForkDenied, TranscriptRunMode::InPlaceDenied] {
        let denied = run_transcript(
            &transcript,
            &TranscriptRunInput {
                mode,
                ..TranscriptRunInput::default()
            },
            |_| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into()),
        )
        .expect("denied receipt");
        assert_eq!(denied.decision, DECISION_DENY);
        assert_eq!(denied.state_root, None);
    }
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
    let run = {
        let state = crate::test_support::process_workspace("transcript-state").expect("state workspace");
        run_transcript(&transcript, &TranscriptRunInput::default(), |_| Ok(state.to_path_buf()))
    }
    .expect("run");
    assert_eq!(run.decision, DECISION_PASS);
    let bad = parse_markdown(&format!("```shell\necho {}\n```", n), &TranscriptParseInput::default());
    assert!(bad.is_err());
    let value = parse_text(&format!("<value {}>", n)).expect("value");
    assert_eq!(
        canonical_hash(run.stanza_outcomes[0].output.as_ref().expect("output")).expect("output ref"),
        canonical_hash(&value).expect("value ref")
    );
}
