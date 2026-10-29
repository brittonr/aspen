const MAX_INSTANCE_ID_BYTES: usize = 256;
const MAX_INSTALLED_BINDING_REFS: usize = 256;

/// Immutable, genuinely published Mantle bytes bound to one admitted manifest.
///
/// The implementation reference names the portable component bytes, not its
/// bundle, an arbitrary path, or a core-module probe. The constructor rechecks
/// materialization and import admission before any Dispatcher callback runs.
pub struct Installation {
    pub admitted: crate::system_extension::AdmittedManifest,
    pub profile: crate::wasm_component::RuntimeProfile,
    pub materialization: std::sync::Arc<crate::wasm_component::VerifiedBundle>,
    pub envelope: crate::wasm_component::AdmissionEnvelope,
    pub facts: crate::wasm_component::ArtifactFacts,
    pub import_manifest: crate::wasm_component::DeclaredManifest,
    pub import_grants: Vec<crate::wasm_component::ImportGrant>,
    pub instance_id: String,
}

impl Installation {
    pub(super) fn admit(&self) -> crate::wasm_component::Result<crate::system_extension::CallbackContext> {
        let manifest = self.admitted.manifest();
        if manifest.execution_profile != crate::system_extension::ExecutionProfile::SandboxedComponent {
            return Err(crate::wasm_component::Denial::new(
                "component executor requires a sandboxed-component manifest",
            ));
        }
        if self.instance_id.is_empty() || self.instance_id.len() > MAX_INSTANCE_ID_BYTES {
            return Err(crate::wasm_component::Denial::new(
                "component extension instance identifier is outside its byte bound",
            ));
        }
        let materialization = crate::wasm_component::verify(
            &self.profile,
            crate::wasm_component::EvidenceScope::Production,
            self.materialization.source(&self.envelope, crate::wasm_component::Consumer::SystemExtension),
        )?;
        let bytes = self.materialization.component_bytes();
        crate::wasm_component::classify_for_profile(
            crate::wasm_component::RequestedExecutionProfile::ComponentV1,
            bytes,
        )?;
        crate::wasm_component::admission::inspection::verify_artifact_facts(bytes, &self.facts)?;
        let observation = crate::wasm_component::observe_artifact_surface(
            &self.facts.declared_world,
            &materialization.component_ref,
            bytes,
        )?;
        let verdict = crate::wasm_component::admit_declared(
            &self.import_manifest,
            &crate::wasm_component::declared_world(&self.profile),
            &observation,
        )?;
        if !verdict.is_admitted {
            return Err(crate::wasm_component::Denial::from_blockers(verdict.blockers));
        }
        if materialization.component_ref != manifest.implementation_ref {
            return Err(crate::wasm_component::Denial::new(
                "extension implementation reference differs from the Mantle component",
            ));
        }
        if !manifest.policy_refs.iter().all(|reference| materialization.policy_refs.contains(reference)) {
            return Err(crate::wasm_component::Denial::new(
                "extension policy references are absent from current component admission",
            ));
        }
        if self.profile.resources.max_hostcall_bytes > manifest.resources.max_inflight_bytes
            || self.profile.resources.max_result_bytes > manifest.resources.max_inflight_bytes
            || self.profile.resources.max_concurrency > manifest.resources.max_concurrent_callbacks
        {
            return Err(crate::wasm_component::Denial::new(
                "component callback resources exceed the installed extension envelope",
            ));
        }
        crate::wasm_component::plan_execution(
            &self.profile,
            materialization,
            &self.import_manifest,
            &self.facts,
            &self.import_grants,
        )?;
        let resource = crate::preserves_rail::record("component-extension-resources-v1", vec![
            crate::preserves_rail::string(self.admitted.manifest_ref()),
            crate::preserves_rail::string(crate::wasm_component::r#ref(&self.profile)),
        ]);
        let resource_ref = crate::preserves_rail::canonical_hash(&resource)
            .map_err(|error| crate::wasm_component::Denial::new(error.to_string()))?;
        let binding_count = self.admitted.all_binding_refs().count();
        if binding_count > MAX_INSTALLED_BINDING_REFS {
            return Err(crate::wasm_component::Denial::new("component extension has too many installed port bindings"));
        }
        let mut port_binding_refs = Vec::with_capacity(binding_count);
        for reference in self.admitted.all_binding_refs() {
            if port_binding_refs.len() >= MAX_INSTALLED_BINDING_REFS {
                return Err(crate::wasm_component::Denial::new("component extension binding capacity exhausted"));
            }
            port_binding_refs.push(reference.to_string());
        }
        Ok(crate::system_extension::CallbackContext {
            manifest_ref: self.admitted.manifest_ref().to_string(),
            executable_ref: manifest.implementation_ref.clone(),
            instance_id: self.instance_id.clone(),
            extension_id: manifest.extension_id.clone(),
            service_id: manifest.service_id.clone(),
            state_ref: None,
            policy_refs: manifest.policy_refs.clone(),
            resource_ref,
            port_binding_refs,
        })
    }
}
