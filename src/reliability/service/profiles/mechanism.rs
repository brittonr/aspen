//! Explicit bounded three-node experimental mechanism recipe, not an authorization grant.

/// Caller supplies real signed-grant and schema references. Tick units belong to the selected
/// runtime.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentalInput {
    pub service_id: String,
    pub extension_id: String,
    pub protocol_id: String,
    pub adapter_id: String,
    pub namespace_id: String,
    pub registration_authority_ref: String,
    pub value_schema_ref: String,
    pub operation_deadline_ticks: u64,
    pub max_duration_ticks: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Experimental {
    pub transport: super::TransportProfileDto,
    pub protocol: super::ProtocolDescriptorDto,
    pub time: super::TimeProfileDescriptorDto,
    pub durable: super::DurableStateProfileDto,
    pub namespace: super::DurableNamespaceDescriptorDto,
}

/// Builds and admits every mechanism before returning serializable configuration.
/// Group membership, exact peer identities, sockets, roots, and effect authority remain explicit in
/// the caller.
pub fn experimental(input: ExperimentalInput) -> crate::error::Result<Experimental> {
    let transport = transport_definition(&input)?;
    let durable = durable_definition(&input)?;
    let time = time_definition(&input)?;
    let protocol = protocol_definition(&input, &transport)?;
    let namespace = namespace_definition(&input, &durable)?;
    let profiles = Experimental {
        transport: decode(transport)?,
        protocol: decode(protocol)?,
        time: decode(time)?,
        durable: decode(durable)?,
        namespace: decode(namespace)?,
    };
    let admitted_transport = profiles.transport.clone().into_profile()?;
    profiles.protocol.clone().into_descriptor(&admitted_transport)?;
    profiles.time.clone().into_descriptor()?;
    let admitted_durable = profiles.durable.clone().into_profile()?;
    profiles.namespace.clone().into_descriptor(&admitted_durable)?;
    Ok(profiles)
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> crate::error::Result<T> {
    serde_json::from_value(value).map_err(|error| super::invalid(format!("experimental mechanism: {error}")))
}

/// Identity covers the entire domain-separated definition, before its own reference is inserted.
fn referenced(mut definition: serde_json::Value) -> crate::error::Result<serde_json::Value> {
    let projection = super::super::super::checker::canonical_value(&definition).map_err(super::invalid)?;
    let value = crate::preserves_rail::record("molten.experimental.mechanism-definition.v1", vec![projection]);
    definition["profile_ref"] = serde_json::json!(crate::preserves_rail::canonical_hash(&value)?);
    Ok(definition)
}

fn transport_definition(input: &ExperimentalInput) -> crate::error::Result<serde_json::Value> {
    referenced(serde_json::json!({
        "schema": crate::fabric_transport::TRANSPORT_PROFILE_SCHEMA, "profile_id": input.service_id,
        "adapter_kind": "iroh-live", "capabilities": ["bidirectional-streams"],
        "limits": { "max_listeners": 1, "max_sessions": 8, "max_streams_per_session": 4,
            "max_frame_bytes": 65536, "max_datagram_bytes": 1024, "max_queued_events": 128,
            "max_queued_bytes": 1048576, "max_inflight_bytes": 1048576,
            "operation_deadline_ticks": input.operation_deadline_ticks },
        "non_claims": crate::fabric_transport::REQUIRED_TRANSPORT_NON_CLAIMS.map(|value| value.as_str())
    }))
}

fn durable_definition(input: &ExperimentalInput) -> crate::error::Result<serde_json::Value> {
    referenced(serde_json::json!({
        "schema": crate::fabric_durability::DURABLE_STATE_PROFILE_SCHEMA, "profile_id": input.service_id,
        "adapter_kind": "live-redb", "supported_levels": ["machine-loss"],
        "max_namespaces": 1, "max_log_records": 65536, "max_ordered_entries": 65536,
        "max_operation_bytes": 65536, "max_namespace_bytes": 67108864,
        "max_batch_operations": 64, "max_snapshots": 8, "max_effect_transactions": 128,
        "non_claims": crate::fabric_durability::REQUIRED_DURABILITY_NON_CLAIMS.map(|value| value.as_str())
    }))
}

fn time_definition(input: &ExperimentalInput) -> crate::error::Result<serde_json::Value> {
    referenced(serde_json::json!({
        "schema": crate::fabric_time::FABRIC_TIME_PROFILE_SCHEMA, "profile_id": input.service_id,
        "kind": "live", "supported_domains": ["wall-clock", "monotonic", "logical", "virtual"],
        "max_duration_ticks": input.max_duration_ticks, "max_uncertainty_ticks": 0,
        "max_timers": 128, "max_runnables": 128, "max_entropy_request_bytes": 4096,
        "max_entropy_total_bytes": 1048576, "max_scheduler_concurrency": 4,
        "max_scheduler_queue_depth": 128, "fairness_bound_turns": null,
        "scheduler_policy": { "ordering": "fifo", "replay": "recorded-choice-required", "overload": "reject" },
        "evidence_mode": "selected-semantic-boundaries",
        "non_claims": crate::fabric_time::REQUIRED_TIME_NON_CLAIMS.map(|value| value.as_str())
    }))
}

fn protocol_definition(
    input: &ExperimentalInput,
    profile: &serde_json::Value,
) -> crate::error::Result<serde_json::Value> {
    let framing = referenced(serde_json::json!({
        "profile_id": input.protocol_id, "max_frame_bytes": 65536,
        "length_prefix_bytes": 8, "payload_hash_required": true
    }))?;
    Ok(serde_json::json!({
        "schema": crate::fabric_transport::TRANSPORT_PROTOCOL_SCHEMA, "protocol_id": input.protocol_id,
        "version": "v1", "alpn": "molten/experimental-coordination/1",
        "extension_id": input.extension_id, "service_id": input.service_id,
        "generation": 1, "listener_limit": 1, "requested_capabilities": ["bidirectional-streams"],
        "framing": framing, "cleanup_policy": { "kind": "immediate" },
        "registration_authority_ref": input.registration_authority_ref,
        "profile_ref": profile["profile_ref"]
    }))
}

fn namespace_definition(
    input: &ExperimentalInput,
    profile: &serde_json::Value,
) -> crate::error::Result<serde_json::Value> {
    Ok(serde_json::json!({
        "schema": crate::fabric_durability::DURABLE_STATE_NAMESPACE_SCHEMA, "profile_ref": profile["profile_ref"],
        "adapter_id": input.adapter_id, "namespace_id": input.namespace_id, "generation": 1,
        "value_schema_ref": input.value_schema_ref, "retention_authority_ref": null, "quota_bytes": 67108864,
        "atomicity_domain": { "domain_id": input.namespace_id, "adapter_id": input.adapter_id,
            "namespace_id": input.namespace_id, "generation": 1,
            "object_classes": ["log-record", "ordered-value", "snapshot", "checkpoint", "effect-transaction"],
            "max_operations": 64, "max_bytes": 67108864, "supported_levels": ["machine-loss"] }
    }))
}
