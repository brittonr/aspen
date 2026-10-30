//! Bounded canonical admission inputs. Decoding conveys data, never authority.
#[macro_use]
pub(crate) mod wire;
mod durability;
mod execution;
mod manifest;
mod native;
#[cfg(test)]
mod tests;
mod validation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub services: Vec<ExtensionInput>,
    pub replacements: Vec<ReplacementInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacementInput {
    pub previous_manifest_ref: String,
    pub service: ExtensionInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionInput {
    pub manifest: crate::system_extension::SystemExtensionManifestInput,
    pub tier: crate::fabric::ExtensionTierRequest,
    pub descriptors: Vec<crate::fabric::FabricPortDescriptor>,
    pub host_profile: crate::system_extension::NativeHostProfile,
    pub executable: crate::system_extension::NativeExecutableEvidence,
    pub execution_profile: crate::fabric_execution::ExecutionProfileDescriptor,
    pub request: crate::fabric_execution::ExecutionRequest,
    pub authority: crate::fabric_execution::ExecutionAuthorityFacts,
    pub resources: crate::fabric_execution::ExecutionResourceGrant,
    pub resolved: crate::fabric_execution::ResolvedContext,
    pub context: crate::system_extension::CallbackContext,
    pub durability_profile: crate::fabric_durability::DurableStateProfile,
    pub namespace: crate::fabric_durability::DurableNamespaceDescriptor,
    pub artifact_path: String,
    pub storage_namespace: String,
    pub cache_namespace: String,
}

wire_struct!(Bundle, "node-service-host-bundle-v1", services, replacements);
wire_struct!(ReplacementInput, "node-service-host-replacement-v1", previous_manifest_ref, service);
wire_struct!(
    ExtensionInput,
    "node-service-host-input-v1",
    manifest,
    tier,
    descriptors,
    host_profile,
    executable,
    execution_profile,
    request,
    authority,
    resources,
    resolved,
    context,
    durability_profile,
    namespace,
    artifact_path,
    storage_namespace,
    cache_namespace
);

pub const MAX_BUNDLE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SERVICES: usize = 64;

/// Encodes only bounded, structurally valid inputs; does not grant admission.
pub fn encode(bundle: &Bundle) -> crate::error::Result<Vec<u8>> {
    validation::bundle(bundle)?;
    let value = wire::Wire::encode(bundle)?;
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_BUNDLE_BYTES {
        return Err(wire::error("service host bundle byte bound exceeded"));
    }
    Ok(bytes)
}

pub fn reference(bundle: &Bundle) -> crate::error::Result<String> {
    Ok(crate::preserves_rail::content_ref_from_bytes(&encode(bundle)?))
}

/// Exact canonical bytes and their content reference are checked before materialization.
pub fn decode(bytes: &[u8], expected_ref: &str) -> crate::error::Result<Bundle> {
    crate::preserves_rail::ContentRef::parse(expected_ref)?;
    if bytes.len() > MAX_BUNDLE_BYTES {
        return Err(wire::error("service host bundle byte bound exceeded"));
    }
    if crate::preserves_rail::content_ref_from_bytes(bytes) != expected_ref {
        return Err(wire::error("service host bundle content reference mismatch"));
    }
    let value = crate::preserves_rail::parse_canonical_bytes(bytes)?;
    let bundle = <Bundle as wire::Wire>::decode(&value)?;
    validation::bundle(&bundle)?;
    Ok(bundle)
}
