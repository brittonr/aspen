
    #[test]
    fn chaos_schedule_is_deterministic_replay_evidence_only() {
        let input = ChaosScheduleInput {
            seed_ref: DEFAULT_SEED_REF.to_string(),
            schedule_position: 7,
            event_ref: DEFAULT_ARTIFACT_REF.to_string(),
            fault_kind: "drop".to_string(),
            intensity_percent: 50,
        };
        let first = chaos_schedule_receipt(&input).expect("chaos schedule");
        let second = chaos_schedule_receipt(&input).expect("chaos schedule repeat");
        assert_eq!(first.schedule_ref, second.schedule_ref);
        assert_eq!(first.decision, second.decision);
        let text = to_text(&first.value).expect("render chaos schedule");
        assert!(text.contains("deterministic-chaos-schedule-v1"));
        assert!(text.contains("replay-identity-bound"));
        assert!(text.contains("evidence-only-no-authority"));

        let changed = chaos_schedule_receipt(&ChaosScheduleInput {
            schedule_position: 8,
            ..input
        })
        .expect("changed chaos schedule");
        assert_ne!(first.schedule_ref, changed.schedule_ref);
        assert!(
            chaos_schedule_receipt(&ChaosScheduleInput {
                seed_ref: DEFAULT_SEED_REF.to_string(),
                schedule_position: 7,
                event_ref: DEFAULT_ARTIFACT_REF.to_string(),
                fault_kind: "drop".to_string(),
                intensity_percent: 101,
            })
            .is_err()
        );
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_replay_identity_scheduler_trace_and_snapshot_properties(tc: TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(10_000));
        let first = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("first baseline replay");
        let second = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("second baseline replay");
        assert_eq!(first.receipt_ref, second.receipt_ref);
        assert_eq!(first.decision, "pass");
        assert_eq!(first.divergence, ReplayDivergenceKind::None);
        let first_text = to_text(&first.value).expect("render first replay");
        assert!(first_text.contains("ordered-boundary-comparison"));
        assert!(first_text.contains("recorded-effects-only"));

        let trace_a = record_fixture_value().expect("first fixture record");
        let trace_b = record_fixture_value().expect("second fixture record");
        assert_eq!(trace_a.record_ref, trace_b.record_ref);
        assert_eq!(trace_a.effect_log_ref, trace_b.effect_log_ref);
        assert_eq!(trace_a.final_state_ref, trace_b.final_state_ref);
        let trace_text = to_text(&trace_a.value).expect("render fixture record");
        assert!(trace_text.contains("no-ambient-observations"));

        let variant = if salt.is_multiple_of(2) {
            ReplayFixtureVariant::ChangedScheduler
        } else {
            ReplayFixtureVariant::Baseline
        };
        let scheduler_check = verify_fixture_value(variant).expect("scheduler replay check");
        if variant == ReplayFixtureVariant::ChangedScheduler {
            assert_eq!(scheduler_check.decision, "deny");
            assert_eq!(scheduler_check.divergence, ReplayDivergenceKind::Scheduler);
        } else {
            assert_eq!(scheduler_check.decision, "pass");
        }

        let snapshot_ref = canonical_hash(&string(format!("snapshot-{salt}"))).expect("snapshot ref");
        let admitted_ref = canonical_hash(&string(format!("admitted-{salt}"))).expect("admitted ref");
        let redacted_ref = canonical_hash(&string(format!("redacted-{salt}"))).expect("redacted ref");
        let mut requested_refs = vec![admitted_ref.clone(), redacted_ref.clone()];
        requested_refs.sort();
        let snapshot_state = RuntimeSnapshotAuthorityState {
            snapshot_ref,
            admitted_authority_refs: vec![admitted_ref.clone()],
            claimed_authority_refs: vec![admitted_ref.clone()],
            requested_assertion_refs: requested_refs,
            readable_assertion_refs: vec![admitted_ref],
            redacted_assertion_refs: vec![redacted_ref],
        };
        let snapshot =
            crate::runtime::evaluate_snapshot_authority(&snapshot_state).expect("snapshot authority predicate");
        assert!(snapshot.is_allowed);
        assert_eq!(snapshot.receipt.decision, PredicateDecision::Pass);
    }
