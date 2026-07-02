type Outcome<T> = molten::error::Result<T>;

const ROUTER_GENERATION_ONE: u64 = 1;
const ROUTER_GENERATION_TWO: u64 = 2;
const FRAME_SEQUENCE: u64 = 0;
const FRAME_MAX_BYTES: u64 = 4_096;
const FRAME_MAX_COUNT: u64 = 64;
const RELAY_LATENCY_MS: u64 = 25;
const WATCHER_OBSERVED_EVENTS: u64 = 3;
const WATCHER_RETAINED_EVENTS: u64 = 1;
const METRIC_VALUE: u64 = 1;
const PORT_DURATION_SECONDS: u64 = 600;
const EXTERNAL_PORT: u64 = 443;
const INTERNAL_PORT: u64 = 8443;

const _: () = assert!(ROUTER_GENERATION_ONE < ROUTER_GENERATION_TWO);
const _: () = assert!(FRAME_MAX_BYTES > 0);
const _: () = assert!(FRAME_MAX_COUNT > 0);

pub(super) fn router_fixture(input: super::command::iroh::RouterFixture) -> Outcome<()> {
    let install_input = router_input("install", ROUTER_GENERATION_ONE);
    let install =
        molten::node_iroh::evaluate_router_operation(&molten::node_iroh::empty_protocol_registry(), &install_input)?;
    let mut replace_input = router_input("replace", ROUTER_GENERATION_TWO);
    replace_input.prior_generation = Some(ROUTER_GENERATION_ONE);
    replace_input.shutdown_evidence_ref = Some(fixture_ref("router-shutdown-one"));
    let replace = molten::node_iroh::evaluate_router_operation(&install.registry, &replace_input)?;
    let mut remove_input = router_input("remove", ROUTER_GENERATION_TWO);
    remove_input.prior_generation = Some(ROUTER_GENERATION_TWO);
    remove_input.shutdown_evidence_ref = Some(fixture_ref("router-shutdown-two"));
    let remove = molten::node_iroh::evaluate_router_operation(&replace.registry, &remove_input)?;
    let unsupported_input = molten::node_iroh::RouterOperationInput {
        operation: "unsupported-alpn".to_string(),
        authority_refs: Vec::new(),
        policy_refs: Vec::new(),
        resource_refs: Vec::new(),
        evidence_refs: Vec::new(),
        ..router_input("unsupported-alpn", ROUTER_GENERATION_ONE)
    };
    let unsupported = molten::node_iroh::evaluate_router_operation(&remove.registry, &unsupported_input)?;
    let value = molten::preserves_rail::record("node-iroh-router-fixture-v1", vec![
        molten::preserves_rail::record("insert", vec![install.receipt_value]),
        molten::preserves_rail::record("replace", vec![replace.receipt_value]),
        molten::preserves_rail::record("remove", vec![remove.receipt_value]),
        molten::preserves_rail::record("unsupported", vec![unsupported.receipt_value]),
    ]);
    super::core::emit_named_receipt(input.out.as_ref(), "node-iroh-router-fixture", &value)
}

