//! Component runtime profile cohort regressions.

use alloc::string::ToString;

fn export(profile: crate::component::model::RuntimeProfile) -> crate::component::model::ProfileExport {
    crate::component::model::ProfileExport {
        schema_id: crate::component::profile::COMPONENT_PROFILE_SCHEMA.into(),
        schema_version: crate::component::profile::COMPONENT_PROFILE_SCHEMA_VERSION,
        source_language: crate::component::profile::COMPONENT_PROFILE_SOURCE_LANGUAGE.into(),
        profile,
    }
}

// r[verify molten.wasm_component_adoption.patched_engine]
// r[verify molten.wasm.no_std_wasm_policy_kernel]
#[test]
fn reviewed_export_binds_exact_toolchain_wit_and_determinism() {
    // r[verify molten.wasm_component.profile]
    let profile = crate::component::profile::admit_export(export(super::support::component_profile()))
        .expect("reviewed component profile");
    crate::component::profile::validate_runtime(&profile).expect("profile validates");
    assert!(crate::refs::valid_content_ref(&crate::component::profile::r#ref(&profile)));
    assert_eq!(
        crate::component::profile::r#ref(&profile),
        crate::component::profile::r#ref(&super::support::component_profile())
    );
    crate::component::profile::validate_runtime(&super::support::effect_profile())
        .expect("effect-request cohort validates");
    assert_ne!(
        crate::component::profile::r#ref(&profile),
        crate::component::profile::r#ref(&super::support::effect_profile())
    );
}

#[test]
fn export_header_is_checked_before_the_profile() {
    let mut stale_schema = export(super::support::component_profile());
    stale_schema.schema_id = "molten.wasm-component-profile.v0".into();
    let denial = crate::component::profile::admit_export(stale_schema).expect_err("stale schema");
    assert_eq!(denial.canonical_class(), crate::component::model::DenialClass::ComponentAdmissionDenial.as_str());
    assert!(denial.blockers.iter().any(|blocker| blocker.contains("schema id")));
    let mut unsupported_version = export(super::support::component_profile());
    unsupported_version.schema_version = 2;
    unsupported_version.profile.toolchain.wasmtime = "44.0.0".into();
    let denial = crate::component::profile::admit_export(unsupported_version).expect_err("unsupported version");
    assert_eq!(denial.blockers, alloc::vec!["component profile schema version is unsupported".to_string()]);
}

#[test]
fn rejects_stale_partial_or_authority_bearing_cohort() {
    // r[verify molten.wasm_component.profile]
    // r[verify molten.wasm_component.authority]
    let profile = super::support::component_profile();
    let mut stale = profile.clone();
    stale.toolchain.wasmtime = "44.0.0".to_string();
    assert!(crate::component::profile::validate_runtime(&stale).is_err());

    let mut partial = profile.clone();
    partial.determinism.fuel_interruption = false;
    assert!(crate::component::profile::validate_runtime(&partial).is_err());

    let mut unsupported = profile.clone();
    unsupported.features.tail_call = true;
    assert!(crate::component::profile::validate_runtime(&unsupported).is_err());

    let mut ambient = profile.clone();
    ambient.allowed_imports = alloc::vec!["wasi:filesystem/types@0.2.6".to_string()];
    ambient.allowed_wasi_interfaces = ambient.allowed_imports.clone();
    assert!(crate::component::profile::validate_runtime(&ambient).is_err());

    let mut effect_without_import = super::support::effect_profile();
    effect_without_import.allowed_imports.clear();
    assert!(crate::component::profile::validate_runtime(&effect_without_import).is_err());
}

// r[verify molten.wasm_component.execution_quantum]
#[test]
fn slicing_binds_only_the_time_sliced_profile_with_a_covering_budget() {
    let mut sliced = super::support::component_profile();
    sliced.profile_id = crate::component::profile::TIME_SLICED_PROFILE_ID.into();
    assert!(crate::component::profile::validate_runtime(&sliced).is_err(), "time slicing requires a binding");
    sliced.slicing = Some(crate::component::model::SlicingProfile {
        quantum_fuel: 1_000,
        total_budget_fuel: 10_000,
    });
    crate::component::profile::validate_runtime(&sliced).expect("bound time-sliced profile");
    sliced.slicing = Some(crate::component::model::SlicingProfile {
        quantum_fuel: 1_000,
        total_budget_fuel: 999,
    });
    assert!(crate::component::profile::validate_runtime(&sliced).is_err());
    let mut synchronous = super::support::component_profile();
    synchronous.slicing = Some(crate::component::model::SlicingProfile {
        quantum_fuel: 1,
        total_budget_fuel: 1,
    });
    assert!(crate::component::profile::validate_runtime(&synchronous).is_err());
}
