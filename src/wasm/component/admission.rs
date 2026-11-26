//! Shell for component execution admission: the pure planner lives in
//! `molten_wasm_policy_core::component::admission`; this module binds the declared
//! import-manifest port (Preserves manifest identity) and keeps byte inspection.

pub(crate) mod inspection;

pub use molten_wasm_policy_core::component::admission::ArtifactFacts;
pub use molten_wasm_policy_core::component::admission::COMPONENT_INVOKE_EXPORT;
pub use molten_wasm_policy_core::component::admission::ExecutionPlan;
pub use molten_wasm_policy_core::component::admission::GrowthFacts;
pub use molten_wasm_policy_core::component::admission::ImportGrant;
pub use molten_wasm_policy_core::component::admission::WASI_IMPORT_PREFIX;

pub fn plan_execution(
    profile: &super::model::RuntimeProfile,
    materialization: super::evidence::materialization::Admission,
    manifest: &super::imports::surface::DeclaredManifest,
    facts: &ArtifactFacts,
    grants: &[ImportGrant],
) -> super::model::Result<ExecutionPlan> {
    molten_wasm_policy_core::component::admission::plan_execution(
        profile,
        materialization,
        facts,
        grants,
        |profile, materialization, facts| admit_declared_import_surface(profile, materialization, manifest, facts),
    )
}

fn admit_declared_import_surface(
    profile: &super::model::RuntimeProfile,
    materialization: &super::evidence::materialization::Admission,
    manifest: &super::imports::surface::DeclaredManifest,
    facts: &ArtifactFacts,
) -> super::model::Result<()> {
    let world = super::imports::surface::declared_world(profile);
    let observation = super::imports::surface::facts_observation(&materialization.component_ref, facts);
    let admission = super::imports::surface::admit_declared(manifest, &world, &observation)?;
    if !admission.is_admitted {
        return Err(super::model::Denial::from_blockers(admission.blockers));
    }
    Ok(())
}
