fn reference(value: &str) -> String {
    super::identity(&serde_json::json!({"test-definition":value})).unwrap()
}
fn public(byte: u8) -> String {
    iroh::SecretKey::from_bytes(&[byte; 32]).public().to_string()
}

fn node(index: u8) -> super::NodeInput {
    let id = format!("node-{index}");
    super::NodeInput {
        node_id: id.clone(),
        grant_id: format!("grant-{index}"),
        source_binding_ref: reference("source"),
        endpoint: format!("127.0.0.1:{}", 19000 + u16::from(index)).parse().unwrap(),
        transport_public_key: public(index),
        transport_key_file: "transport-key.json".into(),
        bind_addr: format!("127.0.0.1:{}", 19100 + u16::from(index)).parse().unwrap(),
        client_bind_addr: format!("127.0.0.1:{}", 19200 + u16::from(index)).parse().unwrap(),
        probe_public_key: public(index + 10),
        client_public_key: public(30),
        client_scope_ref: reference("client-scope"),
        transport_capability_ref: reference("transport-capability"),
        time_capability_ref: reference("time-capability"),
        listener_identity_ref: reference(&format!("listener-{index}")),
        peer_context_ref: reference("peer-context"),
        locator_cohort_ref: reference("locator-cohort"),
        validity: super::super::profiles::EndpointValidityCohortDto {
            cohort_ref: reference("validity"),
            not_before_tick: 0,
            expires_at_tick: 100000,
        },
        observed_tick: 1,
        transport_session_ref: reference(&format!("session-{index}")),
        transport_admission: super::super::profiles::EndpointAdmissionStateDto {
            registration_active: true,
            transport_capability_active: true,
            protocol_capability_active: true,
            profile_active: true,
            listener_ready: true,
        },
        storage_namespace: "coordination".into(),
        health_path: "health.json".into(),
        supervision_ref: reference("supervision"),
        bindings: crate::fabric_consistency::raft::REQUIRED_REPLICA_PORTS
            .iter()
            .map(|(port, version)| super::super::launch::PortBinding {
                port_id: (*port).into(),
                version: (*version).into(),
                implementation_profile: "experimental".into(),
                binding_ref: reference(&format!("binding:{id}:{port}")),
            })
            .collect(),
    }
}

fn input() -> super::Input {
    let nodes = [node(1), node(2), node(3)];
    let workload = workload(&nodes);
    super::Input {
        run_id: "recipe-test".into(),
        group: group(),
        workload,
        registry: super::super::registry::RegistryAdmission {
            object_key: "registry".into(),
            command_schema_ref: reference("command-schema"),
            endpoints: [(1, reference("endpoint-value"))].into(),
        },
        operator: super::OperatorInput {
            public_key: public(40),
            policy_generation: 1,
            allowed_operations: [
                "init".into(),
                "serve".into(),
                "probe".into(),
                "read".into(),
                "write".into(),
            ]
            .into(),
            not_before_ms: 1000,
            expires_at_ms: 2000,
            max_grant_lifetime_ms: 1000,
        },
        mechanisms: super::MechanismInput {
            protocol_id: "registry-protocol".into(),
            adapter_id: "redb".into(),
            namespace_id: "registry".into(),
            registration_authority_ref: reference("registration-capability"),
            operation_deadline_ticks: 500,
            max_duration_ticks: 100000,
        },
        nodes,
        handler_ref: reference("handler"),
        tick_ms: 10,
    }
}

fn group() -> super::super::consensus::Group {
    super::super::consensus::Group {
        group_id: "registry-group".into(),
        extension_id: "registry-extension".into(),
        service_id: "registry-service".into(),
        service_generation: 1,
        application_manifest_ref: reference("application-manifest"),
        engine_algorithm_profile: crate::fabric_consistency::raft::LIVE_RAFT_ALGORITHM_PROFILE.into(),
        engine_implementation_profile: crate::fabric_consistency::raft::LIVE_RAFT_IMPLEMENTATION_PROFILE.into(),
        membership_ref: reference("membership"),
        config_epoch: 1,
        placement_ref: reference("placement"),
        fencing_ref: reference("fencing"),
        fencing_epoch: 1,
        resource_profile_ref: reference("resources"),
        policy_refs: vec![reference("consistency-policy")],
        non_claims: vec!["live-startup-does-not-prove-production-consensus".into()],
        max_command_bytes: 65536,
        max_in_flight_operations: 32,
    }
}

