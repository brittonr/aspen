use super::ConsistencyGroupBinding;
use super::raft::REQUIRED_REPLICA_PORTS;
use super::raft::ReplicaPortBinding;
use super::raft::ReplicaProfile;
use super::raft::ReplicaStartInput;
use super::raft::ReplicaStartPlan;
use super::raft::StaticMembership;
use super::raft::plan_live_replica_start;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::FabricPortKey;
use crate::system_extension::LifecyclePhase;
use crate::system_extension::SystemExtensionExecutor;
use crate::system_extension::SystemExtensionHost;

// r[impl molten.fabric_consistency.live_service_ports]
pub fn plan_live_replica_start_for_host<E: SystemExtensionExecutor>(
    host: &SystemExtensionHost<E>,
    group: ConsistencyGroupBinding,
    node_id: String,
    membership: StaticMembership,
    profile: ReplicaProfile,
) -> Result<ReplicaStartPlan> {
    validate_host_scope(host, &group)?;
    let port_bindings = project_required_port_bindings(host)?;
    plan_live_replica_start(ReplicaStartInput {
        group,
        node_id,
        membership,
        profile,
        port_bindings,
    })
}

fn validate_host_scope<E: SystemExtensionExecutor>(
    host: &SystemExtensionHost<E>,
    group: &ConsistencyGroupBinding,
) -> Result<()> {
    if host.state().phase != LifecyclePhase::Running {
        return Err(MoltenError::invalid_harness(
            "live Raft startup requires a running supervised system-extension host",
        ));
    }
    let manifest = host.manifest();
    let admitted = manifest.manifest();
    if admitted.extension_id != group.extension_id || admitted.service_id != group.service_id {
        return Err(MoltenError::invalid_harness(
            "live Raft host extension or service identity does not match the consistency group",
        ));
    }
    if manifest.manifest_ref() != group.application_manifest_ref {
        return Err(MoltenError::invalid_harness(
            "live Raft host manifest ref does not match the consistency application manifest",
        ));
    }
    if host.state().generation != group.service_generation {
        return Err(MoltenError::invalid_harness("live Raft host uses a stale service generation"));
    }
    if !group.policy_refs.iter().all(|reference| admitted.policy_refs.contains(reference)) {
        return Err(MoltenError::invalid_harness(
            "live Raft group policy refs are not admitted by the system-extension host",
        ));
    }
    Ok(())
}

fn project_required_port_bindings<E: SystemExtensionExecutor>(
    host: &SystemExtensionHost<E>,
) -> Result<Vec<ReplicaPortBinding>> {
    let mut bindings = Vec::with_capacity(REQUIRED_REPLICA_PORTS.len());
    for (port_id, version) in REQUIRED_REPLICA_PORTS {
        let key = FabricPortKey {
            port_id: port_id.to_string(),
            version: version.to_string(),
        };
        let binding = host.manifest().binding_for(&key).ok_or_else(|| {
            MoltenError::invalid_harness(format!(
                "live Raft host is missing required admitted fabric port {port_id}@{version}"
            ))
        })?;
        bindings.push(ReplicaPortBinding {
            port_id: binding.binding.key.port_id.clone(),
            version: binding.binding.key.version.clone(),
            implementation_profile: binding.binding.implementation_profile.clone(),
            binding_ref: binding.binding_ref.clone(),
        });
    }
    Ok(bindings)
}
