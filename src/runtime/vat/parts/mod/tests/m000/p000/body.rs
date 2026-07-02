    type TestCase = hegel::TestCase;

    use super::*;

    #[test]
    fn vat_fixture_binds_near_far_actormap_pipeline_and_revocation_predicates() {
        let run = run_vat_fixture().expect("vat fixture run");
        assert_eq!(run.receipts.len(), 7);
        assert!(run.receipts.iter().any(|receipt| receipt.predicate == "molten.trellis-runtime.near-far-refs.v1"));
        assert!(
            run.receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.actormap-transaction.v1")
        );
        assert!(run.receipts.iter().any(|receipt| receipt.predicate == "molten.trellis-runtime.promise-pipeline.v1"));
        assert!(
            run.receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.revocation-cleanup.v1")
        );
        assert!(run.receipts.iter().any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Deny));
        assert!(run.diagnostics.iter().any(|diagnostic| diagnostic == "expected-denials-present"));
        assert!(run.run_ref.starts_with("blake3:"));
    }

    #[test]
    fn vat_fixture_summary_uses_canonical_ref() {
        let run = run_vat_fixture().expect("vat fixture run");
        let summary = vat_fixture_summary(&run.value).expect("summary");
        assert!(summary.contains(&run.run_ref));
    }

    #[test]
    fn vat_snapshot_fixture_denies_unheld_authority() {
        let snapshot = run_vat_snapshot_fixture().expect("snapshot fixture");
        assert_eq!(snapshot.receipts.len(), 2);
        assert!(snapshot.snapshot_ref.starts_with("blake3:"));
        assert!(snapshot.fixture_ref.starts_with("blake3:"));
        assert!(
            snapshot
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.snapshot-authority.v1")
        );
        assert!(snapshot.receipts.iter().any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Pass));
        assert!(snapshot.receipts.iter().any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Deny));
        assert!(
            snapshot
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "snapshot-claimed-authority-not-admitted")
        );
    }

    #[test]
    fn vat_restore_fixture_records_upgrade_and_missing_recipe_denial() {
        let restore = run_vat_restore_fixture().expect("restore fixture");
        assert_eq!(restore.receipts.len(), 2);
        assert!(restore.fixture_ref.starts_with("blake3:"));
        assert!(restore.diagnostics.iter().all(|diagnostic| diagnostic.starts_with("restore-receipt:blake3:")));
        let rendered = to_text(&restore.value).expect("render restore fixture");
        assert!(rendered.contains("vat-object-upgrade-recipe-v1"));
        assert!(rendered.contains("missing-compatible-upgrade-recipe"));
    }

    #[test]
    fn vat_distributed_ref_fixture_records_lifetime_and_handoff() {
        let distributed_ref = run_vat_distributed_ref_fixture().expect("distributed ref fixture");
        assert_eq!(distributed_ref.receipts.len(), 5);
        assert!(distributed_ref.fixture_ref.starts_with("blake3:"));
        assert!(
            distributed_ref
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.distributed-ref-lifetime.v1")
        );
        assert!(
            distributed_ref
                .receipts
                .iter()
                .any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Pass)
        );
        assert!(
            distributed_ref
                .receipts
                .iter()
                .any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Deny)
        );
        assert!(
            distributed_ref
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "distributed-ref-stale-descriptor-used")
        );
        assert!(
            distributed_ref
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "distributed-ref-disconnected-pending-calls-not-failed")
        );
    }

    #[test]
    fn vat_rights_fixture_records_unseal_and_denials() {
        let rights = run_vat_rights_fixture().expect("rights fixture");
        assert_eq!(rights.receipts.len(), 3);
        assert!(rights.fixture_ref.starts_with("blake3:"));
        assert!(
            rights
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.rights-amplification.v1")
        );
        assert!(rights.receipts.iter().any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Pass));
        assert!(rights.receipts.iter().any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Deny));
        assert!(
            rights
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "rights-amplification-brand-mismatch")
        );
        assert!(
            rights
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "rights-amplification-recovered-authority-not-sealed")
        );
    }

    #[test]
    fn vat_ambient_authority_fixture_denies_unendowed_authority() {
        let authority = run_vat_ambient_authority_fixture().expect("ambient authority fixture");
        assert_eq!(authority.receipts.len(), 11);
        assert!(authority.fixture_ref.starts_with("blake3:"));
        assert!(
            authority
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.object-authority.v1")
        );
        assert!(authority.receipts.iter().any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Pass));
        assert!(authority.receipts.iter().any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Deny));
        assert!(
            authority
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "object-authority-not-endowed")
        );
        assert!(
            authority
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "object-authority-not-policy-admitted")
        );
    }

    #[test]
    fn vat_promise_fixture_records_terminal_results_and_denials() {
        let promise = run_vat_promise_fixture().expect("promise fixture");
        assert_eq!(promise.receipts.len(), 6);
        assert!(promise.fixture_ref.starts_with("blake3:"));
        assert!(
            promise
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.promise-state.v1")
        );
        assert!(
            promise
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.promise-pipeline.v1")
        );
        assert!(promise.receipts.iter().any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Pass));
        assert!(promise.receipts.iter().any(|receipt| receipt.decision == crate::runtime::PredicateDecision::Deny));
        assert!(
            promise
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "terminal-promise-state-changed")
        );
        assert!(
            promise
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "terminal-promise-pipeline-not-cleaned")
        );
    }

    #[test]
    fn vat_time_travel_fixture_records_trace_snapshot_replay_hooks() {
        let debug = run_vat_time_travel_fixture().expect("time travel fixture");
        assert_eq!(debug.receipts.len(), 2);
        assert!(debug.fixture_ref.starts_with("blake3:"));
        assert!(debug.diagnostics.iter().any(|diagnostic| diagnostic == "evidence-only-debugging-surface"));
        let rendered = to_text(&debug.value).expect("render time travel fixture");
        assert!(rendered.contains("vat-time-travel-debug-receipt-v1"));
        assert!(rendered.contains("debug-authority-missing"));
        assert!(rendered.contains("deterministic-replay"));
    }

    #[test]
    fn vat_replay_fixture_reports_identity_and_first_divergence() {
        let replay = run_vat_replay_fixture().expect("replay fixture");
        assert_eq!(replay.receipts.len(), 6);
        assert!(replay.fixture_ref.starts_with("blake3:"));
        let rendered = to_text(&replay.value).expect("render replay fixture");
        assert!(rendered.contains("vat-replay-receipt-v1"));
        assert!(rendered.contains("deterministic-replay-verify-v1"));
        assert!(rendered.contains("deterministic-first-divergence-v1"));
        assert!(rendered.contains("evidence-only-debugging-surface"));
        assert!(rendered.contains("deterministic-replay-identical-trace-and-state"));
        assert!(rendered.contains("first-divergence-input"));
        assert!(rendered.contains("first-divergence-effect-response"));
        assert!(rendered.contains("first-divergence-effect-request"));
        assert!(rendered.contains("first-divergence-policy-decision"));
        assert!(rendered.contains("first-divergence-state-hash"));
        assert!(rendered.contains("logical-clock-response-stable"));
        assert!(rendered.contains("seeded-random-response-stable"));
        assert!(rendered.contains("replay-profile-denies-real-external-effects"));
    }

    #[test]
    fn vat_authority_graph_fixture_records_inspection_denials() {
        let graph = run_vat_authority_graph_fixture().expect("authority graph fixture");
        assert_eq!(graph.receipts.len(), 2);
        assert!(graph.fixture_ref.starts_with("blake3:"));
        let rendered = to_text(&graph.value).expect("render authority graph fixture");
        assert!(rendered.contains("vat-authority-graph-inspect-receipt-v1"));
        assert!(rendered.contains("proxy-chain-visible"));
        assert!(rendered.contains("inspection-authority-missing"));
    }

    #[test]
    fn vat_portable_storage_fixture_records_encrypted_chunked_storage() {
        let storage = run_vat_portable_storage_fixture().expect("portable storage fixture");
        assert_eq!(storage.receipts.len(), 2);
        assert!(storage.fixture_ref.starts_with("blake3:"));
        let rendered = to_text(&storage.value).expect("render portable storage fixture");
        assert!(rendered.contains("vat-portable-storage-receipt-v1"));
        assert!(rendered.contains("content-addressed-chunked-encrypted"));
        assert!(rendered.contains("plaintext-storage-denied"));
    }
