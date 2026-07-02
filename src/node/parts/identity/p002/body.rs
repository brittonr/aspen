
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
    fn restart_with_same_data_dir_preserves_endpoint_id_without_secret_in_receipts() {
        let dir = temp_dir("node-identity-restart");
        let config = Config::new("node-a", &dir);
        let first = resolve(&config).expect("first resolve");
        let first_identity = first.identity.expect("first identity");
        let second = resolve(&config).expect("second resolve");
        let second_identity = second.identity.expect("second identity");
        assert_eq!(first_identity.endpoint_id, second_identity.endpoint_id);
        assert_eq!(second_identity.key_source_class, "persisted-file");
        let secret = fs::read_to_string(dir.join(SECRET_FILE)).expect("read secret");
        let first_receipt_text = crate::preserves_rail::to_text(&first.receipt_value).expect("receipt text");
        let second_receipt_text = crate::preserves_rail::to_text(&second.receipt_value).expect("receipt text");
        assert!(!first_receipt_text.contains(secret.trim()));
        assert!(!second_receipt_text.contains(secret.trim()));
    }

    #[test]
    fn drift_is_denied_unless_rotation_policy_is_admitted() {
        let dir = temp_dir("node-identity-drift");
        let config = Config::new("node-a", &dir);
        let first = resolve(&config).expect("first resolve");
        let first_endpoint = first.identity.expect("identity").endpoint_id;
        fs::write(dir.join(SECRET_FILE), "replacement-secret\n").expect("replace secret");
        let drift = resolve(&config).expect("drift receipt");
        assert!(drift.identity.is_none());
        assert!(crate::preserves_rail::to_text(&drift.receipt_value).expect("drift text").contains("drift-detected"));

        let mut rotation = config.clone();
        rotation.allow_rotation = true;
        let rotated = resolve(&rotation).expect("rotation allowed");
        let rotated_endpoint = rotated.identity.expect("rotated identity").endpoint_id;
        assert_ne!(first_endpoint, rotated_endpoint);
    }

    #[test]
    fn explicit_key_and_deny_if_unavailable_follow_resolution_order() {
        let explicit_dir = temp_dir("node-identity-explicit");
        let mut explicit = Config::new("node-explicit", &explicit_dir);
        explicit.explicit_key = Some("deployment-secret".to_string());
        explicit.allow_generate = false;
        let resolved = resolve(&explicit).expect("explicit resolve");
        assert_eq!(resolved.identity.expect("explicit identity").key_source_class, "explicit-key");
        assert!(
            !crate::preserves_rail::to_text(&resolved.receipt_value)
                .expect("receipt text")
                .contains("deployment-secret")
        );

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
