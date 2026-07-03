    type AtomicU64 = std::sync::atomic::AtomicU64;
    type Ordering = std::sync::atomic::Ordering;
    type PathBuf = std::path::PathBuf;
    type PredicateDecision = crate::runtime::PredicateDecision;
    type RuntimeSnapshotAuthorityState = crate::runtime::RuntimeSnapshotAuthorityState;
    type TestCase = hegel::TestCase;

    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove stale temp dir {path:?}: {error}"),
        }
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn replay_fixture_record_binds_identity_effects_and_final_state() {
        let fixture = record_fixture_value().expect("fixture record");
        assert!(fixture.record_ref.starts_with("blake3:"));
        assert!(fixture.identity_ref.starts_with("blake3:"));
        assert!(fixture.effect_log_ref.starts_with("blake3:"));
        assert!(fixture.output_ref.starts_with("blake3:"));
        assert!(fixture.final_state_ref.starts_with("blake3:"));
        let text = to_text(&fixture.value).expect("render fixture");
        assert!(text.contains("deterministic-fixture-record-v1"));
        assert!(text.contains("deterministic-run-identity-v1"));
        assert!(text.contains("artifact-ref"));
        assert!(text.contains("dependency-closure-ref"));
        assert!(text.contains("initial-state-ref"));
        assert!(text.contains("handler-profile-ref"));
        assert!(text.contains("seed-ref"));
        assert!(text.contains("deterministic-effect-log-v1"));
        assert!(text.contains("effect-entry-v1"));
        assert!(text.contains("request-ref"));
        assert!(text.contains("response-ref"));
        assert!(text.contains("no-ambient-observations"));
    }

    #[test]
    fn unchanged_replay_passes_and_binds_output_refs() {
        let receipt = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("verify baseline");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.divergence, ReplayDivergenceKind::None);
        assert!(receipt.first_divergence.is_none());
        assert_eq!(receipt.receipt_ref, canonical_hash(&receipt.value).expect("receipt hash"));
    }

    #[test]
    fn supplied_replay_fixture_verifies_against_recorded_refs() {
        let fixture = record_fixture_value().expect("fixture record");
        let from_fixture = verify_fixture_record_value(&fixture.value).expect("verify supplied fixture");
        let generated = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("verify generated baseline");
        assert_eq!(from_fixture.decision, "pass");
        assert_eq!(from_fixture.divergence, ReplayDivergenceKind::None);
        assert_eq!(from_fixture.receipt_ref, generated.receipt_ref);
        assert_eq!(from_fixture.receipt_ref, canonical_hash(&from_fixture.value).expect("fixture receipt hash"));
    }

    #[test]
    fn supplied_tampered_replay_fixture_denies_at_first_divergence() {
        let fixture = tampered_fixture_record_value(ReplayFixtureVariant::ChangedEffectResponse)
            .expect("tampered fixture record");
        let receipt = verify_fixture_record_value(&fixture.value).expect("verify tampered fixture");
        assert_eq!(receipt.decision, "deny");
        assert_eq!(receipt.divergence, ReplayDivergenceKind::EffectResponse);
        let divergence = receipt.first_divergence.expect("first divergence");
        let text = to_text(&divergence).expect("render divergence");
        assert!(text.contains("effect-response"));
        assert!(text.contains("safe-canonical-refs-only"));
    }

    #[test]
    fn malformed_replay_fixture_ref_mismatch_fails_closed() {
        const STALE_IDENTITY_REF: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
        let fixture = record_fixture_value().expect("fixture record");
        let text = to_text(&fixture.value).expect("render fixture")
            .replacen(&fixture.identity_ref, STALE_IDENTITY_REF, 1);
        let malformed = crate::preserves_rail::parse_text(&text).expect("parse malformed fixture");
        let error = verify_fixture_record_value(&malformed).expect_err("malformed fixture denied");
        assert!(error.to_string().contains("identity ref mismatch"));
    }

    #[test]
    fn replay_reports_first_divergence_matrix() {
        let cases = [
            (ReplayFixtureVariant::ChangedIdentity, ReplayDivergenceKind::Identity),
            (ReplayFixtureVariant::ChangedScheduler, ReplayDivergenceKind::Scheduler),
            (ReplayFixtureVariant::ChangedInput, ReplayDivergenceKind::Input),
            (ReplayFixtureVariant::ChangedEffectRequest, ReplayDivergenceKind::EffectRequest),
            (ReplayFixtureVariant::ChangedEffectResponse, ReplayDivergenceKind::EffectResponse),
            (ReplayFixtureVariant::ChangedPolicyDecision, ReplayDivergenceKind::PolicyDecision),
            (ReplayFixtureVariant::ChangedAction, ReplayDivergenceKind::Action),
            (ReplayFixtureVariant::ChangedReceipt, ReplayDivergenceKind::Receipt),
            (ReplayFixtureVariant::ChangedOutput, ReplayDivergenceKind::Output),
            (ReplayFixtureVariant::ChangedStateHash, ReplayDivergenceKind::StateHash),
        ];
        for (variant, expected) in cases {
            let receipt = verify_fixture_value(variant).expect("verify divergent fixture");
            assert_eq!(receipt.decision, "deny");
            assert_eq!(receipt.divergence, expected);
            let divergence = receipt.first_divergence.expect("first divergence");
            let text = to_text(&divergence).expect("render divergence");
            assert!(text.contains(expected.as_str()));
            assert!(text.contains("safe-canonical-refs-only"));
        }
    }

    #[test]
    fn replay_profile_denies_live_external_effects() {
        let receipt = verify_fixture_value(ReplayFixtureVariant::MissingRecordedEffect).expect("verify missing effect");
        assert_eq!(receipt.decision, "deny");
        assert_eq!(receipt.divergence, ReplayDivergenceKind::LiveEffect);
        let text = to_text(&receipt.value).expect("render receipt");
        assert!(text.contains("recorded-effects-only"));
        assert!(text.contains("live-effect"));
    }

    #[test]
    fn replay_rollup_summarizes_pass_deny_and_divergence_counts() {
        let pass = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("pass replay");
        let deny = verify_fixture_value(ReplayFixtureVariant::ChangedEffectResponse).expect("deny replay");
        let rollup = rollup_replay_receipts(&[
            ReplayRollupInput {
                expected_ref: Some(pass.receipt_ref.clone()),
                value: pass.value,
            },
            ReplayRollupInput {
                expected_ref: Some(deny.receipt_ref.clone()),
                value: deny.value,
            },
        ])
        .expect("rollup replay receipts");
        assert_eq!(rollup.decision, "deny");
        assert_eq!(rollup.total_count, 2);
        assert_eq!(rollup.pass_count, 1);
        assert_eq!(rollup.deny_count, 1);
        assert_eq!(rollup.rollup_ref, canonical_hash(&rollup.value).expect("rollup hash"));
        let text = to_text(&rollup.value).expect("render rollup");
        assert!(text.contains("deterministic-replay-rollup-v1"));
        assert!(text.contains("effect-response"));
        assert!(text.contains("individual-receipts-required"));
    }

    #[test]
    fn replay_rollup_denies_mismatched_receipt_refs_without_counting_them() {
        let pass = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("pass replay");
        let wrong_ref = canonical_hash(&record_fixture_value().expect("fixture").value).expect("fixture ref");
        let rollup = rollup_replay_receipts(&[ReplayRollupInput {
            expected_ref: Some(wrong_ref.clone()),
            value: pass.value,
        }])
        .expect("rollup replay receipts");
        assert_eq!(rollup.decision, "deny");
        assert_eq!(rollup.total_count, 0);
        let text = to_text(&rollup.value).expect("render rollup");
        assert!(text.contains("replay receipt ref mismatch"));
        assert!(text.contains(&wrong_ref));
        assert!(text.contains("all-inputs-readable"));
    }

    #[test]
    fn replay_snapshots_and_logs_are_manifest_backed_for_partial_debug_fetch() {
        let root = temp_dir("replay-snapshot-manifests");
        let bundle = replay_snapshot_manifest_bundle(&root, ReplayFixtureVariant::ChangedEffectResponse)
            .expect("snapshot manifest bundle");
        assert!(bundle.bundle_ref.starts_with("blake3:"));
        assert!(bundle.effect_log_manifest_ref.starts_with("blake3:"));
        assert!(bundle.turn_journal_manifest_ref.starts_with("blake3:"));
        assert!(bundle.snapshot_manifest_ref.starts_with("blake3:"));
        let first_divergence_manifest_ref =
            bundle.first_divergence_manifest_ref.as_ref().expect("first divergence manifest ref");
        assert!(first_divergence_manifest_ref.starts_with("blake3:"));
        assert!(bundle.debug_range_receipt_ref.as_ref().expect("range receipt").starts_with("blake3:"));
        let effect_log_read = read_object(&root, &bundle.effect_log_manifest_ref).expect("read effect log");
        assert!(parse_canonical_bytes(&effect_log_read.bytes).is_ok());
        let range = range_read(&root, first_divergence_manifest_ref, 0, 16).expect("partial divergence range");
        assert_eq!(range.bytes.len(), 16);
        let text = to_text(&bundle.value).expect("render bundle");
        assert!(text.contains("partial-divergence-debug-fetch"));
    }

    #[test]
    fn replay_index_combines_rollups_and_raw_receipts() {
        let pass = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("pass replay");
        let deny = verify_fixture_value(ReplayFixtureVariant::ChangedOutput).expect("deny replay");
        let rollup = rollup_replay_receipts(&[ReplayRollupInput {
            expected_ref: Some(pass.receipt_ref.clone()),
            value: pass.value,
        }])
        .expect("rollup replay receipts");
        let index = index_replay_evidence(&[
            ReplayIndexInput {
                expected_ref: Some(rollup.rollup_ref.clone()),
                value: rollup.value,
            },
            ReplayIndexInput {
                expected_ref: Some(deny.receipt_ref.clone()),
                value: deny.value,
            },
        ])
        .expect("index replay evidence");
        assert_eq!(index.decision, "deny");
        assert_eq!(index.total_count, 2);
        assert_eq!(index.pass_count, 1);
        assert_eq!(index.deny_count, 1);
        assert_eq!(index.raw_receipt_count, 1);
        assert_eq!(index.rollup_count, 1);
        assert_eq!(index.index_ref, canonical_hash(&index.value).expect("index hash"));
        let text = to_text(&index.value).expect("render index");
        assert!(text.contains("deterministic-replay-index-v1"));
        assert!(text.contains("rollup-and-receipt-refs-verified"));
        assert!(text.contains("output"));
    }

    #[test]
    fn replay_index_denies_mismatched_rollup_ref() {
        let pass = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("pass replay");
        let rollup = rollup_replay_receipts(&[ReplayRollupInput {
            expected_ref: Some(pass.receipt_ref.clone()),
            value: pass.value,
        }])
        .expect("rollup replay receipts");
        let wrong_ref = canonical_hash(&record_fixture_value().expect("fixture").value).expect("fixture ref");
        let index = index_replay_evidence(&[ReplayIndexInput {
            expected_ref: Some(wrong_ref.clone()),
            value: rollup.value,
        }])
        .expect("index replay evidence");
        assert_eq!(index.decision, "deny");
        assert_eq!(index.total_count, 0);
        let text = to_text(&index.value).expect("render index");
        assert!(text.contains("replay index ref mismatch"));
        assert!(text.contains(&wrong_ref));
    }

    #[test]
    fn deterministic_integration_gates_bind_recorded_replay_inputs() {
        for integration_kind in ["remote-sync", "storage", "job-dag", "upgrade"] {
            let receipt = deterministic_integration_receipt(&DeterministicIntegrationInput {
                integration_kind: integration_kind.to_string(),
                handler_profile_ref: DEFAULT_HANDLER_PROFILE_REF.to_string(),
                effect_log_ref: DEFAULT_SEED_REF.to_string(),
                snapshot_ref: DEFAULT_INITIAL_STATE_REF.to_string(),
                gate_ref: DEFAULT_ARTIFACT_REF.to_string(),
                admitted_live_effects: false,
            })
            .expect("integration receipt");
            assert_eq!(receipt.decision, "pass");
            assert_eq!(receipt.receipt_ref, canonical_hash(&receipt.value).expect("receipt ref"));
            let text = to_text(&receipt.value).expect("render integration receipt");
            assert!(text.contains(integration_kind));
            assert!(text.contains("handler-profile-bound"));
            assert!(text.contains("effect-log-bound"));
            assert!(text.contains("snapshot-bound"));
        }
        let denied = deterministic_integration_receipt(&DeterministicIntegrationInput {
            integration_kind: "remote-sync".to_string(),
            handler_profile_ref: DEFAULT_HANDLER_PROFILE_REF.to_string(),
            effect_log_ref: DEFAULT_SEED_REF.to_string(),
            snapshot_ref: DEFAULT_INITIAL_STATE_REF.to_string(),
            gate_ref: DEFAULT_ARTIFACT_REF.to_string(),
            admitted_live_effects: true,
        })
        .expect("integration denial");
        assert_eq!(denied.decision, "deny");
        assert!(to_text(&denied.value).expect("denial text").contains("no-live-effect-during-replay"));
    }

    #[test]
    fn trace_privacy_gates_sensitive_trace_and_snapshot_exports() {
        let input = TracePrivacyInput {
            trace_ref: DEFAULT_ARTIFACT_REF.to_string(),
            snapshot_ref: DEFAULT_INITIAL_STATE_REF.to_string(),
            requester_ref: DEFAULT_CAPABILITY_REF.to_string(),
            policy_ref: DEFAULT_POLICY_REF.to_string(),
            has_export_authority: false,
            contains_sensitive_refs: true,
        };
        let denied = trace_privacy_receipt(&input).expect("trace privacy deny");
        assert_eq!(denied.decision, "deny");
        assert_eq!(denied.receipt_ref, canonical_hash(&denied.value).expect("privacy receipt ref"));
        let denied_text = to_text(&denied.value).expect("render denied privacy receipt");
        assert!(denied_text.contains("policy-admission-before-render"));
        assert!(denied_text.contains("sensitive-trace-gated"));

        let redacted = trace_privacy_receipt(&TracePrivacyInput {
            has_export_authority: true,
            ..input.clone()
        })
        .expect("trace privacy redacted");
        assert_eq!(redacted.decision, "redacted");
        let redacted_text = to_text(&redacted.value).expect("render redacted privacy receipt");
        assert!(redacted_text.contains("redacted-view-when-authorized-sensitive"));

        let public = trace_privacy_receipt(&TracePrivacyInput {
            contains_sensitive_refs: false,
            ..input
        })
        .expect("trace privacy public");
        assert_eq!(public.decision, "pass");
    }
