#[path = "config/assembly.rs"]
mod assembly;
#[path = "config/codec.rs"]
mod codec;
#[path = "config/group.rs"]
mod group;
#[path = "config/startup.rs"]
mod startup;
#[path = "config/stores.rs"]
mod stores;
#[cfg(test)]
#[path = "config/tests/mod.rs"]
mod tests;
#[path = "config/transport.rs"]
mod transport;
#[path = "config/validation.rs"]
mod validation;

/// Complete measured mechanism input. References name identities, never adapter substitutes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub group: crate::live_reliability::service::Group,
    pub activation: group::Activation,
    pub membership: crate::live_reliability::service::Membership,
    pub replica: crate::live_reliability::service::Replica,
    pub bindings: Vec<crate::live_reliability::service::PortBinding>,
    pub node_id: String,
    pub installation_ref: String,
    pub policy_ref: String,
    pub resource_ref: String,
    pub raft_store: crate::live_reliability::service::Durability,
    pub scope_store: crate::live_reliability::service::Durability,
    pub replay_store: crate::live_reliability::service::Durability,
    pub callback_namespace: String,
    pub transport: crate::live_reliability::service::Transport,
    pub time: crate::live_reliability::service::Time,
    pub handler_ref: String,
    pub session_ref: String,
    pub ingress_capacity: usize,
    pub ingress_delivery_limit: u64,
    pub timeout_ms: u64,
    /// Empty genesis only; privileged scopes, bindings and operations cannot be supplied.
    pub genesis: Option<Genesis>,
}

pub use group::Activation;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Genesis {
    pub consistency_epoch: u64,
    pub revocation_ref: String,
    pub observation_ref: String,
}

/// Pure admission owns no listeners, clocks, database handles or callbacks.
pub struct AdmittedPlan {
    input: Input,
    plan: crate::fabric_consistency::raft::ReplicaStartPlan,
    installation: super::super::Installation,
    time: crate::fabric_time::AdmittedTimeProfile,
    authority: super::authority::config::Unbound,
}

impl AdmittedPlan {
    pub fn installation_ref(&self) -> &str {
        &self.input.installation_ref
    }
    pub fn group_ref(&self) -> &str {
        &self.plan.state.profile.group_binding_ref
    }
    pub fn policy_ref(&self) -> &str {
        &self.input.policy_ref
    }
    pub fn resource_ref(&self) -> &str {
        &self.input.resource_ref
    }
}

/// Maintained producer for the exact canonical bytes admitted by the native runtime.
pub fn encode(input: &Input, authority: &preserves::IOValue) -> crate::error::Result<Vec<u8>> {
    let json = serde_json::to_value(input).map_err(codec::invalid)?;
    let value = crate::preserves_rail::record("collaboration-native-runtime-v1", vec![
        codec::value(&json, 0)?,
        authority.clone(),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > codec::MAX_BYTES {
        return Err(codec::invalid("runtime frame exceeds bound"));
    }
    Ok(bytes)
}

pub fn admit<E: crate::system_extension::SystemExtensionExecutor>(
    bytes: &[u8],
    host: &crate::system_extension::Dispatcher<E>,
) -> crate::error::Result<AdmittedPlan> {
    if bytes.len() > codec::MAX_BYTES {
        return Err(codec::invalid("runtime frame exceeds bound"));
    }
    let value = crate::preserves_rail::parse_canonical_bytes(bytes)?;
    let fields = super::super::decode::fields(&value, "collaboration-native-runtime-v1", 2)?;
    let json = codec::parse(&fields[0], 0)?;
    let input: Input = serde_json::from_value(json).map_err(codec::invalid)?;
    if encode(&input, &fields[1])? != bytes {
        return Err(codec::invalid("noncanonical runtime input"));
    }
    let authority = super::authority::config::parse(&fields[1])?;
    let group = group::activate(&input.group, &input.activation)?;
    let installation = super::super::Installation::from_host(host, &group)?;
    let plan = crate::fabric_consistency::plan_replica_start_for_host(
        host,
        group,
        input.node_id.clone(),
        input.membership.decode(),
        input.replica.decode(),
    )?;
    let time = input.time.profile.clone().into_descriptor()?.profile;
    validation::host(&input, host, &time)?;
    validation::validate(&input, &plan, &installation, &time)?;
    Ok(AdmittedPlan {
        input,
        plan,
        installation,
        time,
        authority,
    })
}

pub(super) async fn start(
    root: &crate::node_state::DirectoryRoot,
    admitted: AdmittedPlan,
) -> crate::error::Result<super::Parts> {
    startup::start(root, admitted).await
}
