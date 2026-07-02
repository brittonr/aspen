
impl<T, S> PushLimited<T> for S
where S: crate::bounded::VecSink<T>
{
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()> {
        ensure_count_at_most(self.item_count().saturating_add(1), maximum, label)?;
        self.push_item(value);
        Ok(())
    }
}

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
