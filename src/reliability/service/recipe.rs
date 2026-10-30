//! Deterministic three-node configuration materialization. Output remains unsigned and
//! unprovisioned.
mod network;
mod node;
#[cfg(test)]
mod tests;
mod validation;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub run_id: String,
    pub group: super::consensus::Group,
    pub workload: super::super::profile::Register,
    pub registry: super::registry::RegistryAdmission,
    pub operator: OperatorInput,
    pub mechanisms: MechanismInput,
    pub nodes: [NodeInput; 3],
    pub handler_ref: String,
    pub tick_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MechanismInput {
    pub protocol_id: String,
    pub adapter_id: String,
    pub namespace_id: String,
    pub registration_authority_ref: String,
    pub operation_deadline_ticks: u64,
    pub max_duration_ticks: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorInput {
    pub public_key: String,
    pub policy_generation: u64,
    pub allowed_operations: std::collections::BTreeSet<String>,
    pub not_before_ms: u64,
    pub expires_at_ms: u64,
    pub max_grant_lifetime_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeInput {
    pub node_id: String,
    pub grant_id: String,
    pub source_binding_ref: String,
    pub endpoint: std::net::SocketAddr,
    pub transport_public_key: String,
    pub transport_key_file: String,
    pub bind_addr: std::net::SocketAddr,
    pub client_bind_addr: std::net::SocketAddr,
    pub probe_public_key: String,
    pub client_public_key: String,
    pub client_scope_ref: String,
    pub transport_capability_ref: String,
    pub time_capability_ref: String,
    pub listener_identity_ref: String,
    pub peer_context_ref: String,
    pub locator_cohort_ref: String,
    pub validity: super::profiles::EndpointValidityCohortDto,
    pub observed_tick: u64,
    pub transport_admission: super::profiles::EndpointAdmissionStateDto,
    pub transport_session_ref: String,
    pub storage_namespace: String,
    pub health_path: String,
    pub supervision_ref: String,
    pub bindings: Vec<super::launch::PortBinding>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Output {
    pub schema: String,
    pub configurations: std::collections::BTreeMap<String, super::decoder::Configuration>,
    pub unsigned_policy: super::super::authority::Policy,
    pub unsigned_grants: std::collections::BTreeMap<String, super::super::authority::Grant>,
    pub readiness_admission: std::collections::BTreeMap<String, ReadinessAdmission>,
}

/// Expected public readiness state, independently derived before any endpoint is contacted.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReadinessAdmission {
    pub probe_public_key: String,
    pub state: super::super::authority::ProbeState,
}

/// Does not create roots, keys, signatures, admission policy, or source trust.
pub fn materialize(input: Input) -> crate::error::Result<Output> {
    validation::input(&input)?;
    let group = input.group.open(&input.mechanisms.registration_authority_ref)?;
    let mechanisms = super::profiles::experimental(super::profiles::ExperimentalInput {
        service_id: input.group.service_id.clone(),
        extension_id: input.group.extension_id.clone(),
        protocol_id: input.mechanisms.protocol_id.clone(),
        adapter_id: input.mechanisms.adapter_id.clone(),
        namespace_id: input.mechanisms.namespace_id.clone(),
        registration_authority_ref: input.mechanisms.registration_authority_ref.clone(),
        value_schema_ref: input.registry.command_schema_ref.clone(),
        operation_deadline_ticks: input.mechanisms.operation_deadline_ticks,
        max_duration_ticks: input.mechanisms.max_duration_ticks,
    })?;
    let endpoints = network::endpoints(&input, &mechanisms)?;
    let materialized = input
        .nodes
        .iter()
        .map(|node| {
            let config = node::configuration(&input, node, &group, &mechanisms, &endpoints)?;
            validation::configuration(&config, &group)?;
            let grant = grant(&input, node, &group, &config)?;
            let admission = (node.endpoint.to_string(), readiness(&grant)?);
            Ok((admission, (node.node_id.clone(), grant), (node.node_id.clone(), config)))
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    let (readiness_admission, nodes): (_, Vec<_>) =
        materialized.into_iter().map(|(readiness, grant, config)| (readiness, (grant, config))).unzip();
    let (unsigned_grants, configurations) = nodes.into_iter().unzip();
    let unsigned_policy = policy(&input, &group);
    Ok(Output {
        schema: "molten.experimental-coordination-recipe.v1".into(),
        configurations,
        unsigned_policy,
        unsigned_grants,
        readiness_admission,
    })
}

fn readiness(grant: &super::super::authority::Grant) -> crate::error::Result<ReadinessAdmission> {
    let grant_ref = super::super::authority::grant_identity(grant)?;
    let bootstrap_ref = super::control::bootstrap_reference(&grant_ref, grant)?;
    Ok(ReadinessAdmission {
        probe_public_key: grant.probe_public_key.clone(),
        state: super::super::authority::ProbeState {
            grant_ref,
            generation: grant.generation,
            group_ref: grant.group_ref.clone(),
            membership_ref: grant.membership_ref.clone(),
            transport_session_ref: grant.transport_session_ref.clone(),
            bootstrap_ref,
            ready: true,
        },
    })
}

fn policy(input: &Input, group: &crate::fabric_consistency::Group) -> super::super::authority::Policy {
    super::super::authority::Policy {
        schema: "molten.coordination-policy.v1".into(),
        generation: input.operator.policy_generation,
        operator_public_key: input.operator.public_key.clone(),
        allowed_nodes: input.nodes.iter().map(|node| node.node_id.clone()).collect(),
        allowed_groups: [group.binding_ref.clone()].into(),
        allowed_source_bindings: input.nodes.iter().map(|node| node.source_binding_ref.clone()).collect(),
        allowed_operations: input.operator.allowed_operations.clone(),
        not_before_ms: input.operator.not_before_ms,
        expires_at_ms: input.operator.expires_at_ms,
        max_grant_lifetime_ms: input.operator.max_grant_lifetime_ms,
    }
}

fn grant(
    input: &Input,
    node: &NodeInput,
    group: &crate::fabric_consistency::Group,
    config: &super::decoder::Configuration,
) -> crate::error::Result<super::super::authority::Grant> {
    Ok(super::super::authority::Grant {
        schema: "molten.coordination-grant.v1".into(),
        grant_id: node.grant_id.clone(),
        policy_generation: input.operator.policy_generation,
        generation: input.group.service_generation,
        node_id: node.node_id.clone(),
        group_ref: group.binding_ref.clone(),
        membership_ref: input.group.membership_ref.clone(),
        source_binding_ref: node.source_binding_ref.clone(),
        endpoint: node.endpoint.to_string(),
        transport_session_ref: node.transport_session_ref.clone(),
        allowed_operations: input.operator.allowed_operations.clone(),
        not_before_ms: input.operator.not_before_ms,
        expires_at_ms: input.operator.expires_at_ms,
        probe_public_key: node.probe_public_key.clone(),
        client_public_key: node.client_public_key.clone(),
        client_scope_ref: node.client_scope_ref.clone(),
        configuration: serde_json::to_value(config).map_err(|error| invalid(error.to_string()))?,
    })
}

fn identity<T: serde::Serialize>(value: &T) -> crate::error::Result<String> {
    let json = serde_json::to_value(value).map_err(|error| invalid(error.to_string()))?;
    super::super::checker::canonical_ref(&json).map_err(invalid)
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
