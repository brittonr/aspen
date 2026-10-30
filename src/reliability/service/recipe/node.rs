pub(super) fn configuration(
    input: &super::Input,
    node: &super::NodeInput,
    group: &crate::fabric_consistency::Group,
    mechanisms: &super::super::profiles::Experimental,
    endpoints: &super::network::Endpoints,
) -> crate::error::Result<super::super::decoder::Configuration> {
    let transport = super::network::configuration(input, node, mechanisms, endpoints)?;
    let replica = replica(input, node, group, mechanisms)?;
    let admission = super::super::runtime::HostAdmission {
        run_id: input.run_id.clone(),
        endpoint: node.endpoint,
        max_connections: 65536,
        peers: input.nodes.iter().map(|node| (node.node_id.clone(), node.endpoint)).collect(),
        profile: input.workload.clone(),
    };
    let peer_probes = input
        .nodes
        .iter()
        .filter(|peer| peer.node_id != node.node_id)
        .map(|peer| {
            (peer.node_id.clone(), super::super::leader::PeerProbe {
                public_key: peer.probe_public_key.clone(),
                source_binding_ref: peer.source_binding_ref.clone(),
                transport_session_ref: peer.transport_session_ref.clone(),
            })
        })
        .collect();
    Ok(super::super::decoder::Configuration {
        schema: "molten.live-registry-service.v1".into(),
        node_id: node.node_id.clone(),
        group: input.group.clone(),
        membership: super::super::consensus::Membership {
            membership_ref: input.group.membership_ref.clone(),
            config_epoch: input.group.config_epoch,
            voters: input.nodes.iter().map(|node| node.node_id.clone()).collect(),
        },
        replica,
        bindings: node.bindings.clone(),
        durability: super::super::decoder::Durability {
            profile: mechanisms.durable.clone(),
            namespace: mechanisms.namespace.clone(),
            storage_namespace: node.storage_namespace.clone(),
        },
        transport,
        time: super::super::decoder::Time {
            profile: mechanisms.time.clone(),
            capability_ref: node.time_capability_ref.clone(),
            tick_ms: input.tick_ms,
        },
        registry: input.registry.clone(),
        admission,
        handler_ref: input.handler_ref.clone(),
        ingress_capacity: 128,
        ingress_delivery_limit: 65536,
        health_path: node.health_path.clone(),
        peer_probes,
    })
}

fn replica(
    input: &super::Input,
    node: &super::NodeInput,
    group: &crate::fabric_consistency::Group,
    mechanisms: &super::super::profiles::Experimental,
) -> crate::error::Result<super::super::consensus::Replica> {
    let protocol_ref = super::identity(&mechanisms.protocol)?;
    let durable_log_ref = store_ref("log", node, mechanisms)?;
    let snapshot_store_ref = store_ref("snapshot", node, mechanisms)?;
    let entropy_profile_ref = super::identity(&serde_json::json!({
        "schema": "molten.experimental-os-entropy-binding.v1", "time_profile_ref": mechanisms.time.profile_ref,
        "capability_ref": node.time_capability_ref, "source_binding_ref": node.source_binding_ref,
    }))?;
    let mut profile = super::super::consensus::Replica {
        profile_ref: String::new(),
        group_binding_ref: group.binding_ref.clone(),
        service_generation: input.group.service_generation,
        protocol_ref,
        durable_log_ref,
        snapshot_store_ref,
        timer_profile_ref: mechanisms.time.profile_ref.clone(),
        entropy_profile_ref,
        placement_ref: input.group.placement_ref.clone(),
        fencing_ref: input.group.fencing_ref.clone(),
        fencing_epoch: input.group.fencing_epoch,
        supervision_ref: node.supervision_ref.clone(),
        resource_profile_ref: input.group.resource_profile_ref.clone(),
        heartbeat_ticks: 5,
        election_min_ticks: 20,
        election_max_ticks: 40,
        max_log_entries: 4096,
        max_message_entries: 128,
        max_effects_per_step: 128,
    };
    let mut definition = serde_json::to_value(&profile).map_err(|error| super::invalid(error.to_string()))?;
    definition
        .as_object_mut()
        .ok_or_else(|| super::invalid("replica definition must be a map"))?
        .remove("profile_ref");
    profile.profile_ref = super::identity(&serde_json::json!({
        "schema": "molten.experimental-replica-definition.v1", "profile": definition }))?;
    Ok(profile)
}

fn store_ref(
    kind: &str,
    node: &super::NodeInput,
    mechanisms: &super::super::profiles::Experimental,
) -> crate::error::Result<String> {
    super::identity(&serde_json::json!({ "schema": "molten.experimental-replica-store.v1", "kind": kind,
        "namespace": mechanisms.namespace, "storage_namespace": node.storage_namespace, "node_id": node.node_id }))
}
