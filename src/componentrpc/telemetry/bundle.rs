//! Admission of the separate Mantle telemetry emitter; hello bundles remain unchanged.
use cap_fs_ext::DirExt;

pub struct Bundle {
    verified: crate::wasm_component::VerifiedBundle,
}
impl Bundle {
    pub fn load_package(package: &cap_std::fs::Dir, staging: &cap_std::fs::Dir) -> super::Result<Self> {
        let bytes = super::super::package::read(package, "manifest.json")?;
        let manifest: serde_json::Value = serde_json::from_slice(&bytes).map_err(super::super::failure)?;
        if manifest["schema"] != "mantle-wrpc-telemetry-reference-bundle-v1"
            || manifest["capability_root"] != "capability"
            || manifest["source"]["repository"] != "https://github.com/WebAssembly/wasi-otel"
            || manifest["source"]["revision"] != super::REVISION
            || manifest["source"]["effect_interface"] != super::INTERFACE
            || manifest["source"]["guest_build_owner"] != "Mantle"
            || manifest["source"]["comparison_is_trusted_host_evidence"] != false
            || manifest["source"]["otlp_protocol_claim"] != false
            || manifest["expected_result"] != 42
            || manifest["effect_interface"] != super::INTERFACE
        {
            return Err("telemetry materialization source/cohort is not selected".into());
        }
        let entry = &manifest["telemetry"];
        if entry["bundle"] != "telemetry/materialization-bundle.json"
            || entry["runtime_profile"] != "telemetry/runtime-profile.json"
        {
            return Err("telemetry bundle/profile selector is not exact".into());
        }
        let bundle = super::super::package::read(package, "telemetry/materialization-bundle.json")?;
        let profile = super::super::package::read(package, "telemetry/runtime-profile.json")?;
        let profile: crate::wasm_component::RuntimeProfile =
            serde_json::from_slice(&profile).map_err(super::super::failure)?;
        validate_profile(&profile)?;
        if entry["runtime_profile_ref"] != crate::wasm_component::r#ref(&profile) {
            return Err("telemetry declared runtime profile identity differs".into());
        }
        let dto: serde_json::Value = serde_json::from_slice(&bundle).map_err(super::super::failure)?;
        let selected = entry["final_portable"].as_str().ok_or("telemetry final component absent")?;
        let logical = dto["final_portable"]["logical_path"].as_str().ok_or("telemetry portable member absent")?;
        if dto["bundle_identity_blake3"] != entry["bundle_identity_blake3"]
            || !selected.starts_with("telemetry/")
            || selected.split('/').any(|part| part == ".." || part.is_empty())
            || !logical.ends_with(&format!("/{selected}"))
            || manifest["comparison_only"]
                .as_array()
                .is_some_and(|paths| paths.iter().any(|path| path.as_str() == Some(selected)))
        {
            return Err("telemetry final portable selector differs from standalone materialized emitter".into());
        }
        let root = package.open_dir_nofollow("capability").map_err(super::super::failure)?;
        let verified =
            crate::wasm_component::VerifiedBundle::load(&root, staging, "rpc-otel-snapshot", &bundle, &profile)
                .map_err(super::super::failure)?;
        Ok(Self { verified })
    }
    pub fn reference(&self) -> &str {
        self.verified.bundle_ref()
    }
    pub fn component(&self) -> &[u8] {
        self.verified.component_bytes()
    }
    pub fn wit(&self) -> String {
        super::super::reference(self.verified.wit_bytes())
    }
    pub fn check(&self, scope: &super::Scope) -> super::Result<()> {
        if scope.bundle != self.reference()
            || scope.component != super::super::reference(self.component())
            || scope.wit != self.wit()
        {
            return Err("telemetry authority selects different measured component/WIT/bundle".into());
        }
        Ok(())
    }
}
fn validate_profile(profile: &crate::wasm_component::RuntimeProfile) -> super::Result<()> {
    if profile.profile_id != "molten-wrpc-wasmtime33-otel-tracing-v1"
        || profile.wit.package != "mantle:wrpc-telemetry-pilot"
        || profile.wit.world != "emitter"
        || profile.toolchain.wasmtime != "33.0.0"
        || profile.toolchain.wasmtime_wasi != "33.0.0"
        || profile.allowed_imports != [super::INTERFACE]
        || !profile.allowed_wasi_interfaces.is_empty()
        || profile.features.component_async
        || profile.resources.fuel != super::super::profile::FUEL
        || profile.resources.max_memory_bytes != super::super::profile::MEMORY_BYTES as u64
        || profile.resources.max_component_bytes > 16 * 1024 * 1024
        || profile.resources.max_wit_bytes > 1024 * 1024
    {
        return Err("telemetry requires the exact isolated sync Wasmtime33 profile".into());
    }
    Ok(())
}
