pub const IROH_PROTOCOL_ROUTER_SCHEMA: &str = "molten.node.iroh-protocol-router-receipt.v1";
pub const IROH_FRAMED_ENVELOPE_SCHEMA: &str = "molten.node.iroh-framed-envelope-receipt.v1";
pub const IROH_STREAM_SESSION_SCHEMA: &str = "molten.node.iroh-stream-session-receipt.v1";
pub const NETWORK_DIAGNOSTICS_REPORT_SCHEMA: &str = "molten.node.network-diagnostics-report.v1";
pub const NETWORK_CONNECTIVITY_PROBE_SCHEMA: &str = "molten.node.network-connectivity-probe-receipt.v1";
pub const NETWORK_PORT_MAPPING_SCHEMA: &str = "molten.node.network-port-mapping-receipt.v1";
pub const NETWORK_WATCHER_SNAPSHOT_SCHEMA: &str = "molten.node.network-watcher-snapshot.v1";
pub const METRICS_SNAPSHOT_SCHEMA: &str = "molten.node.metrics-snapshot-receipt.v1";
pub const EXTERNAL_DIAGNOSTICS_BRIDGE_SCHEMA: &str = "molten.node.external-diagnostics-bridge-receipt.v1";

const MIN_GENERATION: u64 = 1;
const MAX_GENERATION: u64 = u64::MAX - 1;
const MAX_ALPN_BYTES: usize = 64;
const MAX_HANDLER_KIND_BYTES: usize = 64;
const MAX_REF_COUNT: usize = 64;
const MAX_DIAGNOSTICS: usize = 64;
const MAX_SESSION_FRAMES: u64 = 4_096;
const MAX_FRAME_BYTES: u64 = 1_048_576;
const MIN_FRAME_BYTES: u64 = 1;
const MAX_SERVICE_ID_BYTES: usize = 128;
const MAX_OPERATION_ID_BYTES: usize = 128;
const MAX_NETWORK_OBSERVATIONS: usize = 64;
const MAX_METRIC_SAMPLES: usize = 256;
const MAX_METRIC_LABELS: usize = 16;
const MAX_METRIC_NAME_BYTES: usize = 128;
const MAX_METRIC_LABEL_BYTES: usize = 128;
const MAX_OPENMETRICS_BYTES: usize = 65_536;
const MAX_WATCHER_ITEMS: usize = 32;
const MAX_PORT_DURATION_SECONDS: u64 = 86_400;
const MIN_PORT_NUMBER: u64 = 1;
const MAX_PORT_NUMBER: u64 = 65_535;
const DEFAULT_ROUTER_DRAIN_POLICY: &str = "bounded-drain";
const DEFAULT_LIMIT_PROFILE: &str = "iroh-framed-envelope-default-v1";
const DEFAULT_NODE_CONTROL_OWNER: &str = "node-runtime";
const DEFAULT_NODE_CONTROL_HANDLER_PROFILE: &str = "node-control-v1";
const DEFAULT_NODE_CONTROL_SYMBOL: &str = "node-control";
const DEFAULT_NODE_CONTROL_LIFECYCLE: &str = "active";
const EVIDENCE_ONLY_CAVEAT: &str = "evidence-only: does not grant authority, policy, resource, provenance, source-gate, retention, transport-correctness, or deterministic replay trust";