pub(super) fn frame_fixture(input: super::command::iroh::FrameFixture) -> Outcome<()> {
    let registry = molten::node_iroh::evaluate_router_operation(
        &molten::node_iroh::empty_protocol_registry(),
        &router_input("install", ROUTER_GENERATION_ONE),
    )?
    .registry;
    let envelope =
        molten::preserves_rail::record("node-control-envelope", vec![molten::preserves_rail::string("status")]);
    let bytes = molten::preserves_rail::canonical_bytes(&envelope)?;
    let frame = molten::node_iroh::evaluate_framed_envelope(&registry, &molten::node_iroh::FramedEnvelopeInput {
        alpn: "molten/node-control/1".to_string(),
        peer: "peer:operator".to_string(),
        node: "node:local".to_string(),
        stream_id: "stream:fixture".to_string(),
        sequence: FRAME_SEQUENCE,
        declared_length: bytes.len() as u64,
        declared_envelope_ref: molten::preserves_rail::content_ref_from_bytes(&bytes),
        envelope_bytes: bytes,
        limit_profile_ref: molten::node_iroh::default_limit_profile_ref(),
        authority_refs: refs("authority"),
        policy_refs: refs("policy"),
        resource_refs: refs("resource"),
        evidence_refs: refs("frame-evidence"),
        limits: molten::node_iroh::FramedEnvelopeLimits {
            max_frame_bytes: FRAME_MAX_BYTES,
            max_frames_per_session: FRAME_MAX_COUNT,
            max_outstanding_frames: FRAME_MAX_COUNT,
        },
    })?;
    let frame_ref = molten::preserves_rail::canonical_hash(&frame.receipt_value)?;
    let service = molten::node_iroh::evaluate_service_session(&molten::node_iroh::ServiceSessionInput {
        service_id: "node-control".to_string(),
        operation_id: "status".to_string(),
        interaction_kind: "unary".to_string(),
        path_kind: "remote".to_string(),
        request_ref: fixture_ref("service-request"),
        response_refs: refs("service-response"),
        capability_refs: refs("capability"),
        policy_refs: refs("policy"),
        resource_refs: refs("resource"),
        alpn: Some("molten/node-control/1".to_string()),
        peer: Some("peer:operator".to_string()),
        node: Some("node:local".to_string()),
        stream_id: Some("stream:fixture".to_string()),
        frame_receipt_refs: vec![frame_ref],
    })?;
    let value = molten::preserves_rail::record("node-iroh-frame-fixture-v1", vec![
        molten::preserves_rail::record("frame", vec![frame.receipt_value]),
        molten::preserves_rail::record("service-session", vec![service.receipt_value]),
    ]);
    super::core::emit_named_receipt(input.out.as_ref(), "node-iroh-frame-fixture", &value)
}

pub(super) fn diagnostics_fixture(input: super::command::iroh::DiagnosticsFixture) -> Outcome<()> {
    let report = molten::node_iroh::network_diagnostics_report(&molten::node_iroh::NetworkDiagnosticsInput {
        nat_class: "cone".to_string(),
        udp_status: "pass".to_string(),
        direct_path_status: "degraded".to_string(),
        relay_latency_ms: Some(RELAY_LATENCY_MS),
        port_map_protocols: vec!["pcp".to_string(), "nat-pmp".to_string()],
        interface_refs: refs("interface"),
        route_refs: refs("route"),
        live_observations_recorded: false,
        diagnostics: Vec::new(),
    })?;
    let probe = molten::node_iroh::connectivity_probe_receipt(&molten::node_iroh::ConnectivityProbeInput {
        source_node: "node:a".to_string(),
        target_node: "node:b".to_string(),
        expected_endpoint_ref: fixture_ref("endpoint"),
        observed_endpoint_ref: Some(fixture_ref("endpoint")),
        direct_path_status: "deny".to_string(),
        relay_path_status: "pass".to_string(),
        timeout_ms: None,
        authority_refs: refs("authority"),
        policy_refs: refs("policy"),
        resource_refs: refs("resource"),
        evidence_refs: refs("probe"),
    })?;
    let watcher = molten::node_iroh::watcher_snapshot_value(&molten::node_iroh::NetworkWatcherInput {
        node: "node:a".to_string(),
        interface_state: "eth0-up".to_string(),
        address_state: "ipv6-ready".to_string(),
        default_route: "relay".to_string(),
        relay_state: "online".to_string(),
        endpoint_state: "listening".to_string(),
        observed_event_count: WATCHER_OBSERVED_EVENTS,
        retained_event_count: WATCHER_RETAINED_EVENTS,
        evidence_refs: refs("watcher"),
    })?;
    let value = molten::preserves_rail::record("node-network-diagnostics-fixture-v1", vec![
        molten::preserves_rail::record("report", vec![report.receipt_value]),
        molten::preserves_rail::record("probe", vec![probe.receipt_value]),
        molten::preserves_rail::record("watcher", vec![watcher.receipt_value]),
    ]);
    super::core::emit_named_receipt(input.out.as_ref(), "node-network-diagnostics-fixture", &value)
}

