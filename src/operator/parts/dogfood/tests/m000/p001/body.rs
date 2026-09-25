
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
