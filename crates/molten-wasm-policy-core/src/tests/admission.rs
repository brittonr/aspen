//! Pure component execution planning over parsed facts, without Wasmtime or component bytes.

use alloc::string::ToString;

fn admitted_surface(
    _: &crate::component::model::RuntimeProfile,
    _: &crate::component::materialization::Admission,
    _: &crate::component::admission::ArtifactFacts,
) -> crate::component::model::Result<()> {
    Ok(())
}

fn grant(import: &str) -> crate::component::admission::ImportGrant {
    crate::component::admission::ImportGrant {
        import: import.into(),
        capability: "hostcall:blob-get".into(),
        policy_ref: super::support::fixture_ref("grant-policy"),
        authority_ref: super::support::fixture_ref("grant-authority"),
        resource_ref: super::support::fixture_ref("grant-resource"),
        recorded_effect_ref: super::support::fixture_ref("grant-effect"),
    }
}

// r[verify molten.wasm.no_std_wasm_policy_kernel]
#[test]
fn identical_facts_produce_identical_pure_execution_plans() {
    // r[verify molten.wasm_component.determinism]
    // r[verify molten.wasm_component.functional_core]
    let profile = super::support::component_profile();
    let facts = super::support::facts(&profile);
    let left = crate::component::admission::plan_execution(
        &profile,
        super::support::materialization(&profile),
        &facts,
        &[],
        admitted_surface,
    )
    .expect("left plan");
    let right = crate::component::admission::plan_execution(
        &profile,
        super::support::materialization(&profile),
        &facts,
        &[],
        admitted_surface,
    )
    .expect("right plan");
    assert_eq!(left, right);
    assert!(left.imports.is_empty());
    assert!(left.capabilities.is_empty());
    assert_eq!(left.profile_ref, crate::component::profile::r#ref(&profile));
    assert!(crate::refs::valid_content_ref(&left.runtime_configuration_ref));
    let mut other_component = super::support::materialization(&profile);
    other_component.component_ref = super::support::fixture_ref("other-component");
    let other = crate::component::admission::plan_execution(&profile, other_component, &facts, &[], admitted_surface)
        .expect("other component plan");
    assert_ne!(left.runtime_configuration_ref, other.runtime_configuration_ref);
}

#[test]
fn declared_surface_port_runs_after_profile_validation_and_its_denial_is_final() {
    let profile = super::support::component_profile();
    let facts = super::support::facts(&profile);
    let mut stale = profile.clone();
    stale.toolchain.wasmtime = "44.0.0".into();
    let mut is_consulted = false;
    let denied = crate::component::admission::plan_execution(
        &stale,
        super::support::materialization(&profile),
        &facts,
        &[],
        |_, _, _| {
            is_consulted = true;
            Ok(())
        },
    );
    assert!(denied.is_err());
    assert!(!is_consulted, "a stale profile is denied before the declared surface is consulted");
    let surface_denial =
        crate::component::model::Denial::new("component import manifest identity is stale or tampered");
    assert_eq!(
        crate::component::admission::plan_execution(
            &profile,
            super::support::materialization(&profile),
            &facts,
            &[],
            |_, _, _| Err(surface_denial.clone())
        ),
        Err(surface_denial)
    );
}

#[test]
fn rejects_wrong_world_unsupported_feature_dynamic_growth_and_stale_cohort() {
    // r[verify molten.wasm_component.abi]
    // r[verify molten.wasm_component.resources]
    let profile = super::support::component_profile();
    let plan = |facts: &crate::component::admission::ArtifactFacts| {
        crate::component::admission::plan_execution(
            &profile,
            super::support::materialization(&profile),
            facts,
            &[],
            admitted_surface,
        )
    };
    let mut wrong_world = super::support::facts(&profile);
    wrong_world.declared_world = "other-world".into();
    assert!(plan(&wrong_world).is_err());
    let mut unsupported = super::support::facts(&profile);
    unsupported.enabled_features.push("threads".into());
    unsupported.enabled_features.sort();
    assert!(plan(&unsupported).is_err());
    let mut growing = super::support::facts(&profile);
    growing.memory.maximum = None;
    growing.memory.strategy = crate::component::model::GrowthStrategy::Dynamic;
    assert!(plan(&growing).is_err());
    let mut oversized = super::support::facts(&profile);
    oversized.memory.initial = profile.resources.max_memory_bytes + 65_536;
    oversized.memory.maximum = Some(oversized.memory.initial);
    assert!(plan(&oversized).is_err());
    let mut stale_cohort = super::support::facts(&profile);
    stale_cohort.declared_cohort_ref = super::support::fixture_ref("stale-cohort");
    assert!(plan(&stale_cohort).is_err());
    let mut no_invoke = super::support::facts(&profile);
    no_invoke.exports = alloc::vec!["run".into()];
    assert!(plan(&no_invoke).is_err());
}

#[test]
fn every_import_needs_exactly_one_complete_grant_and_no_grant_is_unused() {
    // r[verify molten.wasm_component.authority]
    let profile = super::support::effect_profile();
    let facts = super::support::facts(&profile);
    let plan = |grants: &[crate::component::admission::ImportGrant]| {
        crate::component::admission::plan_execution(
            &profile,
            super::support::materialization(&profile),
            &facts,
            grants,
            admitted_surface,
        )
    };
    let missing = plan(&[]).expect_err("ungranted import");
    assert!(missing.blockers.iter().any(|blocker| blocker.contains("requires exactly one authority grant")));
    assert!(
        plan(&[
            grant(crate::component::profile::HOST_REQUEST_IMPORT),
            grant(crate::component::profile::HOST_REQUEST_IMPORT)
        ])
        .is_err()
    );
    let mut malformed = grant(crate::component::profile::HOST_REQUEST_IMPORT);
    malformed.authority_ref = "authority".into();
    assert!(plan(&[malformed]).is_err());
    let unused = plan(&[
        grant(crate::component::profile::HOST_REQUEST_IMPORT),
        grant("wasi:clocks/wall-clock@0.2.6"),
    ])
    .expect_err("unused grant is ambient authority");
    assert_eq!(unused.blockers, alloc::vec!["component import grants include unused authority".to_string()]);

    let admitted = plan(&[grant(crate::component::profile::HOST_REQUEST_IMPORT)]).expect("granted effect import");
    assert_eq!(admitted.imports, alloc::vec![alloc::string::String::from(
        crate::component::profile::HOST_REQUEST_IMPORT
    )]);
    assert_eq!(admitted.capabilities, alloc::vec![alloc::string::String::from("hostcall:blob-get")]);
    assert_eq!(admitted.recorded_effect_refs, alloc::vec![super::support::fixture_ref("grant-effect")]);
    let mut expected_policies: alloc::vec::Vec<alloc::string::String> = alloc::vec![
        super::support::fixture_ref("materialization-policy"),
        super::support::fixture_ref("grant-policy"),
    ];
    expected_policies.sort();
    assert_eq!(admitted.policy_refs, expected_policies);
}

#[test]
fn undeclared_and_ambient_wasi_imports_are_denied_even_when_granted() {
    let profile = super::support::component_profile();
    let mut facts = super::support::facts(&profile);
    facts.imports = alloc::vec!["wasi:filesystem/types@0.2.6".into()];
    let denial = crate::component::admission::plan_execution(
        &profile,
        super::support::materialization(&profile),
        &facts,
        &[grant("wasi:filesystem/types@0.2.6")],
        admitted_surface,
    )
    .expect_err("ambient WASI");
    assert!(denial.blockers.iter().any(|blocker| blocker.contains("is not declared by the profile")));
    assert!(denial.blockers.iter().any(|blocker| blocker.contains("ambient WASI import")));
}
