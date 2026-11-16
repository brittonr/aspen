    type PathBuf = std::path::PathBuf;
    type SignReceiptInput<'a> = crate::evidence::SignReceiptInput<'a>;
    type SignedReceiptKeyInput<'a> = crate::evidence::SignedReceiptKeyInput<'a>;
    type SignedReceiptKeyRevocationInput<'a> = crate::evidence::SignedReceiptKeyRevocationInput<'a>;

    use super::*;

    fn parse_signed_receipt_key(value: &IoValue) -> Result<SignedReceiptKey> {
        crate::evidence::parse_signed_receipt_key(value)
    }

    fn parse_signed_receipt_key_revocation(value: &IoValue) -> Result<SignedReceiptKeyRevocation> {
        crate::evidence::parse_signed_receipt_key_revocation(value)
    }

    fn sign_receipt(input: &SignReceiptInput<'_>) -> Result<IoValue> {
        crate::evidence::sign_receipt(input)
    }

    fn signed_receipt_key_value(input: &SignedReceiptKeyInput<'_>) -> Result<IoValue> {
        crate::evidence::signed_receipt_key_value(input)
    }

    fn signed_receipt_key_revocation_value(input: &SignedReceiptKeyRevocationInput<'_>) -> Result<IoValue> {
        crate::evidence::signed_receipt_key_revocation_value(input)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    #[test]
    fn local_node_dogfood_runs_and_gates_release() {
        let root = temp_dir("operator-dogfood-pass");
        let run = run_local_node_dogfood(&LocalNodeDogfoodInput { state_root: &root }).expect("dogfood run");
        assert_eq!(run.decision, "pass", "{}", to_text(&run.report_value).expect("report text"));
        let release_gate_ref = run.release_gate_ref.as_deref().expect("release gate ref");
        crate::preserves_rail::validate_content_ref(release_gate_ref).expect("release gate ref is canonical");
        assert_eq!(crate::ledger::artifact_kind(&run.workflow_value), "operator-workflow");
        assert_eq!(crate::ledger::artifact_kind(&run.report_value), "dogfood-report");
        assert_eq!(
            crate::ledger::artifact_kind(run.release_gate_value.as_ref().expect("release gate")),
            "release-gate-receipt"
        );
        let entries = crate::ledger::list_artifacts(&root.join("ledger")).expect("ledger entries");
        assert!(entries.iter().any(|entry| entry.artifact_kind == "dogfood-report"));
        assert!(entries.iter().any(|entry| entry.artifact_kind == "operator-checkpoint"));
        assert!(entries.iter().any(|entry| entry.artifact_kind == "retention-gc-audit"));
        assert!(entries.iter().any(|entry| entry.artifact_kind == "retention-candidate-bundle-verify"));
        let workflow = parse_operator_workflow(&run.workflow_value).expect("parse workflow");
        assert!(workflow.steps.iter().any(|step| step.name == "plan-retention-gc"));
        assert!(workflow.steps.iter().any(|step| step.name == "apply-retention-gc-plan"));
        assert!(workflow.steps.iter().any(|step| step.name == "execute-retention-gc"));
        assert!(workflow.steps.iter().any(|step| step.name == "audit-retention-gc"));
        assert!(workflow.steps.iter().any(|step| step.name == "export-retention-gc-bundle"));
        assert!(workflow.steps.iter().any(|step| step.name == "search-retention-gc-catalog"));
        assert!(workflow.steps.iter().any(|step| step.name == "index-replay-evidence"));
        assert_eq!(
            crate::ledger::artifact_kind(run.replay_index_value.as_ref().expect("replay index")),
            "deterministic-replay-index"
        );
        let release_text = to_text(run.release_gate_value.as_ref().expect("release gate text")).expect("release text");
        assert!(release_text.contains("replay-evidence-index-bound"));
        assert!(release_text.contains("replay-index-is-evidence-only"));
        assert!(release_text.contains("retention-gc-review-bound"));
        assert!(release_text.contains("retention-gc-is-evidence-only"));
        assert!(operator_dogfood_summary(&run.report_value).expect("summary").contains("decision=pass"));
    }

    // r[verify molten.operator_dogfood_node_workflow.spec.unqualified_fixture_gate]
    // r[verify molten.octet_source_gate_receipt_validation.spec.unqualified_fixture_denied]
    #[test]
    fn dogfood_binds_only_labelled_fixture_gate_that_real_consumers_deny() {
        let root = temp_dir("operator-dogfood-fixture-gate");
        let run = run_local_node_dogfood(&LocalNodeDogfoodInput { state_root: &root }).expect("dogfood run");
        assert_eq!(run.decision, "pass");
        let report = parse_dogfood_report(&run.report_value).expect("report");
        let is_labelled = |checks: &[(String, String)]| {
            checks.iter().any(|(name, status)| name == UNQUALIFIED_FIXTURE_SOURCE_GATE_CHECK && status == "pass")
        };
        assert!(is_labelled(&report.checks));
        let release = parse_release_gate_receipt(run.release_gate_value.as_ref().expect("release gate")).expect("gate");
        assert!(is_labelled(&release.checks));
        let registries =
            [root.join("registry"), root.join("job-source-registry"), root.join("job-target-registry")];
        assert!(registries.iter().all(|registry| registry.exists()));
        for registry in &registries {
            let gates = crate::artifacts::list_artifacts(registry, Some("octet-gate-receipt")).expect("list gates");
            assert!(gates.is_empty(), "dogfood installs no Octet gate receipt into {}", registry.display());
        }

        let fixture = dogfood_fixture_gate().expect("dogfood fixture gate");
        for consumer in ["node-startup", "job-remote-admission", "upgrade-plan"] {
            let validation =
                crate::octet_gate::validate_octet_source_gate(&crate::octet_gate::OctetSourceGateValidationInput {
                    consumer: consumer.to_string(),
                    subject_ref: dogfood_ref("fixture-subject").expect("subject"),
                    receipt_value: Some(fixture.value().clone()),
                    source_scope: Vec::new(),
                })
                .expect("validate fixture");
            assert_eq!(validation.decision, "deny", "{consumer}");
        }
        crate::octet_gate::validate_produced_octet_gate_receipt(fixture.value())
            .expect_err("produced-gate validation rejects the dogfood fixture");
    }

    #[test]
    fn nix_dogfood_release_evidence_verifies_and_denies_stale_refs() {
        let case = build_nix_case();
        assert_release_binding_search(&case);
        let signed_members = signed_members(&case);
        let signed_bundle_verify = signed_bundle_pass(&case, &signed_members);
        let key = signed_key();
        assert_promotion_receipts(&case, &signed_bundle_verify, &key);
        assert_ordered_release_workflow(&case, &signed_bundle_verify, &key);
        assert_signed_denials(&case, &signed_members, &key);
        assert_missing_duplicate_and_tampered_bundle_members_deny(&case);
        assert_stale_bundle_denies(&case);
        assert_stale_evidence_denies(&case);
    }

    struct NixCase {
        root: PathBuf,
        output_root: PathBuf,
        run: LocalNodeDogfoodRun,
        parsed: NixDogfoodEvidence,
        evidence: IoValue,
        receipt: NixDogfoodVerifyReceipt,
        bundle: IoValue,
        parsed_bundle: ReleaseEvidenceBundle,
        bundle_verify: ReleaseEvidenceBundleVerifyReceipt,
    }

    struct PromotionInput<'a> {
        output_path: &'a std::path::Path,
        bundle_verify_value: &'a IoValue,
        source_evidence: &'a str,
        key: &'a SignedReceiptKey,
        revocations: &'a [SignedReceiptKeyRevocation],
    }

    fn build_nix_case() -> NixCase {
        let root = temp_dir("operator-dogfood-nix-evidence");
        let state_root = root.join("state");
        let output_root = root.join("nix-output");
        std::fs::create_dir_all(&output_root).expect("create nix output");
        let run = run_local_node_dogfood(&LocalNodeDogfoodInput {
            state_root: &state_root,
        })
        .expect("dogfood run");
        write_run_outputs(&output_root, &run);
        let evidence = nix_dogfood_release_evidence_value(&NixDogfoodEvidenceInput {
            output_path: &output_root,
        })
        .expect("nix evidence");
        let parsed = parse_nix_dogfood_evidence(&evidence).expect("parse nix evidence");
        assert_eq!(crate::ledger::artifact_kind(&evidence), "nix-dogfood-release-evidence");
        assert_eq!(parsed.release_gate_ref, run.release_gate_ref.clone().expect("release ref"));
        assert_eq!(parsed.replay_verify_ref, run.replay_verify_ref.clone().expect("replay verify ref"));
        assert_eq!(parsed.replay_index_ref, run.replay_index_ref.clone().expect("replay index ref"));
        let receipt = verify_nix_dogfood_evidence(&NixDogfoodVerifyInput {
            output_path: &output_root,
            evidence_value: &evidence,
        })
        .expect("verify nix evidence");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&receipt.value), "nix-dogfood-release-verify-receipt");
        assert_tampered_replay_denies(&output_root, &run, &evidence);
        write_bundle_inputs(&output_root, &evidence, &receipt);
        let bundle = release_evidence_bundle_value(&ReleaseEvidenceBundleInput {
            output_path: &output_root,
        })
        .expect("release bundle");
        let parsed_bundle = parse_release_evidence_bundle(&bundle).expect("parse release bundle");
        assert_eq!(crate::ledger::artifact_kind(&bundle), "release-evidence-bundle");
        assert!(parsed_bundle.schema_ref.starts_with("blake3:"));
        assert_eq!(parsed_bundle.report_ref, parsed.report_ref);
        assert_eq!(parsed_bundle.replay_verify_ref, parsed.replay_verify_ref);
        assert_eq!(parsed_bundle.replay_index_ref, parsed.replay_index_ref);
        let bundle_verify = unsigned_bundle_verify(&output_root, &bundle);
        assert_eq!(bundle_verify.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&bundle_verify.value), "release-evidence-bundle-verify-receipt");
        NixCase {
            root,
            output_root,
            run,
            parsed,
            evidence,
            receipt,
            bundle,
            parsed_bundle,
            bundle_verify,
        }
    }

    fn write_run_outputs(output_root: &std::path::Path, run: &LocalNodeDogfoodRun) {
        std::fs::write(output_root.join("dogfood-report.preserves"), to_text(&run.report_value).expect("report text"))
            .expect("write report");
        std::fs::write(
            output_root.join("release-gate.preserves"),
            to_text(run.release_gate_value.as_ref().expect("release gate")).expect("release text"),
        )
        .expect("write release gate");
        std::fs::write(
            output_root.join("replay-verify.preserves"),
            to_text(run.replay_verify_value.as_ref().expect("replay verify")).expect("replay verify text"),
        )
        .expect("write replay verify");
        std::fs::write(
            output_root.join("replay-evidence-index.preserves"),
            to_text(run.replay_index_value.as_ref().expect("replay index")).expect("replay index text"),
        )
        .expect("write replay index");
        std::fs::write(
            output_root.join("dogfood-summary.txt"),
            format!(
                "dogfood local-node decision=pass report={} release-gate={} source-gate=unqualified-fixture\n",
                run.report_ref,
                run.release_gate_ref.as_deref().expect("release ref")
            ),
        )
        .expect("write summary");
        std::fs::write(output_root.join("after-nextest.txt"), "/nix/store/test-molten-nextest\n")
            .expect("write nextest marker");
    }

    fn assert_tampered_replay_denies(output_root: &std::path::Path, run: &LocalNodeDogfoodRun, evidence: &IoValue) {
        std::fs::write(output_root.join("replay-evidence-index.preserves"), "<tampered-replay-index>\n")
            .expect("tamper replay index");
        let tampered_replay_verify = verify_nix_dogfood_evidence(&NixDogfoodVerifyInput {
            output_path: output_root,
            evidence_value: evidence,
        })
        .expect("verify tampered replay index evidence");
        assert_eq!(tampered_replay_verify.decision, "deny");
        assert!(
            tampered_replay_verify
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("replay index") || diagnostic.contains("observation failed"))
        );
        std::fs::write(
            output_root.join("replay-evidence-index.preserves"),
            to_text(run.replay_index_value.as_ref().expect("replay index")).expect("replay index text"),
        )
        .expect("restore replay index");
    }

    fn write_bundle_inputs(output_root: &std::path::Path, evidence: &IoValue, receipt: &NixDogfoodVerifyReceipt) {
        std::fs::write(output_root.join("nix-dogfood-evidence.preserves"), to_text(evidence).expect("evidence text"))
            .expect("write evidence");
        std::fs::write(output_root.join("nix-dogfood-verify.preserves"), to_text(&receipt.value).expect("verify text"))
            .expect("write verify");
    }

    fn unsigned_bundle_verify(output_root: &std::path::Path, bundle: &IoValue) -> ReleaseEvidenceBundleVerifyReceipt {
        verify_release_evidence_bundle(&ReleaseEvidenceBundleVerifyInput {
            output_path: output_root,
            bundle_value: bundle,
            signed_member_values: &[],
            signed_purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
            signed_trust_root: "local-release-trust-root",
            signed_key: "local-release-key",
            signed_keys: &[],
            signed_key_revocations: &[],
            signed_key_ref: None,
            signed_key_id: None,
            signed_signer: None,
            is_signed_members_required: false,
        })
        .expect("verify release bundle")
    }

