//! Execution identity binds admission to the shared runtime source cohort, not just its inputs.

/// Canonical source identity of the compiled shared runtime dependency cohort.
/// This is linkage evidence, not execution, authority, or release approval.
// r[impl molten.wasm_component_adoption.consumers]
pub fn source_cohort() -> preserves::IOValue {
    crate::preserves_rail::record("hardened-wasmtime-runtime-source-v1", vec![
        crate::preserves_rail::string("hardened-wasmtime-shell.component"),
        crate::preserves_rail::string(
            "rad://z3hRCegTsS8jpJVgxYfb9psEzxHpG/z6MksnXbFoE8zkCkGWhHc8zuxpnEUhrJHv2KECRV4GSv9gkx",
        ),
        crate::preserves_rail::string("cf4a1913f7f2ca7eb7f655663e87b8b72d0c506c"),
        crate::preserves_rail::sequence(vec![
            crate::preserves_rail::string("hardened-wasmtime-runtime@0.1.0"),
            crate::preserves_rail::string("hardened-wasmtime-shell@0.1.0"),
        ]),
    ])
}

// r[impl molten.wasm_component_adoption.consumers]
pub(super) fn bind_configuration_ref(admission_ref: &str) -> super::super::model::Result<String> {
    let value = crate::preserves_rail::record("wasm-component-runtime-configuration-v2", vec![
        crate::preserves_rail::string(admission_ref),
        source_cohort(),
    ]);
    crate::preserves_rail::canonical_hash(&value)
        .map_err(|error| super::super::model::Denial::new(format!("runtime source binding failed: {error}")))
}
