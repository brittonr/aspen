
    fn signed_key() -> SignedReceiptKey {
        let key_value = signed_receipt_key_value(&SignedReceiptKeyInput {
            key_id: "release-key-1",
            signer: "release-signer",
            trust_root: "release-root",
            key: "release-key",
            generation: 1,
            predecessor_ref: None,
        })
        .expect("signed key value");
        parse_signed_receipt_key(&key_value).expect("parse signed key")
    }

    fn assert_promotion_receipts(
        case: &NixCase,
        signed_bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
        key: &SignedReceiptKey,
    ) {
        let promotion = promotion_receipt(PromotionInput {
            output_path: &case.output_root,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "source:working-tree-reviewed",
            key,
            revocations: &[],
        });
        assert_eq!(promotion.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&promotion.value), "release-promotion-gate-receipt");
        let revocation = signed_revocation(key);
        assert_revoked_promotion(case, signed_bundle_verify, key, &revocation);
        assert_missing_source_promotion(case, signed_bundle_verify, key);
        assert_stale_output_promotion(case, signed_bundle_verify, key);
    }

    fn signed_revocation(key: &SignedReceiptKey) -> SignedReceiptKeyRevocation {
        let revocation_value = signed_receipt_key_revocation_value(&SignedReceiptKeyRevocationInput {
            key,
            reason: "test-revoked",
            superseded_by: None,
        })
        .expect("revocation value");
        parse_signed_receipt_key_revocation(&revocation_value).expect("parse revocation")
    }

    fn assert_revoked_promotion(
        case: &NixCase,
        signed_bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
        key: &SignedReceiptKey,
        revocation: &SignedReceiptKeyRevocation,
    ) {
        let revoked_promotion = promotion_receipt(PromotionInput {
            output_path: &case.output_root,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "source:working-tree-reviewed",
            key,
            revocations: std::slice::from_ref(revocation),
        });
        assert_eq!(revoked_promotion.decision, "deny");
        assert!(
            revoked_promotion
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("revoked") || diagnostic.contains("stale"))
        );
    }

    fn assert_missing_source_promotion(
        case: &NixCase,
        signed_bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
        key: &SignedReceiptKey,
    ) {
        let missing_source_promotion = promotion_receipt(PromotionInput {
            output_path: &case.output_root,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "",
            key,
            revocations: &[],
        });
        assert_eq!(missing_source_promotion.decision, "deny");
        assert!(missing_source_promotion.diagnostics.iter().any(|diagnostic| diagnostic.contains("source evidence")));
    }

    fn assert_stale_output_promotion(
        case: &NixCase,
        signed_bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
        key: &SignedReceiptKey,
    ) {
        let stale_output = case.output_root.join("stale-output");
        let stale_output_promotion = promotion_receipt(PromotionInput {
            output_path: &stale_output,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "source:working-tree-reviewed",
            key,
            revocations: &[],
        });
        assert_eq!(stale_output_promotion.decision, "deny");
        assert!(
            stale_output_promotion
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("output-path-ref mismatch"))
        );
    }

    fn promotion_receipt(input: PromotionInput<'_>) -> ReleasePromotionGateReceipt {
        release_promotion_gate_receipt_value(&ReleasePromotionGateInput {
            output_path: input.output_path,
            bundle_verify_value: input.bundle_verify_value,
            source_evidence: input.source_evidence,
            octet_evidence: "octet:clean",
            cairn_evidence: "cairn:strict-validate",
            signed_keys: std::slice::from_ref(input.key),
            signed_key_revocations: input.revocations,
            signed_trust_root: "release-root",
            signed_signer: Some("release-signer"),
            signed_key_ref: Some(&input.key.key_ref),
            signed_key_id: Some("release-key-1"),
        })
        .expect("promotion receipt")
    }

    fn assert_signed_denials(case: &NixCase, signed_members: &[IoValue], key: &SignedReceiptKey) {
        let missing_signed_member_verify = required_bundle_verify(case, &[], Some("release-signer"), Some(key));
        assert_eq!(missing_signed_member_verify.decision, "deny");
        let denied_bundle_promotion = promotion_receipt(PromotionInput {
            output_path: &case.output_root,
            bundle_verify_value: &missing_signed_member_verify.value,
            source_evidence: "source:working-tree-reviewed",
            key,
            revocations: &[],
        });
        assert_eq!(denied_bundle_promotion.decision, "deny");
        assert!(denied_bundle_promotion
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("current passing bundle verification")));
        let wrong_signer_verify = required_bundle_verify(case, signed_members, Some("wrong-signer"), None);
        assert_eq!(wrong_signer_verify.decision, "deny");
        assert!(wrong_signer_verify.diagnostics.iter().any(|diagnostic| diagnostic.contains("signer")));
        let missing_signed_verify = required_bundle_verify(case, &signed_members[..1], Some("release-signer"), None);
        assert_eq!(missing_signed_verify.decision, "deny");
        assert!(
            missing_signed_verify
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("missing signed member receipt"))
        );
        let wrong_purpose_members = signed_members_with_purpose(case, RELEASE_PROMOTION_SIGNING_PURPOSE);
        let wrong_purpose_verify = required_bundle_verify(
            case,
            &wrong_purpose_members,
            Some("release-signer"),
            Some(key),
        );
        assert_eq!(wrong_purpose_verify.decision, "deny");
        assert!(wrong_purpose_verify.diagnostics.iter().any(|diagnostic| diagnostic.contains("purpose")));
        let revocation = signed_revocation(key);
        let revoked_member_verify = required_bundle_verify_with_revocations(
            case,
            signed_members,
            Some("release-signer"),
            Some(key),
            std::slice::from_ref(&revocation),
        );
        assert_eq!(revoked_member_verify.decision, "deny");
        assert!(revoked_member_verify
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("revoked") || diagnostic.contains("unrevoked")));
        let stale_signed_member = sign_stale_member();
        let mut signed_with_stale = signed_members.to_vec();
        signed_with_stale.push(stale_signed_member);
        let stale_member_verify = required_bundle_verify(
            case,
            &signed_with_stale,
            Some("release-signer"),
            Some(key),
        );
        assert_eq!(stale_member_verify.decision, "deny");
        assert!(stale_member_verify
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("not a signable bundle member")));
    }

    fn required_bundle_verify(
        case: &NixCase,
        signed_member_values: &[IoValue],
        signed_signer: Option<&str>,
        key: Option<&SignedReceiptKey>,
    ) -> ReleaseEvidenceBundleVerifyReceipt {
        required_bundle_verify_with_revocations(case, signed_member_values, signed_signer, key, &[])
    }

    fn required_bundle_verify_with_revocations(
        case: &NixCase,
        signed_member_values: &[IoValue],
        signed_signer: Option<&str>,
        key: Option<&SignedReceiptKey>,
        revocations: &[SignedReceiptKeyRevocation],
    ) -> ReleaseEvidenceBundleVerifyReceipt {
        let empty_keys: &[SignedReceiptKey] = &[];
        let signed_keys = key.map(std::slice::from_ref).unwrap_or(empty_keys);
        verify_release_evidence_bundle(&ReleaseEvidenceBundleVerifyInput {
            output_path: &case.output_root,
            bundle_value: &case.bundle,
            signed_member_values,
            signed_purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
            signed_trust_root: "release-root",
            signed_key: "release-key",
            signed_keys,
            signed_key_revocations: revocations,
            signed_key_ref: key.map(|key| key.key_ref.as_str()),
            signed_key_id: key.map(|_| "release-key-1"),
            signed_signer,
            is_signed_members_required: true,
        })
        .expect("verify required release bundle")
    }

    fn signed_members_with_purpose(case: &NixCase, purpose: &str) -> Vec<IoValue> {
        [
            &case.run.report_value,
            case.run.release_gate_value.as_ref().expect("release gate"),
            case.run.replay_verify_value.as_ref().expect("replay verify"),
            case.run.replay_index_value.as_ref().expect("replay index"),
            &case.evidence,
            &case.receipt.value,
        ]
        .into_iter()
        .map(|receipt| {
            sign_receipt(&SignReceiptInput {
                receipt,
                signer: "release-signer",
                purpose,
                trust_root: "release-root",
                key: "release-key",
                parents: &[],
            })
            .expect("sign member with purpose")
        })
        .collect()
    }

    fn sign_stale_member() -> IoValue {
        let stale = crate::preserves_rail::record("stale-release-member-v1", vec![crate::preserves_rail::string("stale")]);
        sign_receipt(&SignReceiptInput {
            receipt: &stale,
            signer: "release-signer",
            purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
            trust_root: "release-root",
            key: "release-key",
            parents: &[],
        })
        .expect("sign stale member")
    }

    fn assert_ordered_release_workflow(
        case: &NixCase,
        signed_bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
        key: &SignedReceiptKey,
    ) {
        let promotion = promotion_receipt(PromotionInput {
            output_path: &case.output_root,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "source:working-tree-reviewed",
            key,
            revocations: &[],
        });
        let signed_promotion = sign_receipt(&SignReceiptInput {
            receipt: &promotion.value,
            signer: "release-signer",
            purpose: RELEASE_PROMOTION_SIGNING_PURPOSE,
            trust_root: "release-root",
            key: "release-key",
            parents: &[],
        })
        .expect("sign promotion");
        let signed_promotion_ref = crate::preserves_rail::canonical_hash(&signed_promotion).expect("signed promotion ref");
        let summary_ref = dogfood_ref("release-workflow-summary").expect("summary ref");
        let manifest_ref = dogfood_ref("release-workflow-manifest").expect("manifest ref");
        let export_verify_ref = dogfood_ref("release-workflow-export-verify").expect("export verify ref");
        let required_signed_member_refs = release_required_signed_member_refs(case);
        let pass = evaluate_release_workflow_state(&ReleaseWorkflowStateInput {
            required_stage: RELEASE_WORKFLOW_STAGE_ARCHIVE_VERIFY,
            dogfood_report_ref: Some(&case.parsed.report_ref),
            dogfood_report_decision: "pass",
            release_gate_ref: Some(&case.parsed.release_gate_ref),
            bundle_ref: Some(&case.parsed_bundle.bundle_ref),
            bundle_verify_ref: Some(&signed_bundle_verify.receipt_ref),
            bundle_verify_decision: "pass",
            signed_member_refs: &required_signed_member_refs,
            required_signed_member_refs: &required_signed_member_refs,
            promotion_ref: Some(&promotion.receipt_ref),
            promotion_decision: "pass",
            signed_promotion_ref: Some(&signed_promotion_ref),
            signed_promotion_subject_ref: Some(&promotion.receipt_ref),
            summary_ref: Some(&summary_ref),
            summary_decision: "pass",
            summary_promotion_ref: Some(&promotion.receipt_ref),
            export_manifest_ref: Some(&manifest_ref),
            export_manifest_summary_ref: Some(&summary_ref),
            export_verify_ref: Some(&export_verify_ref),
            export_verify_decision: "pass",
            export_verify_manifest_ref: Some(&manifest_ref),
        })
        .expect("release workflow pass");
        assert_eq!(pass.decision, "pass");
        assert!(pass
            .completed_stages
            .iter()
            .any(|stage| stage == RELEASE_WORKFLOW_STAGE_ARCHIVE_VERIFY));

        let premature = evaluate_release_workflow_state(&ReleaseWorkflowStateInput {
            required_stage: RELEASE_WORKFLOW_STAGE_PROMOTION,
            dogfood_report_ref: Some(&case.parsed.report_ref),
            dogfood_report_decision: "pass",
            release_gate_ref: Some(&case.parsed.release_gate_ref),
            bundle_ref: Some(&case.parsed_bundle.bundle_ref),
            bundle_verify_ref: Some(&signed_bundle_verify.receipt_ref),
            bundle_verify_decision: "deny",
            signed_member_refs: &required_signed_member_refs,
            required_signed_member_refs: &required_signed_member_refs,
            promotion_ref: Some(&promotion.receipt_ref),
            promotion_decision: "pass",
            signed_promotion_ref: None,
            signed_promotion_subject_ref: None,
            summary_ref: None,
            summary_decision: "deny",
            summary_promotion_ref: None,
            export_manifest_ref: None,
            export_manifest_summary_ref: None,
            export_verify_ref: None,
            export_verify_decision: "deny",
            export_verify_manifest_ref: None,
        })
        .expect("release workflow premature promotion");
        assert_eq!(premature.decision, "deny");
        assert!(premature
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("current passing bundle verification")));
    }

    fn release_required_signed_member_refs(case: &NixCase) -> Vec<String> {
        case.parsed_bundle
            .member_refs
            .iter()
            .filter_map(|(name, member_ref)| name.ends_with(".preserves").then_some(member_ref.clone()))
            .collect()
    }

    fn assert_missing_duplicate_and_tampered_bundle_members_deny(case: &NixCase) {
        let missing_member_refs = case
            .parsed_bundle
            .member_refs
            .iter()
            .filter(|(name, _)| name != "nix-dogfood-verify.preserves")
            .cloned()
            .collect::<Vec<_>>();
        let missing_bundle = bundle_with_member_refs(case, missing_member_refs);
        let missing_verify = unsigned_bundle_verify(&case.output_root, &missing_bundle);
        assert_eq!(missing_verify.decision, "deny");
        assert!(missing_verify
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("nix-dogfood-verify.preserves")));

        let mut duplicate_member_refs = case.parsed_bundle.member_refs.clone();
        duplicate_member_refs.push(case.parsed_bundle.member_refs[0].clone());
        let duplicate_bundle = bundle_with_member_refs(case, duplicate_member_refs);
        let duplicate_verify = unsigned_bundle_verify(&case.output_root, &duplicate_bundle);
        assert_eq!(duplicate_verify.decision, "deny");
        assert!(duplicate_verify.diagnostics.iter().any(|diagnostic| diagnostic.contains("duplicate")));

        let summary_path = case.output_root.join("dogfood-summary.txt");
        let original_summary = std::fs::read_to_string(&summary_path).expect("read summary");
        std::fs::write(&summary_path, "tampered summary\n").expect("tamper summary");
        let tampered_verify = unsigned_bundle_verify(&case.output_root, &case.bundle);
        std::fs::write(&summary_path, original_summary).expect("restore summary");
        assert_eq!(tampered_verify.decision, "deny");
        assert!(tampered_verify
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("summary") || diagnostic.contains("observation failed")));
    }

    fn bundle_with_member_refs(case: &NixCase, member_refs: Vec<(String, String)>) -> IoValue {
        crate::preserves_rail::record("release-evidence-bundle-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_EVIDENCE_BUNDLE_SCHEMA),
            crate::preserves_rail::record("output-path", vec![
                crate::preserves_rail::string(case.parsed_bundle.output_path.as_str()),
                crate::preserves_rail::string(&case.parsed_bundle.output_path_ref),
            ]),
            crate::preserves_rail::record("members", vec![file_refs_sequence(&member_refs)]),
            crate::preserves_rail::record("dogfood", vec![
                crate::preserves_rail::string(&case.parsed_bundle.report_ref),
                crate::preserves_rail::string(&case.parsed_bundle.release_gate_ref),
            ]),
            crate::preserves_rail::record("replay", vec![
                crate::preserves_rail::string(&case.parsed_bundle.replay_verify_ref),
                crate::preserves_rail::string(&case.parsed_bundle.replay_index_ref),
            ]),
            crate::preserves_rail::record("nix", vec![
                crate::preserves_rail::string(&case.parsed_bundle.nix_evidence_ref),
                crate::preserves_rail::string(&case.parsed_bundle.nix_verify_ref),
            ]),
            crate::preserves_rail::record("nextest", vec![
                crate::preserves_rail::string(&case.parsed_bundle.nextest_marker_ref),
                crate::preserves_rail::string(case.parsed_bundle.nextest_check_path.as_str()),
            ]),
            checks_value_from_pairs(&[
                ("dogfood-report-pass", "pass"),
                ("release-gate-pass", "pass"),
                ("replay-verify-bound", "pass"),
                ("replay-index-bound", "pass"),
                ("replay-index-is-evidence-only", "pass"),
                ("nix-verify-pass", "pass"),
                ("bundle-members-bound", "pass"),
                ("nextest-dependency-bound", "pass"),
                ("release-evidence-only", "pass"),
                ("no-text-oracle", "pass"),
            ]),
        ])
    }

    fn assert_stale_bundle_denies(case: &NixCase) {
        let stale_bundle_ref = dogfood_ref("stale-bundle-summary").expect("stale bundle ref");
        let stale_bundle_text = to_text(&case.bundle)
            .expect("bundle text")
            .replace(&case.parsed_bundle.summary_ref, &stale_bundle_ref);
        let stale_bundle = crate::preserves_rail::parse_text(&stale_bundle_text).expect("stale bundle parse");
        let stale_bundle_verify = unsigned_bundle_verify(&case.output_root, &stale_bundle);
        assert_eq!(stale_bundle_verify.decision, "deny");
        assert!(stale_bundle_verify.diagnostics.iter().any(|diagnostic| diagnostic.contains("summary-ref mismatch")));
    }

    fn assert_stale_evidence_denies(case: &NixCase) {
        let stale_ref = dogfood_ref("stale-summary").expect("stale ref");
        let stale_text = to_text(&case.evidence).expect("evidence text").replace(&case.parsed.summary_ref, &stale_ref);
        let stale_evidence = crate::preserves_rail::parse_text(&stale_text).expect("stale evidence parse");
        let stale_receipt = verify_nix_dogfood_evidence(&NixDogfoodVerifyInput {
            output_path: &case.output_root,
            evidence_value: &stale_evidence,
        })
        .expect("verify stale evidence");
        assert_eq!(stale_receipt.decision, "deny");
        assert!(stale_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("summary-ref mismatch")));
    }

    #[test]
    fn missing_receipt_and_non_replayable_mandatory_steps_deny_report() {
        let report = report_with_mandatory_gaps();
        let parsed = parse_dogfood_report(&report).expect("parse report");
        assert_eq!(parsed.decision, "deny");
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("lacks canonical receipt")));
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("non-release replay status")));
        assert_gate_rejects(&report);
    }

    fn report_with_mandatory_gaps() -> IoValue {
        let request_ref = dogfood_ref("request").expect("request ref");
        let missing_step = mandatory_step("install-artifact", &request_ref, None, "deterministic");
        let live_receipt = dogfood_ref("live-receipt").expect("live receipt");
        let live_step = mandatory_step("live-diagnostic", &request_ref, Some(&live_receipt), "non-replayable");
        let policy_refs = vec![dogfood_ref("policy").expect("policy")];
        let capability_refs = vec![dogfood_ref("capability").expect("capability")];
        let resource_refs = vec![dogfood_ref("resource").expect("resource")];
        let workflow = operator_workflow_value(&OperatorWorkflowInput {
            workflow_id: LOCAL_NODE_WORKFLOW_ID,
            steps: &[missing_step, live_step],
            policy_refs: &policy_refs,
            capability_refs: &capability_refs,
            resource_refs: &resource_refs,
            replay_profile: "recorded",
        })
        .expect("workflow");
        let checkpoint = operator_checkpoint_value(&OperatorCheckpointInput {
            workflow_id: LOCAL_NODE_WORKFLOW_ID,
            sequence: 0,
            step_ref: &dogfood_ref("step").expect("step"),
            request_ref: Some(&request_ref),
            receipt_ref: None,
            result_ref: None,
            state_root_ref: &dogfood_ref("state").expect("state"),
        })
        .expect("checkpoint");
        dogfood_report_value(&DogfoodReportInput {
            workflow_value: &workflow,
            checkpoint_values: &[checkpoint],
            gate_receipt_refs: &[dogfood_ref("gate").expect("gate")],
            repro_bundle_refs: &[dogfood_ref("repro").expect("repro")],
            final_state_ref: &dogfood_ref("final-state").expect("final state"),
            diagnostics: &[],
        })
        .expect("report")
    }

    fn mandatory_step(name: &str, request_ref: &str, receipt_ref: Option<&str>, replay_status: &str) -> IoValue {
        operator_step_value(&OperatorStepInput {
            name,
            request_ref: Some(request_ref),
            receipt_ref,
            decision: "pass",
            replay_status,
            mandatory: true,
            artifact_refs: &[],
            diagnostics: &[],
        })
        .expect("mandatory step")
    }

    fn assert_gate_rejects(report: &IoValue) {
        assert!(
            release_gate_receipt_value(&ReleaseGateInput {
                report_value: report,
                node_startup_ref: &dogfood_ref("startup").expect("startup"),
                node_shutdown_ref: &dogfood_ref("shutdown").expect("shutdown"),
                harness_gate_refs: &[dogfood_ref("harness-gate").expect("harness gate")],
                catalog_query_refs: &[dogfood_ref("catalog").expect("catalog")],
                repro_verify_refs: &[dogfood_ref("verify").expect("verify")],
                replay_index_refs: &[dogfood_ref("replay-index").expect("replay index")],
                gc_refs: &[dogfood_ref("retention-gc").expect("retention gc")],
                validation_command_refs: &[dogfood_ref("validation").expect("validation")],
            })
            .is_err()
        );
    }