fn workload(nodes: &[super::NodeInput; 3]) -> super::super::super::profile::Register {
    super::super::super::profile::Register {
        schema: "molten.live-register-profile.v1".into(),
        group: "registry-group".into(),
        service: "registry-service".into(),
        keys: vec!["registry".into()],
        initial_value: None,
        values: vec![1],
        generator_ref: reference("generator"),
        adapter_ref: reference("adapter"),
        model_ref: reference("model"),
        outcome_ref: reference("outcome"),
        seed: 1,
        read_weight: 1,
        write_weight: 1,
        clients: vec!["client".into()],
        endpoints: nodes.iter().map(|node| node.endpoint.to_string()).collect(),
        concurrency: 1,
        operations: 16,
        control_operations: 4,
        consistency_model: "register".into(),
        recovery: super::super::super::profile::RecoveryBounds {
            timeout_ms: 10000,
            stable_membership_observations: 2,
            require_all_endpoint_reads: true,
        },
        evidence: super::super::super::profile::EvidenceBounds {
            max_attempts_per_operation: 1,
            max_events: 64,
            max_uncertain: 16,
            max_bytes: 1048576,
            checker_states: 1000,
            checker_branches: 1000,
            checker_memo_bytes: 1048576,
            checker_reductions: 1000,
        },
    }
}

#[test]
fn binds_exact_three_nodes_and_peer_artifacts_without_signing() {
    let input = input();
    let output = super::materialize(input.clone()).unwrap();
    assert_eq!(output.configurations.keys().cloned().collect::<Vec<_>>(), vec!["node-1", "node-2", "node-3"]);
    for node in &input.nodes {
        let configuration = &output.configurations[&node.node_id];
        let grant = &output.unsigned_grants[&node.node_id];
        assert_eq!(grant.source_binding_ref, node.source_binding_ref);
        assert_eq!(grant.configuration, serde_json::to_value(configuration).unwrap());
        assert_eq!(grant.allowed_operations, input.operator.allowed_operations);
        let expected = &output.readiness_admission[&node.endpoint.to_string()];
        assert_eq!(expected.probe_public_key, node.probe_public_key);
        assert_eq!(expected.state.grant_ref, super::super::super::authority::grant_identity(grant).unwrap());
        assert_eq!(
            expected.state.bootstrap_ref,
            super::super::control::bootstrap_reference(&expected.state.grant_ref, grant).unwrap()
        );
        assert_eq!(expected.state.group_ref, configuration.replica.group_binding_ref);
        assert!(expected.state.ready);
        for (remote, peer) in &configuration.transport.peers {
            let value = crate::preserves_rail::parse_canonical_bytes(&peer.endpoint).unwrap();
            let endpoint = crate::fabric_transport::cross_process::parse_endpoint(&value).unwrap();
            assert_eq!(endpoint.descriptor_ref, output.configurations[remote].transport.local_descriptor_ref);
            assert_eq!(peer.session_ref, output.unsigned_grants[remote].transport_session_ref);
        }
    }
}

#[test]
fn rejects_key_reuse_and_duplicate_node_or_socket() {
    let mut reused = input();
    reused.nodes[0].probe_public_key = reused.operator.public_key.clone();
    assert!(super::materialize(reused).is_err());
    let mut duplicate = input();
    duplicate.nodes[1].node_id = duplicate.nodes[0].node_id.clone();
    assert!(super::materialize(duplicate).is_err());
    let mut placeholder = input();
    placeholder.nodes[0].bind_addr.set_port(0);
    assert!(super::materialize(placeholder).is_err());
}

#[test]
fn rejects_expired_mechanisms_and_unadmitted_registry_values() {
    let mut expired = input();
    expired.nodes[1].validity.expires_at_tick = 1;
    assert!(super::materialize(expired).is_err());
    let mut substituted = input();
    substituted.workload.values = vec![2];
    assert!(super::materialize(substituted).is_err());
    let mut denied = input();
    denied.nodes[0].transport_admission.protocol_capability_active = false;
    assert!(super::materialize(denied).is_err());
}

#[test]
fn readiness_expectation_changes_with_source_bound_grant_configuration() {
    let original = input();
    let first = super::materialize(original.clone()).unwrap();
    let mut changed = original;
    changed.nodes[0].source_binding_ref = reference("different-actual-source-binding");
    let second = super::materialize(changed).unwrap();
    let endpoint = "127.0.0.1:19001";
    assert_ne!(
        first.readiness_admission[endpoint].state.grant_ref,
        second.readiness_admission[endpoint].state.grant_ref
    );
    assert_ne!(
        first.readiness_admission[endpoint].state.bootstrap_ref,
        second.readiness_admission[endpoint].state.bootstrap_ref
    );
}
