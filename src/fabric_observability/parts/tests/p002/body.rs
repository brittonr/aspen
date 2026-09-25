
// r[verify molten.fabric_observability.health_scope]
// r[verify molten.fabric_observability.final_validation]
#[test]
fn node_and_extension_health_project_to_scoped_canonical_readiness_and_operator_snapshot() {
    let profile = profile();
    let extension_state = crate::system_extension::LifecycleState {
        generation: GENERATION_ONE,
        phase: crate::system_extension::LifecyclePhase::Running,
        restart_attempts: 0,
        health: crate::system_extension::HealthState::Healthy,
        checkpoint_ref: None,
    };
    let extension_source_ref = test_ref("extension-a");
    let node_source_ref = test_ref("node-a");
    let resource_ref = test_ref("health-resource");
    let extension = system_extension_health_input(
        health_projection(
            "extension-a",
            &extension_source_ref,
            &profile.profile_ref,
            &resource_ref,
            ClaimScope::SystemExtension,
        ),
        &extension_state,
    );
    canonical_health_input(&profile, &extension, OBSERVED_TICK).expect("extension health");
    let node = node_health_input(
        health_projection("node-a", &node_source_ref, &profile.profile_ref, &resource_ref, ClaimScope::LocalComponent),
        "pass",
    )
    .expect("node health");
    canonical_health_input(&profile, &node, OBSERVED_TICK).expect("node health canonical");

    assert_failed_health_inputs(&profile, &extension_source_ref, &node_source_ref, &resource_ref);

    let policy = ReadinessPolicy {
        schema: READINESS_POLICY_SCHEMA.to_string(),
        policy_ref: test_ref("extension-readiness"),
        target_scope: ClaimScope::SystemExtension,
        required_source_ids: vec!["extension-a".to_string()],
        scope_evidence_refs: Vec::new(),
        allow_degraded: false,
        as_of_tick: OBSERVED_TICK,
    };
    canonical_readiness_policy(&profile, &policy).expect("readiness policy");
    let decision =
        evaluate_health_readiness(&profile, &policy, HealthState::Unavailable, std::slice::from_ref(&extension));
    let canonical_decision = canonical_health_decision(&decision).expect("health decision");
    assert_eq!(decision.readiness, ReadinessDecision::Pass);

    let operator = bounded_operator_snapshot(&profile, SnapshotBuildInput {
        snapshot_id: "operator-extension-a",
        profile_ref: &profile.profile_ref,
        scope: ClaimScope::SystemExtension,
        generation: GENERATION_ONE,
        as_of_tick: OBSERVED_TICK,
        valid_until_tick: FRESH_UNTIL_TICK,
        series: snapshot().series,
        event_refs: Vec::new(),
        health_refs: vec![canonical_decision.artifact_ref],
        integrity_result_refs: Vec::new(),
        adapter_outcome_refs: Vec::new(),
        evidence_refs: vec![test_ref("operator-evidence")],
    })
    .expect("operator snapshot");
    assert_eq!(operator.artifact.scope, ClaimScope::SystemExtension);
    assert_eq!(observation_authority_decision(), AuthorityDecision::Deny);
}

/// A failed extension projects as failed at its new generation, and ambient node health is
/// rejected.
fn assert_failed_health_inputs(
    profile: &ObservationProfile,
    extension_source_ref: &str,
    node_source_ref: &str,
    resource_ref: &str,
) {
    let failed_extension = system_extension_health_input(
        health_projection(
            "extension-a",
            extension_source_ref,
            &profile.profile_ref,
            resource_ref,
            ClaimScope::SystemExtension,
        ),
        &crate::system_extension::LifecycleState {
            generation: GENERATION_TWO,
            phase: crate::system_extension::LifecyclePhase::Running,
            restart_attempts: 1,
            health: crate::system_extension::HealthState::Failed,
            checkpoint_ref: None,
        },
    );
    assert_eq!(failed_extension.state, HealthState::Failed);
    assert_eq!(failed_extension.context.generation, GENERATION_TWO);
    canonical_health_input(profile, &failed_extension, OBSERVED_TICK).expect("failed extension health");
    assert!(
        node_health_input(
            health_projection(
                "node-a",
                node_source_ref,
                &profile.profile_ref,
                resource_ref,
                ClaimScope::LocalComponent,
            ),
            "ambient-healthy",
        )
        .is_err()
    );
}

fn health_projection<'a>(
    source_id: &'a str,
    source_ref: &'a str,
    profile_ref: &'a str,
    resource_ref: &'a str,
    scope: ClaimScope,
) -> HealthProjectionInput<'a> {
    HealthProjectionInput {
        source_id,
        source_ref,
        profile_ref,
        scope,
        generation: GENERATION_ONE,
        observed_tick: OBSERVED_TICK,
        valid_until_tick: FRESH_UNTIL_TICK,
        resource_ref,
        evidence_refs: vec![test_ref("health-evidence")],
        diagnostic_refs: Vec::new(),
    }
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    crate::test_support::cleanup_stale_molten_temp_dirs();
    static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
    }
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
