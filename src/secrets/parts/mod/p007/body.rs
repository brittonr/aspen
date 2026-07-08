
#[cfg(test)]
mod tests {
    type TestCase = hegel::TestCase;

    use super::*;

    fn temp_dir(label: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).expect("remove stale temp root");
        }
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    #[test]
    fn canonical_confidentiality_records_roundtrip() {
        let run = run_secrets_fixture().expect("fixture");
        assert_eq!(run.reveal_denied.decision, "deny");
        assert_eq!(run.reveal_pass.decision, "pass");
        assert_eq!(run.decrypt_denied.decision, "deny");
        assert_eq!(run.decrypt_pass.decision, "pass");
        assert_eq!(run.replay.decision, "pass");
        assert_eq!(run.cleanup.decision, "pass");
        assert_eq!(run.cleanup.retention_refs.len(), 1);
        assert!(run.reveal_denied.plaintext_ref.is_none());
        assert!(fixture_report_summary(&run.value).expect("fixture summary").contains("plaintext=redacted"));
        assert_eq!(parse_secret_ref(&run.secret.value).expect("secret").secret_ref, run.secret.secret_ref);
        assert_eq!(
            parse_encrypted_ref(&run.encrypted.value).expect("encrypted").encrypted_ref,
            run.encrypted.encrypted_ref
        );
        assert_eq!(parse_redaction_marker(&run.marker.value).expect("marker").marker_ref, run.marker.marker_ref);
    }

    #[test]
    fn secret_cleanup_requires_actual_retention_receipt_evidence() {
        let denied_value = secret_cleanup_receipt_value(&SecretCleanupInput {
            secret_ref: fixture_ref("secret"),
            revocation_ref: fixture_ref("revocation"),
            tombstone_ref: fixture_ref("tombstone"),
            retention_refs: Vec::new(),
            retention_receipts: Vec::new(),
            retention_tombstones: Vec::new(),
            authority_refs: vec![fixture_ref("authority")],
            policy_refs: vec![fixture_ref("policy")],
        })
        .expect("cleanup receipt");
        let denied = parse_secret_cleanup_receipt(&denied_value).expect("parse cleanup deny");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("retention receipt")));
    }

    #[test]
    fn catalog_and_mcp_render_redaction_markers_without_plaintext() {
        let root = temp_dir("secrets-catalog");
        let registry = root.join("registry");
        let artifact = crate::artifacts::install_artifact(&registry, &crate::artifacts::ArtifactInstallInput {
            kind: "doc".to_string(),
            payload: parse_text("<doc <credential \"do-not-render\">>").expect("secret doc"),
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: Vec::new(),
            evidence_refs: Vec::new(),
            installer_ref: fixture_ref("catalog-installer"),
            capability_refs: vec![fixture_ref("catalog-capability")],
        })
        .expect("install");
        let viewed = crate::catalog::view(&registry, None, &crate::catalog::ViewInput {
            reference: artifact.artifact_ref.clone(),
            include_payload: true,
            redacted: true,
            visibility: crate::catalog::VisibilityInput::default(),
        })
        .expect("view");
        let text = to_text(&viewed.value).expect("view text");
        assert!(text.contains("redaction-marker-v1"));
        assert!(!text.contains("do-not-render"));
        let request = crate::catalog_mcp::mcp_request_value("catalog.view", vec![
            record("reference", vec![string(&artifact.artifact_ref)]),
            record("payload", vec![bool_value(true)]),
        ])
        .expect("mcp request");
        let response = crate::catalog_mcp::call(&registry, None, &request).expect("mcp call");
        let response_text = to_text(&response.response_value).expect("response");
        assert!(response_text.contains("redaction-marker-v1"));
        assert!(!response_text.contains("do-not-render"));
    }

    #[test]
    fn redaction_reason_uses_structural_markers_not_rendered_strings() {
        let structural = record("wrapper", vec![record("credential", vec![string("token")])]);
        assert_eq!(first_redaction_reason(&structural).expect("structural reason"), "credential");

        let inert = record("wrapper", vec![string("<credential \"diagnostic-looking\">")]);
        assert_eq!(first_redaction_reason(&inert).expect("inert reason"), "secret");
    }

    #[test]
    fn reveal_and_decrypt_require_authority_not_ciphertext_possession() {
        let run = run_secrets_fixture().expect("fixture");
        assert_eq!(run.reveal_denied.decision, "deny");
        assert!(run.reveal_denied.diagnostics.join(";").contains("authority"));
        assert_eq!(run.decrypt_denied.decision, "deny");
        assert!(run.decrypt_denied.diagnostics.join(";").contains("encrypted refs alone are not authority"));
        assert_eq!(run.decrypt_pass.decision, "pass");
        assert_eq!(run.decrypt_pass.commitment_ref, run.encrypted.commitment_ref);
    }

    #[test]
    fn exact_reveal_decrypt_binding_core_denies_mismatches() {
        let run = run_secrets_fixture().expect("fixture");
        let exact = evaluate_secret_access_binding(SecretAccessBindingInput {
            secret: &run.secret,
            encrypted: &run.encrypted,
            reveal: Some(&run.reveal_pass),
            decrypt: Some(&run.decrypt_pass),
            expected_plaintext_ref: run.reveal_pass.plaintext_ref.as_deref(),
        })
        .expect("exact access binding");
        assert_eq!(exact.decision, "pass");
        assert!(exact.plaintext_authorized);

        let wrong_encrypted_value = encrypted_ref_value(&EncryptedRefInput {
            ciphertext_ref: fixture_ref("wrong-ciphertext"),
            commitment_ref: run.encrypted.commitment_ref.clone(),
            encryption_ref: run.encrypted.encryption_ref.clone(),
            schema_ref: run.encrypted.schema_ref.clone(),
            policy_refs: run.encrypted.policy_refs.clone(),
            evidence_refs: run.encrypted.evidence_refs.clone(),
        })
        .expect("wrong encrypted value");
        let wrong_encrypted = parse_encrypted_ref(&wrong_encrypted_value).expect("wrong encrypted");
        let wrong_encrypted_decision = evaluate_secret_access_binding(SecretAccessBindingInput {
            secret: &run.secret,
            encrypted: &wrong_encrypted,
            reveal: Some(&run.reveal_pass),
            decrypt: Some(&run.decrypt_pass),
            expected_plaintext_ref: run.reveal_pass.plaintext_ref.as_deref(),
        })
        .expect("wrong encrypted binding");
        assert_eq!(wrong_encrypted_decision.decision, "deny");
        assert!(!wrong_encrypted_decision.plaintext_authorized);
        assert!(wrong_encrypted_decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == SECRET_ACCESS_DECRYPT_ENCRYPTED_MISMATCH));

        let wrong_commitment_value = encrypted_ref_value(&EncryptedRefInput {
            ciphertext_ref: fixture_ref("commitment-ciphertext"),
            commitment_ref: fixture_ref("wrong-commitment"),
            encryption_ref: run.encrypted.encryption_ref.clone(),
            schema_ref: run.encrypted.schema_ref.clone(),
            policy_refs: run.encrypted.policy_refs.clone(),
            evidence_refs: run.encrypted.evidence_refs.clone(),
        })
        .expect("wrong commitment value");
        let wrong_commitment = parse_encrypted_ref(&wrong_commitment_value).expect("wrong commitment");
        let wrong_commitment_decision = evaluate_secret_access_binding(SecretAccessBindingInput {
            secret: &run.secret,
            encrypted: &wrong_commitment,
            reveal: Some(&run.reveal_pass),
            decrypt: Some(&run.decrypt_pass),
            expected_plaintext_ref: run.reveal_pass.plaintext_ref.as_deref(),
        })
        .expect("wrong commitment binding");
        assert_eq!(wrong_commitment_decision.decision, "deny");
        assert!(wrong_commitment_decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == SECRET_ACCESS_DECRYPT_COMMITMENT_MISMATCH));

        let stale_reveal = evaluate_secret_access_binding(SecretAccessBindingInput {
            secret: &run.secret,
            encrypted: &run.encrypted,
            reveal: Some(&run.reveal_denied),
            decrypt: Some(&run.decrypt_pass),
            expected_plaintext_ref: run.reveal_pass.plaintext_ref.as_deref(),
        })
        .expect("stale reveal binding");
        assert_eq!(stale_reveal.decision, "deny");
        assert!(stale_reveal
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == SECRET_ACCESS_REVEAL_FAILED));
    }

    #[test]
    fn redaction_gate_core_rejects_diagnostic_profiles() {
        let run = run_secrets_fixture().expect("fixture");
        let gate = evaluate_secret_redaction_gate(SecretRedactionGateInput {
            transform: &run.transform,
            private_bundle: Some(&run.private_bundle),
            required_source_ref: &run.transform.source_ref,
            required_output_ref: &run.transform.output_ref,
            requires_gate_preserving: true,
        })
        .expect("gate preserving redaction");
        assert_eq!(gate.decision, "pass");
        assert!(gate.gate_preserving);

        let diagnostic_transform_value = redaction_transform_receipt_value(&RedactionTransformInput {
            source_ref: run.transform.source_ref.clone(),
            output_ref: run.transform.output_ref.clone(),
            policy_refs: vec![fixture_ref("diagnostic-policy")],
            profile_ref: fixture_ref("diagnostic-redaction-profile"),
            marker_refs: run.transform.marker_refs.clone(),
            is_gate_preserving: false,
            diagnostics: Vec::new(),
        })
        .expect("diagnostic transform");
        let diagnostic_transform = parse_redaction_transform_receipt(&diagnostic_transform_value)
            .expect("parse diagnostic transform");
        let denied = evaluate_secret_redaction_gate(SecretRedactionGateInput {
            transform: &diagnostic_transform,
            private_bundle: None,
            required_source_ref: &run.transform.source_ref,
            required_output_ref: &run.transform.output_ref,
            requires_gate_preserving: true,
        })
        .expect("diagnostic redaction gate");
        assert_eq!(denied.decision, "deny");
        assert!(!denied.gate_preserving);
        assert!(denied
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == SECRET_REDACTION_PROFILE_DIAGNOSTIC_ONLY));
    }

    #[test]
    fn cleanup_admission_core_requires_bound_retention_evidence() {
        let core = fixture_core().expect("fixture core");
        let retention = fixture_retention(&core).expect("fixture retention");
        let tombstone = retention.tombstone.as_ref().expect("retention tombstone");
        let admitted_input = SecretCleanupInput {
            secret_ref: core.secret.secret_ref.clone(),
            revocation_ref: fixture_ref("secret-revocation"),
            tombstone_ref: tombstone.tombstone_ref.clone(),
            retention_refs: vec![retention.receipt.receipt_ref.clone()],
            retention_receipts: vec![retention.receipt.value.clone()],
            retention_tombstones: vec![tombstone.value.clone()],
            authority_refs: core.refs.authority.clone(),
            policy_refs: core.refs.policy.clone(),
        };
        let admitted = evaluate_secret_cleanup_admission(&admitted_input).expect("cleanup admission");
        assert_eq!(admitted.decision, "pass");
        assert!(admitted.cleanup_authorized);

        let denied_input = SecretCleanupInput {
            retention_refs: Vec::new(),
            retention_receipts: Vec::new(),
            retention_tombstones: Vec::new(),
            ..admitted_input
        };
        let denied = evaluate_secret_cleanup_admission(&denied_input).expect("cleanup denial");
        assert_eq!(denied.decision, "deny");
        assert!(!denied.cleanup_authorized);
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("retention receipt")));
    }

    #[test]
    fn commitment_replay_and_private_bundle_profiles_are_receipted() {
        let run = run_secrets_fixture().expect("fixture");
        assert_eq!(run.private_bundle.transform_receipt_ref, run.transform.receipt_ref);
        assert!(run.private_bundle.is_gate_preserving);
        let plaintext_required = commitment_replay_receipt_value(&CommitmentReplayInput {
            expected_commitment_ref: run.secret.commitment_ref.clone(),
            actual_commitment_ref: run.encrypted.commitment_ref.clone(),
            reveal_receipt_ref: None,
            is_plaintext_required: true,
        })
        .expect("plaintext required replay");
        let denied = parse_commitment_replay_receipt(&plaintext_required).expect("parse denied replay");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.join(";").contains("plaintext-required"));
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_confidentiality_artifacts() {
        let run = run_secrets_fixture().expect("fixture");
        assert_eq!(crate::ledger::artifact_kind(&run.secret.value), "secret-ref");
        assert_eq!(crate::ledger::artifact_kind(&run.encrypted.value), "encrypted-ref");
        assert_eq!(crate::ledger::artifact_kind(&run.marker.value), "redaction-marker");
        assert_eq!(crate::ledger::artifact_kind(&run.transform.value), "redaction-transform-receipt");
        let root = temp_dir("secrets-ledger");
        let registry = root.join("registry");
        let ledger_root = root.join("ledger");
        std::fs::create_dir_all(&registry).expect("registry");
        crate::ledger::import_artifact(&ledger_root, &run.secret.value).expect("import");
        let list = crate::catalog::list(&registry, Some(&ledger_root), &crate::catalog::ListInput {
            kind: Some("secret-ref".to_string()),
            visibility: crate::catalog::VisibilityInput::default(),
        })
        .expect("list");
        assert_eq!(list.items.len(), 1);
        let request = crate::catalog_mcp::mcp_request_value("catalog.view", vec![record("reference", vec![string(
            &run.secret.secret_ref,
        )])])
        .expect("mcp request");
        let response = crate::catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("mcp call");
        assert_eq!(response.decision, "pass");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_redaction_stability_no_plaintext_and_authority_monotonicity(tc: TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(1_000_000));
        let payload = record("secret", vec![string(format!("payload-{salt}"))]);
        let first = redacted_view(&payload, None).expect("first redaction");
        let second = redacted_view(&payload, None).expect("second redaction");
        assert_eq!(first.value, second.value);
        let redacted = to_text(&first.value).expect("redacted text");
        assert!(!redacted.contains(&format!("payload-{salt}")));
        let secret_ref = fixture_ref(&format!("secret-{salt}"));
        let commitment_ref = fixture_ref(&format!("commitment-{salt}"));
        let denied = parse_reveal_receipt(
            &reveal_receipt_value(&RevealReceiptInput {
                secret_ref: secret_ref.clone(),
                encrypted_ref: None,
                requester_ref: fixture_ref("requester"),
                purpose: "debug".to_string(),
                plaintext_ref: Some(fixture_ref("plain")),
                commitment_ref: commitment_ref.clone(),
                authority_refs: Vec::new(),
                policy_refs: vec![fixture_ref("policy")],
                resource_refs: vec![fixture_ref("resource")],
                effect_handle_refs: vec![fixture_ref("effect")],
                revocation_refs: Vec::new(),
            })
            .expect("deny reveal value"),
        )
        .expect("deny reveal");
        let admitted = parse_reveal_receipt(
            &reveal_receipt_value(&RevealReceiptInput {
                secret_ref,
                encrypted_ref: None,
                requester_ref: fixture_ref("requester"),
                purpose: "debug".to_string(),
                plaintext_ref: Some(fixture_ref("plain")),
                commitment_ref,
                authority_refs: vec![fixture_ref("authority")],
                policy_refs: vec![fixture_ref("policy")],
                resource_refs: vec![fixture_ref("resource")],
                effect_handle_refs: vec![fixture_ref("effect")],
                revocation_refs: Vec::new(),
            })
            .expect("admit reveal value"),
        )
        .expect("admit reveal");
        assert_eq!(denied.decision, "deny");
        assert_eq!(admitted.decision, "pass");
    }
}