const _: () = assert!(MIN_GENERATION > 0);
const _: () = assert!(MAX_ALPN_BYTES > 0);
const _: () = assert!(MAX_HANDLER_KIND_BYTES > 0);
const _: () = assert!(MAX_REF_COUNT > 0);
const _: () = assert!(MAX_DIAGNOSTICS > 0);
const _: () = assert!(MAX_SESSION_FRAMES > 0);
const _: () = assert!(MAX_FRAME_BYTES > MIN_FRAME_BYTES);
const _: () = assert!(MAX_METRIC_LABELS > 0);
const _: () = assert!(MAX_OPENMETRICS_BYTES > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolHandlerDescriptor {
    pub alpn: String,
    pub handler_kind: String,
    pub owner_namespace: String,
    pub handler_profile: String,
    pub generation: u64,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub drain_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProtocolRegistry {
    pub handlers: std::collections::BTreeMap<String, ProtocolHandlerDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterOperationInput {
    pub operation: String,
    pub alpn: String,
    pub handler_kind: String,
    pub owner_namespace: String,
    pub handler_profile: String,
    pub generation: u64,
    pub prior_generation: Option<u64>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub shutdown_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterDecision {
    pub decision: String,
    pub operation: String,
    pub alpn: String,
    pub outcome: String,
    pub generation: Option<u64>,
    pub previous_generation: Option<u64>,
    pub diagnostics: Vec<String>,
    pub registry: ProtocolRegistry,
    pub registry_entry_ref: Option<String>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramedEnvelopeLimits {
    pub max_frame_bytes: u64,
    pub max_frames_per_session: u64,
    pub max_outstanding_frames: u64,
}

impl Default for FramedEnvelopeLimits {
    fn default() -> Self {
        Self {
            max_frame_bytes: MAX_FRAME_BYTES,
            max_frames_per_session: MAX_SESSION_FRAMES,
            max_outstanding_frames: MAX_SESSION_FRAMES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramedEnvelopeInput {
    pub alpn: String,
    pub peer: String,
    pub node: String,
    pub stream_id: String,
    pub sequence: u64,
    pub declared_length: u64,
    pub declared_envelope_ref: String,
    pub envelope_bytes: Vec<u8>,
    pub limit_profile_ref: String,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub limits: FramedEnvelopeLimits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramedEnvelopeDecision {
    pub decision: String,
    pub alpn: String,
    pub peer: String,
    pub node: String,
    pub stream_id: String,
    pub sequence: u64,
    pub declared_envelope_ref: String,
    pub actual_envelope_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSessionInput {
    pub service_id: String,
    pub operation_id: String,
    pub interaction_kind: String,
    pub path_kind: String,
    pub request_ref: String,
    pub response_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub alpn: Option<String>,
    pub peer: Option<String>,
    pub node: Option<String>,
    pub stream_id: Option<String>,
    pub frame_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSessionDecision {
    pub decision: String,
    pub service_id: String,
    pub operation_id: String,
    pub interaction_kind: String,
    pub path_kind: String,
    pub diagnostics: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDiagnosticsInput {
    pub nat_class: String,
    pub udp_status: String,
    pub direct_path_status: String,
    pub relay_latency_ms: Option<u64>,
    pub port_map_protocols: Vec<String>,
    pub interface_refs: Vec<String>,
    pub route_refs: Vec<String>,
    pub live_observations_recorded: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DiagnosticLog {
    values: Vec<String>,
}

impl DiagnosticLog {
    fn new() -> Self {
        Self::default()
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    fn iter(&self) -> std::slice::Iter<'_, String> {
        self.values.iter()
    }

    fn into_values(self) -> Vec<String> {
        self.values
    }

    fn push(&mut self, diagnostic: impl Into<String>) -> crate::error::Result<()> {
        let next = self
            .values
            .len()
            .checked_add(1)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("diagnostic count overflow"))?;
        validate_bounded_value_count(next, MAX_DIAGNOSTICS, "diagnostic")?;
        self.values.push(diagnostic.into());
        Ok(())
    }
}

trait DiagnosticSink {
    fn push_bounded(&mut self, diagnostic: String) -> crate::error::Result<()>;
}

impl DiagnosticSink for DiagnosticLog {
    fn push_bounded(&mut self, diagnostic: String) -> crate::error::Result<()> {
        self.push(diagnostic)
    }
}

impl DiagnosticSink for Vec<String> {
    fn push_bounded(&mut self, diagnostic: String) -> crate::error::Result<()> {
        let next = self
            .len()
            .checked_add(1)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("diagnostic count overflow"))?;
        validate_bounded_value_count(next, MAX_DIAGNOSTICS, "diagnostic")?;
        self.push(diagnostic);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectivityProbeInput {
    pub source_node: String,
    pub target_node: String,
    pub expected_endpoint_ref: String,
    pub observed_endpoint_ref: Option<String>,
    pub direct_path_status: String,
    pub relay_path_status: String,
    pub timeout_ms: Option<u64>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortMappingInput {
    pub mode: String,
    pub requester_ref: Option<String>,
    pub identity_ref: Option<String>,
    pub protocol: String,
    pub external_port: Option<u64>,
    pub internal_port: Option<u64>,
    pub duration_seconds: Option<u64>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub operator_evidence_refs: Vec<String>,
    pub available_protocols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkWatcherInput {
    pub node: String,
    pub interface_state: String,
    pub address_state: String,
    pub default_route: String,
    pub relay_state: String,
    pub endpoint_state: String,
    pub observed_event_count: u64,
    pub retained_event_count: u64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricSample {
    pub name: String,
    pub kind: String,
    pub value: u64,
    pub labels: Vec<(String, String)>,
}
