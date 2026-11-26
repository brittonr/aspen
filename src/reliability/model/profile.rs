//! Shell for register profile admission: the pure rules live in `molten-reliability-model-core`;
//! this module binds the canonical Preserves identity port.

pub use molten_reliability_model_core::profile::Admitted;
pub use molten_reliability_model_core::profile::EvidenceBounds;
pub use molten_reliability_model_core::profile::Issue;
pub use molten_reliability_model_core::profile::RecoveryBounds;
pub use molten_reliability_model_core::profile::Register;
pub use molten_reliability_model_core::profile::is_ref;

/// Admits `profile` and binds its canonical Preserves reference.
pub fn admit(profile: &Register) -> Result<Admitted, Issue> {
    molten_reliability_model_core::profile::admit(profile, canonical_ref)
}

/// Canonical tagged Preserves tree; JSON rendering and Rust layout never enter the identity.
pub fn canonical_ref<T: serde::Serialize>(value: &T) -> Result<String, Issue> {
    let tree = serde_json::to_value(value).map_err(|_| Issue::Encoding)?;
    crate::live_reliability::checker::canonical_ref(&tree).map_err(|_| Issue::Encoding)
}

#[cfg(test)]
mod tests {
    // r[verify molten.consensus.live_register_profile]
    #[test]
    fn shell_admission_binds_the_canonical_preserves_reference() {
        let reference = format!("blake3:{}", "a".repeat(64));
        let profile = super::Register {
            schema: "molten.live-register-profile.v1".into(),
            group: "g".into(),
            service: "registry".into(),
            keys: vec!["k".into()],
            initial_value: None,
            values: vec![-1, 0, 7],
            generator_ref: reference.clone(),
            adapter_ref: reference.clone(),
            model_ref: reference.clone(),
            outcome_ref: reference,
            seed: 0,
            read_weight: 1,
            write_weight: 1,
            clients: vec!["a".into(), "b".into()],
            endpoints: vec!["a:7180".into(), "b:7180".into()],
            concurrency: 2,
            operations: 12,
            control_operations: 3,
            consistency_model: "register".into(),
            recovery: super::RecoveryBounds {
                timeout_ms: 100,
                stable_membership_observations: 2,
                require_all_endpoint_reads: true,
            },
            evidence: super::EvidenceBounds {
                max_attempts_per_operation: 3,
                max_events: 72,
                max_uncertain: 10,
                max_bytes: 100_000,
                checker_states: 50_000,
                checker_branches: 100_000,
                checker_memo_bytes: 8_388_608,
                checker_reductions: 256,
            },
        };
        let admitted = super::admit(&profile).expect("bounded profile");
        assert_eq!(admitted.profile_ref(), super::canonical_ref(&profile).expect("canonical profile ref"));
        assert!(super::is_ref(admitted.profile_ref()));
        let mut changed = profile.clone();
        changed.seed = 1;
        assert_ne!(super::admit(&changed).expect("changed profile").profile_ref(), admitted.profile_ref());
        changed.keys.push("second".into());
        assert_eq!(super::admit(&changed), Err(super::Issue::SingleKey));
    }
}
