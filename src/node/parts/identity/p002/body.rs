
fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_resolution_core_selects_sources_and_denials_without_io() {
        let explicit = resolve_iroh_secret_source(&IrohSecretSourceFacts {
            explicit_key_present: true,
            managed_secret_present: true,
            managed_secret_required: true,
            persisted_file_present: true,
            persisted_file_permission: IrohSecretPermissionStatus::Unsafe,
            generation_allowed: true,
        });
        assert_eq!(explicit.kind, IrohSecretSourceDecisionKind::LoadExplicit);
        assert_eq!(explicit.key_source_class, KEY_SOURCE_EXPLICIT);

        let required_backend_denial = resolve_iroh_secret_source(&IrohSecretSourceFacts {
            explicit_key_present: false,
            managed_secret_present: false,
            managed_secret_required: true,
            persisted_file_present: true,
            persisted_file_permission: IrohSecretPermissionStatus::Restricted,
            generation_allowed: true,
        });
        assert_eq!(required_backend_denial.kind, IrohSecretSourceDecisionKind::Deny);
        assert!(required_backend_denial.diagnostic.contains("required"));

        let unsafe_file_denial = resolve_iroh_secret_source(&IrohSecretSourceFacts {
            explicit_key_present: false,
            managed_secret_present: false,
            managed_secret_required: false,
            persisted_file_present: true,
            persisted_file_permission: IrohSecretPermissionStatus::Unsafe,
            generation_allowed: true,
        });
        assert_eq!(unsafe_file_denial.kind, IrohSecretSourceDecisionKind::Deny);
        assert_eq!(unsafe_file_denial.permission_status, IrohSecretPermissionStatus::Unsafe);

        let first_boot = resolve_iroh_secret_source(&IrohSecretSourceFacts {
            explicit_key_present: false,
            managed_secret_present: false,
            managed_secret_required: false,
            persisted_file_present: false,
            persisted_file_permission: IrohSecretPermissionStatus::NotPresent,
            generation_allowed: true,
        });
        assert_eq!(first_boot.kind, IrohSecretSourceDecisionKind::GenerateAndPersist);
    }

    #[test]
    fn restart_with_same_data_dir_preserves_endpoint_id_without_secret_in_receipts() {
        let dir = temp_dir("node-identity-restart");
        let config = Config::new("node-a", &dir);
        let first = resolve(&config).expect("first resolve");
        let first_identity = first.identity.expect("first identity");
        let second = resolve(&config).expect("second resolve");
        let second_identity = second.identity.expect("second identity");
        assert_eq!(first_identity.endpoint_id, second_identity.endpoint_id);
        assert_eq!(second_identity.key_source_class, KEY_SOURCE_PERSISTED_FILE);
        let secret = fs::read_to_string(dir.join(SECRET_FILE)).expect("read secret");
        let first_receipt_text = crate::preserves_rail::to_text(&first.receipt_value).expect("receipt text");
        let second_receipt_text = crate::preserves_rail::to_text(&second.receipt_value).expect("receipt text");
        assert!(!first_receipt_text.contains(secret.trim()));
        assert!(!second_receipt_text.contains(secret.trim()));
        assert!(second_receipt_text.contains("restricted-owner-only"));
    }

    #[test]
    fn managed_backend_loads_and_required_backend_denies_without_fallback() {
        let backend_dir = temp_dir("node-identity-backend");
        let mut backend = Config::new("node-backend", &backend_dir);
        backend.secret_backend_key = Some("managed-backend-secret".to_string());
        backend.require_secret_backend = true;
        backend.allow_generate = true;
        let resolved = resolve(&backend).expect("managed backend resolve");
        let identity = resolved.identity.expect("managed backend identity");
        assert_eq!(identity.key_source_class, KEY_SOURCE_MANAGED_BACKEND);
        assert!(crate::preserves_rail::to_text(&resolved.receipt_value)
            .expect("receipt text")
            .contains(KEY_SOURCE_MANAGED_BACKEND));

        let required_dir = temp_dir("node-identity-backend-required");
        let mut required = Config::new("node-required", required_dir);
        required.require_secret_backend = true;
        required.allow_generate = true;
        let denied = resolve(&required).expect("required backend denial receipt");
        assert!(denied.identity.is_none());
        assert!(crate::preserves_rail::to_text(&denied.receipt_value)
            .expect("denial text")
            .contains("managed-backend-required"));
    }

    #[test]
    fn explicit_key_precedes_backends_and_malformed_keys_deny_before_startup() {
        let explicit_dir = temp_dir("node-identity-explicit");
        let mut explicit = Config::new("node-explicit", &explicit_dir);
        explicit.explicit_key = Some("deployment-secret".to_string());
        explicit.secret_backend_key = Some("backend-secret".to_string());
        explicit.allow_generate = false;
        let resolved = resolve(&explicit).expect("explicit resolve");
        assert_eq!(resolved.identity.expect("explicit identity").key_source_class, KEY_SOURCE_EXPLICIT);
        assert!(
            !crate::preserves_rail::to_text(&resolved.receipt_value)
                .expect("receipt text")
                .contains("deployment-secret")
        );

        let malformed_dir = temp_dir("node-identity-malformed");
        let mut malformed = Config::new("node-malformed", malformed_dir);
        malformed.explicit_key = Some("bad\nsecret".to_string());
        let error = resolve(&malformed).expect_err("malformed explicit key denies");
        assert!(error.to_string().contains("control characters"));
    }

    #[test]
    fn deny_if_unavailable_fails_closed_before_endpoint_advertising() {
        let denied_dir = temp_dir("node-identity-denied");
        let mut denied = Config::new("node-denied", denied_dir);
        denied.allow_generate = false;
        let denied = resolve(&denied).expect("denial receipt");
        assert!(denied.identity.is_none());
        assert!(
            crate::preserves_rail::to_text(&denied.receipt_value)
                .expect("denial text")
                .contains("deny-if-unavailable")
        );
    }

    #[cfg(unix)]
    #[test]
    fn unsafe_persisted_file_permissions_deny_before_secret_load() {
        use std::os::unix::fs::PermissionsExt;

        const GROUP_READABLE_SECRET_FILE_MODE: u32 = 0o644;

        let dir = temp_dir("node-identity-unsafe-permission");
        let config = Config::new("node-unsafe", &dir);
        resolve(&config).expect("first boot writes secret");
        let secret_path = dir.join(SECRET_FILE);
        let mut permissions = fs::metadata(&secret_path).expect("secret metadata").permissions();
        permissions.set_mode(GROUP_READABLE_SECRET_FILE_MODE);
        std::fs::set_permissions(&secret_path, permissions).expect("set unsafe permissions");

        let denied = resolve(&config).expect("unsafe permission denial receipt");
        assert!(denied.identity.is_none());
        let receipt_text = crate::preserves_rail::to_text(&denied.receipt_value).expect("denial text");
        assert!(receipt_text.contains("unsafe-persisted-permissions"));
        assert!(receipt_text.contains("unsafe-shared"));
    }

    #[test]
    fn drift_requires_matching_rotation_receipt() {
        let dir = temp_dir("node-identity-drift");
        let config = Config::new("node-a", &dir);
        let first = resolve(&config).expect("first resolve");
        let first_endpoint = first.identity.expect("identity").endpoint_id;
        fs::write(dir.join(SECRET_FILE), "replacement-secret\n").expect("replace secret");
        let replacement_endpoint = derive_endpoint_material("replacement-secret")
            .expect("replacement material")
            .endpoint_id;

        let drift = resolve(&config).expect("drift receipt");
        assert!(drift.identity.is_none());
        assert!(crate::preserves_rail::to_text(&drift.receipt_value)
            .expect("drift text")
            .contains("rotation policy is required"));

        let mut stale_rotation = config.clone();
        stale_rotation.allow_rotation = true;
        stale_rotation.rotation_receipt_ref = Some(admitted_rotation_receipt_ref(
            &first_endpoint,
            &derive_endpoint_material("other-secret").expect("other material").endpoint_id,
            &stale_rotation.policy_refs,
        )
        .expect("stale rotation ref"));
        let stale = resolve(&stale_rotation).expect("stale rotation denial receipt");
        assert!(stale.identity.is_none());
        assert!(crate::preserves_rail::to_text(&stale.receipt_value)
            .expect("stale text")
            .contains("stale or mismatched"));

        let mut rotation = config.clone();
        rotation.allow_rotation = true;
        rotation.rotation_receipt_ref = Some(
            admitted_rotation_receipt_ref(&first_endpoint, &replacement_endpoint, &rotation.policy_refs)
                .expect("rotation ref"),
        );
        let rotated = resolve(&rotation).expect("rotation allowed");
        let rotated_endpoint = rotated.identity.expect("rotated identity").endpoint_id;
        assert_ne!(first_endpoint, rotated_endpoint);
        assert_eq!(replacement_endpoint, rotated_endpoint);
    }

    #[test]
    fn bootstrap_and_startup_evidence_bind_identity_without_authority() {
        let dir = temp_dir("node-identity-bootstrap");
        let resolved = resolve(&Config::new("node-a", &dir)).expect("resolve");
        let identity = resolved.identity.expect("identity");
        let handshake = bootstrap_handshake_value(&identity, "peer:b", &[]).expect("handshake");
        let parsed = parse_bootstrap_handshake(&handshake).expect("parse handshake");
        assert_eq!(parsed.identity_ref, identity.identity_ref);
        assert_eq!(parsed.endpoint_id, identity.endpoint_id);
        assert!(
            crate::preserves_rail::to_text(&handshake)
                .expect("handshake text")
                .contains("identity-grants-no-capabilities")
        );
        let startup = startup_evidence_value(&identity.identity_ref, &resolved.receipt_ref).expect("startup");
        assert!(crate::preserves_rail::to_text(&startup).expect("startup text").contains("private-key-not-required"));
        let denial = crate::peer_bootstrap::record_as_authority_denial(&identity.identity_ref, "node-control")
            .expect("endpoint identity authority denial");
        assert_eq!(denial.decision, "deny");
        assert!(denial.diagnostics[0].contains("not authority"));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_explicit_resolution_is_deterministic_and_receipts_redact_secret(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let secret_suffix = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let secret = format!("explicit-secret-{salt}-{secret_suffix}");
        let mut first_config = Config::new(format!("node-{salt}"), temp_dir("node-identity-hegel-a"));
        first_config.explicit_key = Some(secret.clone());
        first_config.allow_generate = false;
        let mut second_config = Config::new(format!("node-{salt}"), temp_dir("node-identity-hegel-b"));
        second_config.explicit_key = Some(secret.clone());
        second_config.allow_generate = false;
        let first = resolve(&first_config).expect("first explicit");
        let second = resolve(&second_config).expect("second explicit");
        assert_eq!(
            first.identity.as_ref().expect("first identity").endpoint_id,
            second.identity.as_ref().expect("second identity").endpoint_id
        );
        assert!(!crate::preserves_rail::to_text(&first.receipt_value).expect("receipt text").contains(&secret));
        assert!(!crate::preserves_rail::to_text(&second.receipt_value).expect("receipt text").contains(&secret));
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
