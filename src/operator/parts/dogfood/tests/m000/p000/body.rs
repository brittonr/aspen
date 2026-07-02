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

    #[test]
    fn nix_dogfood_release_evidence_verifies_and_denies_stale_refs() {
        let case = build_nix_case();
        assert_release_binding_search(&case);
        let signed_members = signed_members(&case);
        let signed_bundle_verify = signed_bundle_pass(&case, &signed_members);
        let key = signed_key();
        assert_promotion_receipts(&case, &signed_bundle_verify, &key);
        assert_signed_denials(&case, &signed_members, &key);
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
                "dogfood local-node decision=pass report={} release-gate={}\n",
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

    fn assert_release_binding_search(case: &NixCase) {
        let catalog_registry = case.root.join("catalog-registry");
        let release_ledger = case.root.join("release-ledger");
        crate::ledger::import_artifact(&release_ledger, case.run.release_gate_value.as_ref().expect("release gate"))
            .expect("import release gate");
        crate::ledger::import_artifact(&release_ledger, case.run.replay_verify_value.as_ref().expect("replay verify"))
            .expect("import replay verify");
        crate::ledger::import_artifact(&release_ledger, case.run.replay_index_value.as_ref().expect("replay index"))
            .expect("import replay index");
        crate::ledger::import_artifact(&release_ledger, &case.evidence).expect("import Nix evidence");
        crate::ledger::import_artifact(&release_ledger, &case.bundle_verify.value).expect("import bundle verify");
        let replay_binding_request = crate::catalog_mcp::mcp_request_value("search_replay_evidence", vec![
            crate::preserves_rail::record("stage", vec![crate::preserves_rail::string("release-binding")]),
            crate::preserves_rail::record("release-replay-index-ref", vec![crate::preserves_rail::string(
                &case.parsed.replay_index_ref,
            )]),
        ])
        .expect("replay binding request");
        let replay_binding =
            crate::catalog_mcp::call(&catalog_registry, Some(&release_ledger), &replay_binding_request)
                .expect("replay binding search");
        assert_eq!(replay_binding.decision, "pass");
        assert!(
            to_text(&replay_binding.response_value)
                .expect("replay binding response")
                .contains("deterministic-replay:release-binding")
        );
    }

    fn signed_members(case: &NixCase) -> Vec<IoValue> {
        vec![
            sign_member(&case.run.report_value),
            sign_member(case.run.release_gate_value.as_ref().expect("release gate")),
            sign_member(case.run.replay_verify_value.as_ref().expect("replay verify")),
            sign_member(case.run.replay_index_value.as_ref().expect("replay index")),
            sign_member(&case.evidence),
            sign_member(&case.receipt.value),
        ]
    }

    fn sign_member(receipt: &IoValue) -> IoValue {
        sign_receipt(&SignReceiptInput {
            receipt,
            signer: "release-signer",
            purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
            trust_root: "release-root",
            key: "release-key",
            parents: &[],
        })
        .expect("sign member")
    }

    fn signed_bundle_pass(case: &NixCase, signed_members: &[IoValue]) -> ReleaseEvidenceBundleVerifyReceipt {
        let signed_bundle_verify = required_bundle_verify(case, signed_members, Some("release-signer"), None);
        assert_eq!(signed_bundle_verify.decision, "pass");
        signed_bundle_verify
    }
