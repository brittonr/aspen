use std::collections::BTreeSet;

use molten_core::coordination_delivery::*;

use crate::system_extension::CanonicalAdmittedSystemExtensionManifest;

pub const DELIVERY_HOST_BINDING_SCHEMA: &str = "molten.coordination-delivery-host-binding.v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryHostBindingFacts {
    pub schema: String,
    pub system_extension_manifest_ref: String,
    pub extension_id: String,
    pub service_id: String,
    pub service_generation: u64,
    pub lifecycle_running: bool,
    pub port_bindings: std::collections::BTreeMap<String, String>,
}

// r[impl molten.coordination_delivery.consistency_durability]
pub fn project_delivery_host_binding(
    admitted: &CanonicalAdmittedSystemExtensionManifest,
    active_generation: u64,
    lifecycle_running: bool,
    delivery: &DeliveryManifest,
) -> Result<DeliveryHostBindingFacts, DeliveryIssue> {
    let system = admitted.manifest();
    let admitted_binding_refs = admitted
        .required_port_bindings()
        .iter()
        .map(|binding| binding.binding_ref.as_str())
        .collect::<BTreeSet<_>>();
    if system.extension_id != delivery.extension_id
        || system.service_id != delivery.service_id
        || system.implementation_ref != delivery.implementation_ref
        || !system.policy_refs.contains(&delivery.policy_ref)
        || !delivery.port_bindings.values().all(|reference| admitted_binding_refs.contains(reference.as_str()))
    {
        return Err(DeliveryIssue::HostBindingMismatch);
    }
    let facts = DeliveryHostBindingFacts {
        schema: DELIVERY_HOST_BINDING_SCHEMA.to_string(),
        system_extension_manifest_ref: admitted.manifest_ref().to_string(),
        extension_id: system.extension_id.clone(),
        service_id: system.service_id.clone(),
        service_generation: active_generation,
        lifecycle_running,
        port_bindings: delivery.port_bindings.clone(),
    };
    validate_delivery_host_binding(&facts, delivery)?;
    Ok(facts)
}

pub fn validate_delivery_host_binding(
    facts: &DeliveryHostBindingFacts,
    delivery: &DeliveryManifest,
) -> Result<(), DeliveryIssue> {
    if facts.schema != DELIVERY_HOST_BINDING_SCHEMA
        || facts.system_extension_manifest_ref.is_empty()
        || facts.extension_id != delivery.extension_id
        || facts.service_id != delivery.service_id
        || facts.service_generation != delivery.service_generation
        || facts.port_bindings != delivery.port_bindings
    {
        return Err(DeliveryIssue::HostBindingMismatch);
    }
    if !facts.lifecycle_running {
        return Err(DeliveryIssue::HostNotRunning);
    }
    Ok(())
}