pub(super) fn metrics_fixture(input: super::command::iroh::MetricsFixture) -> Outcome<()> {
    let metrics = molten::node_iroh::metrics_snapshot(&molten::node_iroh::MetricsSnapshotInput {
        node: "node:a".to_string(),
        scrape_ref: fixture_ref("scrape"),
        policy_refs: refs("policy"),
        redaction_refs: refs("redaction"),
        samples: vec![molten::node_iroh::MetricSample {
            name: "molten_node_queue_depth".to_string(),
            kind: "gauge".to_string(),
            value: METRIC_VALUE,
            labels: vec![("route".to_string(), "redacted".to_string())],
        }],
    })?;
    super::core::emit_named_receipt(input.out.as_ref(), "node-metrics-snapshot", &metrics.receipt_value)
}

pub(super) fn port_mapping_fixture(input: super::command::iroh::PortMappingFixture) -> Outcome<()> {
    let receipt = molten::node_iroh::port_mapping_receipt(&molten::node_iroh::PortMappingInput {
        mode: if input.attempt { "mutate" } else { "probe" }.to_string(),
        requester_ref: input.attempt.then(|| fixture_ref("requester")),
        identity_ref: input.attempt.then(|| fixture_ref("node")),
        protocol: "pcp".to_string(),
        external_port: input.attempt.then_some(EXTERNAL_PORT),
        internal_port: input.attempt.then_some(INTERNAL_PORT),
        duration_seconds: input.attempt.then_some(PORT_DURATION_SECONDS),
        authority_refs: if input.attempt { refs("authority") } else { Vec::new() },
        policy_refs: if input.attempt { refs("policy") } else { Vec::new() },
        resource_refs: if input.attempt { refs("resource") } else { Vec::new() },
        operator_evidence_refs: if input.attempt { refs("operator") } else { Vec::new() },
        available_protocols: vec!["pcp".to_string()],
    })?;
    super::core::emit_named_receipt(input.out.as_ref(), "node-port-mapping-fixture", &receipt.receipt_value)
}

pub(super) fn external_bridge_fixture(input: super::command::iroh::ExternalBridgeFixture) -> Outcome<()> {
    let receipt =
        molten::node_iroh::external_diagnostics_bridge_receipt(&molten::node_iroh::ExternalDiagnosticsBridgeInput {
            enabled: input.enable,
            mode: "push".to_string(),
            target_service_ref: input.enable.then(|| fixture_ref("target-service")),
            capability_refs: if input.enable { refs("capability") } else { Vec::new() },
            policy_refs: if input.enable { refs("policy") } else { Vec::new() },
            redaction_policy_refs: if input.enable { refs("redaction") } else { Vec::new() },
            api_secret_provenance_ref: input.enable.then(|| fixture_ref("secret-provenance")),
            operator_evidence_refs: if input.enable { refs("operator") } else { Vec::new() },
            expiry_ref: input.enable.then(|| fixture_ref("expiry")),
        })?;
    super::core::emit_named_receipt(input.out.as_ref(), "node-external-diagnostics-bridge", &receipt.receipt_value)
}

fn router_input(operation: &str, generation: u64) -> molten::node_iroh::RouterOperationInput {
    molten::node_iroh::RouterOperationInput {
        operation: operation.to_string(),
        alpn: "molten/node-control/1".to_string(),
        handler_kind: "node-control".to_string(),
        generation,
        prior_generation: None,
        authority_refs: refs("authority"),
        policy_refs: refs("policy"),
        resource_refs: refs("resource"),
        evidence_refs: refs("router-evidence"),
        shutdown_evidence_ref: None,
    }
}

fn refs(label: &str) -> Vec<String> {
    vec![fixture_ref(label)]
}

fn fixture_ref(label: &str) -> String {
    molten::node_iroh::fixture_ref(label)
}
