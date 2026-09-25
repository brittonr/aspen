
    struct ReplayCase {
        expected_report_ref: String,
        final_state_ref: String,
        expected_ref: String,
        actual_ref: String,
        handler_profile_ref: String,
    }

    fn replay_case(ledger_root: &Path) -> ReplayCase {
        let fixture = ReplayCase {
            expected_report_ref: test_ref("replay-expected-report"),
            final_state_ref: test_ref("replay-final-state"),
            expected_ref: test_ref("replay-expected-effect"),
            actual_ref: test_ref("replay-actual-effect"),
            handler_profile_ref: test_ref("replay-handler-profile"),
        };
        let verify = replay_verify_record(&fixture, &test_ref("replay-actual-report"));
        let divergence = replay_divergence_record(&fixture);
        let rollup = replay_rollup(&verify);
        let index = replay_index(&verify, &rollup);
        crate::ledger::import_artifact(ledger_root, &verify).expect("import replay verify");
        crate::ledger::import_artifact(ledger_root, &divergence).expect("import first divergence");
        crate::ledger::import_artifact(ledger_root, &rollup.value).expect("import replay rollup");
        crate::ledger::import_artifact(ledger_root, &index.value).expect("import replay index");
        fixture
    }

    fn replay_verify_record(fixture: &ReplayCase, actual_report_ref: &str) -> IoValue {
        record("deterministic-replay-verify-v1", vec![
            string(crate::preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA),
            string("deny"),
            record("expected-report-ref", vec![string(&fixture.expected_report_ref)]),
            record("actual-report-ref", vec![string(actual_report_ref)]),
            record("final-state-ref", vec![string(&fixture.final_state_ref)]),
            record("divergence", vec![string("effect-response")]),
            checks_value(&["evidence-only", "no-authority-grant"]),
        ])
    }

    fn replay_divergence_record(fixture: &ReplayCase) -> IoValue {
        record("deterministic-first-divergence-v1", vec![
            string(crate::preserves_rail::DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA),
            record("kind", vec![string("effect-response")]),
            record("turn-id", vec![string("turn:0001")]),
            record("actor-id", vec![string("actor:helper")]),
            record("log-position", vec![string("0")]),
            record("handler-profile-ref", vec![string(&fixture.handler_profile_ref)]),
            record("expected-ref", vec![string(&fixture.expected_ref)]),
            record("actual-ref", vec![string(&fixture.actual_ref)]),
            checks_value(&["evidence-only", "first-divergence"]),
        ])
    }
