use std::collections::BTreeMap;
use std::collections::BTreeSet;

use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::content_ref_from_bytes;
use crate::preserves_rail::parse_canonical_bytes;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;

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
    pub generation: u64,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub drain_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProtocolRegistry {
    pub handlers: BTreeMap<String, ProtocolHandlerDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterOperationInput {
    pub operation: String,
    pub alpn: String,
    pub handler_kind: String,
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
    pub receipt_value: IOValue,
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
    pub receipt_value: IOValue,
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
    pub receipt_value: IOValue,
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
    pub receipt_value: IOValue,
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
    pub node_identity_ref: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricsSnapshotInput {
    pub node: String,
    pub scrape_ref: String,
    pub policy_refs: Vec<String>,
    pub redaction_refs: Vec<String>,
    pub samples: Vec<MetricSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricsSnapshotDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub openmetrics: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalDiagnosticsBridgeInput {
    pub enabled: bool,
    pub mode: String,
    pub target_service_ref: Option<String>,
    pub capability_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub redaction_policy_refs: Vec<String>,
    pub api_secret_provenance_ref: Option<String>,
    pub operator_evidence_refs: Vec<String>,
    pub expiry_ref: Option<String>,
}

pub fn empty_protocol_registry() -> ProtocolRegistry {
    ProtocolRegistry::default()
}

pub fn evaluate_router_operation(registry: &ProtocolRegistry, input: &RouterOperationInput) -> Result<RouterDecision> {
    let mut diagnostics = Vec::new();
    let alpn_valid = collect_alpn_diagnostic(&input.alpn, &mut diagnostics).is_ok();
    let handler_valid = collect_handler_diagnostic(&input.handler_kind, &mut diagnostics).is_ok();
    collect_ref_diagnostics(&input.authority_refs, "authority", &mut diagnostics)?;
    collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
    collect_ref_diagnostics(&input.resource_refs, "resource", &mut diagnostics)?;
    collect_ref_diagnostics(&input.evidence_refs, "evidence", &mut diagnostics)?;
    if input.generation < MIN_GENERATION || input.generation > MAX_GENERATION {
        push_diagnostic(&mut diagnostics, format!("generation {} outside supported range", input.generation))?;
    }

    let has_admission = !input.authority_refs.is_empty()
        && !input.policy_refs.is_empty()
        && !input.resource_refs.is_empty()
        && !input.evidence_refs.is_empty();
    if !has_admission && input.operation != "unsupported-alpn" {
        push_diagnostic(&mut diagnostics, "router operation requires authority, policy, resource, and evidence refs")?;
    }

    let mut next = registry.clone();
    let mut outcome = "denied".to_string();
    let mut generation = None;
    let mut previous_generation = None;
    let existing = registry.handlers.get(&input.alpn);

    if alpn_valid && handler_valid && diagnostics.is_empty() {
        match input.operation.as_str() {
            "install" => match existing {
                None => {
                    let descriptor = descriptor_from_input(input)?;
                    generation = Some(descriptor.generation);
                    next.handlers.insert(input.alpn.clone(), descriptor);
                    outcome = "inserted".to_string();
                }
                Some(current) => {
                    previous_generation = Some(current.generation);
                    push_diagnostic(&mut diagnostics, "ALPN already registered; use replace with current generation")?;
                }
            },
            "replace" => match existing {
                Some(current) => {
                    previous_generation = Some(current.generation);
                    let expected = Some(current.generation);
                    if input.prior_generation != expected {
                        push_diagnostic(
                            &mut diagnostics,
                            "stale-generation: replacement prior generation does not match registry",
                        )?;
                    } else if input.generation <= current.generation {
                        push_diagnostic(&mut diagnostics, "replacement generation must advance")?;
                    } else if input.shutdown_evidence_ref.as_deref().is_none() {
                        push_diagnostic(
                            &mut diagnostics,
                            "replacement requires shutdown evidence for previous handler",
                        )?;
                    } else {
                        validate_optional_ref(input.shutdown_evidence_ref.as_deref(), "shutdown evidence")?;
                        let descriptor = descriptor_from_input(input)?;
                        generation = Some(descriptor.generation);
                        next.handlers.insert(input.alpn.clone(), descriptor);
                        outcome = "replaced".to_string();
                    }
                }
                None => push_diagnostic(&mut diagnostics, "cannot replace unknown ALPN")?,
            },
            "remove" => match existing {
                Some(current) => {
                    previous_generation = Some(current.generation);
                    let expected = Some(current.generation);
                    if input.prior_generation != expected {
                        push_diagnostic(
                            &mut diagnostics,
                            "stale-generation: remove prior generation does not match registry",
                        )?;
                    } else if input.shutdown_evidence_ref.as_deref().is_none() {
                        push_diagnostic(&mut diagnostics, "remove requires shutdown evidence for previous handler")?;
                    } else {
                        validate_optional_ref(input.shutdown_evidence_ref.as_deref(), "shutdown evidence")?;
                        next.handlers.remove(&input.alpn);
                        generation = Some(current.generation);
                        outcome = "removed".to_string();
                    }
                }
                None => push_diagnostic(&mut diagnostics, "cannot remove unknown ALPN")?,
            },
            "unsupported-alpn" => {
                if existing.is_none() {
                    outcome = "unsupported-alpn".to_string();
                    push_diagnostic(&mut diagnostics, "unsupported ALPN denied before frame delivery")?;
                } else {
                    push_diagnostic(&mut diagnostics, "ALPN is registered; unsupported-alpn denial is not applicable")?;
                }
            }
            other => push_diagnostic(&mut diagnostics, format!("unsupported router operation {other}"))?,
        }
    }

    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let receipt_value = router_receipt_value(RouterReceiptInput {
        decision: &decision,
        operation: &input.operation,
        outcome: &outcome,
        alpn: &input.alpn,
        handler_kind: &input.handler_kind,
        generation,
        previous_generation,
        authority_refs: &input.authority_refs,
        policy_refs: &input.policy_refs,
        resource_refs: &input.resource_refs,
        evidence_refs: &input.evidence_refs,
        shutdown_evidence_ref: input.shutdown_evidence_ref.as_deref(),
        diagnostics: &diagnostics,
    })?;
    Ok(RouterDecision {
        decision,
        operation: input.operation.clone(),
        alpn: input.alpn.clone(),
        outcome,
        generation,
        previous_generation,
        diagnostics,
        registry: next,
        receipt_value,
    })
}

pub fn evaluate_framed_envelope(
    registry: &ProtocolRegistry,
    input: &FramedEnvelopeInput,
) -> Result<FramedEnvelopeDecision> {
    let mut diagnostics = Vec::new();
    collect_alpn_diagnostic(&input.alpn, &mut diagnostics).ok();
    validate_text(&input.peer, "frame peer", &mut diagnostics)?;
    validate_text(&input.node, "frame node", &mut diagnostics)?;
    validate_text(&input.stream_id, "frame stream", &mut diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&input.limit_profile_ref), "limit profile", &mut diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&input.declared_envelope_ref), "declared envelope", &mut diagnostics)?;
    collect_ref_diagnostics(&input.authority_refs, "authority", &mut diagnostics)?;
    collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
    collect_ref_diagnostics(&input.resource_refs, "resource", &mut diagnostics)?;
    collect_ref_diagnostics(&input.evidence_refs, "evidence", &mut diagnostics)?;
    if !registry.handlers.contains_key(&input.alpn) {
        push_diagnostic(&mut diagnostics, "unsupported ALPN denied before payload delivery")?;
    }
    if input.declared_length > input.limits.max_frame_bytes {
        push_diagnostic(&mut diagnostics, "oversized frame denied before parsing payload")?;
    }
    if input.sequence >= input.limits.max_frames_per_session {
        push_diagnostic(&mut diagnostics, "frame sequence exceeds per-session limit")?;
    }
    if input.limits.max_frame_bytes < MIN_FRAME_BYTES || input.limits.max_frame_bytes > MAX_FRAME_BYTES {
        push_diagnostic(&mut diagnostics, "frame byte limit is outside supported bounds")?;
    }
    if input.limits.max_frames_per_session == 0 || input.limits.max_frames_per_session > MAX_SESSION_FRAMES {
        push_diagnostic(&mut diagnostics, "frame count limit is outside supported bounds")?;
    }

    let mut actual_ref = None;
    if !diagnostics.iter().any(|diagnostic| diagnostic.contains("oversized frame")) {
        let byte_len = input.envelope_bytes.len() as u64;
        if byte_len != input.declared_length {
            push_diagnostic(
                &mut diagnostics,
                format!("declared frame length {} does not match bytes {byte_len}", input.declared_length),
            )?;
        }
        let parsed = match parse_canonical_bytes(&input.envelope_bytes) {
            Ok(value) => value,
            Err(error) => {
                push_diagnostic(&mut diagnostics, format!("malformed Preserves frame: {error}"))?;
                record("invalid-frame", Vec::new())
            }
        };
        let encoded = canonical_bytes(&parsed).unwrap_or_default();
        if encoded != input.envelope_bytes && !input.envelope_bytes.is_empty() {
            push_diagnostic(&mut diagnostics, "frame payload is not canonical Preserves bytes")?;
        }
        let computed = content_ref_from_bytes(&input.envelope_bytes);
        if computed != input.declared_envelope_ref {
            push_diagnostic(
                &mut diagnostics,
                format!("declared envelope ref mismatch: got {}, expected {}", computed, input.declared_envelope_ref),
            )?;
        }
        actual_ref = Some(computed);
    }

    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let receipt_value = framed_receipt_value(FramedReceiptInput {
        decision: &decision,
        alpn: &input.alpn,
        peer: &input.peer,
        node: &input.node,
        stream_id: &input.stream_id,
        sequence: input.sequence,
        declared_length: input.declared_length,
        declared_envelope_ref: &input.declared_envelope_ref,
        actual_envelope_ref: actual_ref.as_deref(),
        limit_profile_ref: &input.limit_profile_ref,
        authority_refs: &input.authority_refs,
        policy_refs: &input.policy_refs,
        resource_refs: &input.resource_refs,
        evidence_refs: &input.evidence_refs,
        diagnostics: &diagnostics,
    })?;
    Ok(FramedEnvelopeDecision {
        decision,
        alpn: input.alpn.clone(),
        peer: input.peer.clone(),
        node: input.node.clone(),
        stream_id: input.stream_id.clone(),
        sequence: input.sequence,
        declared_envelope_ref: input.declared_envelope_ref.clone(),
        actual_envelope_ref: actual_ref,
        diagnostics,
        receipt_value,
    })
}

pub fn evaluate_service_session(input: &ServiceSessionInput) -> Result<ServiceSessionDecision> {
    let mut diagnostics = Vec::new();
    validate_bounded_text(&input.service_id, "service id", MAX_SERVICE_ID_BYTES, &mut diagnostics)?;
    validate_bounded_text(&input.operation_id, "operation id", MAX_OPERATION_ID_BYTES, &mut diagnostics)?;
    validate_interaction_kind(&input.interaction_kind, &mut diagnostics)?;
    validate_path_kind(&input.path_kind, &mut diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&input.request_ref), "request", &mut diagnostics)?;
    collect_ref_diagnostics(&input.response_refs, "response", &mut diagnostics)?;
    collect_ref_diagnostics(&input.capability_refs, "capability", &mut diagnostics)?;
    collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
    collect_ref_diagnostics(&input.resource_refs, "resource", &mut diagnostics)?;
    if input.path_kind == "remote" {
        if input.alpn.as_deref().is_none() || input.peer.as_deref().is_none() || input.node.as_deref().is_none() {
            push_diagnostic(&mut diagnostics, "remote session requires ALPN, peer, and node ids")?;
        }
        if input.frame_receipt_refs.is_empty() {
            push_diagnostic(&mut diagnostics, "remote session requires frame receipt refs")?;
        }
    }
    collect_ref_diagnostics(&input.frame_receipt_refs, "frame receipt", &mut diagnostics)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let receipt_value = service_session_receipt_value(input, &decision, &diagnostics)?;
    Ok(ServiceSessionDecision {
        decision,
        service_id: input.service_id.clone(),
        operation_id: input.operation_id.clone(),
        interaction_kind: input.interaction_kind.clone(),
        path_kind: input.path_kind.clone(),
        diagnostics,
        receipt_value,
    })
}

pub fn network_diagnostics_report(input: &NetworkDiagnosticsInput) -> Result<DiagnosticDecision> {
    let mut diagnostics = input.diagnostics.clone();
    ensure_string_count(&diagnostics, MAX_DIAGNOSTICS, "network diagnostics")?;
    validate_status(&input.udp_status, &["pass", "deny", "degraded", "unavailable"], "UDP status")?;
    validate_status(
        &input.direct_path_status,
        &["pass", "deny", "degraded", "relay-only", "unavailable"],
        "direct path status",
    )?;
    validate_bounded_value_count(input.port_map_protocols.len(), MAX_NETWORK_OBSERVATIONS, "port map protocol")?;
    collect_ref_diagnostics(&input.interface_refs, "interface snapshot", &mut diagnostics)?;
    collect_ref_diagnostics(&input.route_refs, "route snapshot", &mut diagnostics)?;
    if !input.live_observations_recorded {
        push_diagnostic(&mut diagnostics, "live-only observations are non-replayable diagnostics")?;
    }
    let decision = if input.udp_status == "deny" || input.direct_path_status == "deny" {
        "deny"
    } else if diagnostics.is_empty() && input.udp_status == "pass" && input.direct_path_status == "pass" {
        "pass"
    } else {
        "degraded"
    }
    .to_string();
    let receipt_value = record("network-diagnostics-report-v1", vec![
        string(NETWORK_DIAGNOSTICS_REPORT_SCHEMA),
        record("decision", vec![string(&decision)]),
        record("nat", vec![string(&input.nat_class)]),
        record("udp", vec![string(&input.udp_status)]),
        record("direct-path", vec![string(&input.direct_path_status)]),
        record("relay-latency-ms", vec![optional_u64_value(input.relay_latency_ms)]),
        record("port-map-protocols", vec![sequence(input.port_map_protocols.iter().map(string).collect())]),
        record("interfaces", vec![refs_value(&input.interface_refs)?]),
        record("routes", vec![refs_value(&input.route_refs)?]),
        record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("diagnostics-evidence-only", "pass"),
            ("live-observations-recorded", pass_fail(input.live_observations_recorded)),
            ("no-transport-derived-authority", "pass"),
        ]),
        record("caveat", vec![string(EVIDENCE_ONLY_CAVEAT)]),
    ]);
    Ok(DiagnosticDecision {
        decision,
        diagnostics,
        receipt_value,
    })
}

pub fn connectivity_probe_receipt(input: &ConnectivityProbeInput) -> Result<DiagnosticDecision> {
    let mut diagnostics = Vec::new();
    validate_text(&input.source_node, "source node", &mut diagnostics)?;
    validate_text(&input.target_node, "target node", &mut diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&input.expected_endpoint_ref), "expected endpoint", &mut diagnostics)?;
    if let Some(observed) = &input.observed_endpoint_ref {
        collect_ref_diagnostics(std::slice::from_ref(observed), "observed endpoint", &mut diagnostics)?;
        if observed != &input.expected_endpoint_ref {
            push_diagnostic(&mut diagnostics, "observed endpoint identity does not match expected endpoint")?;
        }
    } else {
        push_diagnostic(&mut diagnostics, "no observed endpoint identity")?;
    }
    collect_ref_diagnostics(&input.authority_refs, "authority", &mut diagnostics)?;
    collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
    collect_ref_diagnostics(&input.resource_refs, "resource", &mut diagnostics)?;
    collect_ref_diagnostics(&input.evidence_refs, "evidence", &mut diagnostics)?;
    let path_status = if input.direct_path_status == "pass" {
        "direct"
    } else if input.relay_path_status == "pass" {
        push_diagnostic(&mut diagnostics, "relay-only diagnostic path; direct path did not pass")?;
        "relay-only"
    } else if input.timeout_ms.is_some() {
        push_diagnostic(&mut diagnostics, "connectivity probe timed out")?;
        "timeout"
    } else {
        push_diagnostic(&mut diagnostics, "connectivity probe did not find a passing path")?;
        "deny"
    };
    let decision = if diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("identity") || diagnostic.contains("timed out") || diagnostic.contains("did not find")
    }) {
        "deny"
    } else if path_status == "relay-only" {
        "degraded"
    } else {
        "pass"
    }
    .to_string();
    let receipt_value = record("network-connectivity-probe-receipt-v1", vec![
        string(NETWORK_CONNECTIVITY_PROBE_SCHEMA),
        record("decision", vec![string(&decision)]),
        record("source", vec![string(&input.source_node)]),
        record("target", vec![string(&input.target_node)]),
        record("path", vec![string(path_status)]),
        record("expected-endpoint", vec![string(&input.expected_endpoint_ref)]),
        record("observed-endpoint", vec![optional_string_value(input.observed_endpoint_ref.as_deref())]),
        record("authority", vec![refs_value(&input.authority_refs)?]),
        record("policy", vec![refs_value(&input.policy_refs)?]),
        record("resource", vec![refs_value(&input.resource_refs)?]),
        record("evidence", vec![refs_value(&input.evidence_refs)?]),
        record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("connectivity-diagnostic-only", "pass"),
            ("no-state-mutation", pass_fail(decision != "pass" || !input.evidence_refs.is_empty())),
            ("transport-does-not-grant-authority", "pass"),
        ]),
    ]);
    Ok(DiagnosticDecision {
        decision,
        diagnostics,
        receipt_value,
    })
}

pub fn port_mapping_receipt(input: &PortMappingInput) -> Result<DiagnosticDecision> {
    let mut diagnostics = Vec::new();
    validate_status(&input.mode, &["probe", "mutate"], "port mapping mode")?;
    validate_bounded_value_count(input.available_protocols.len(), MAX_NETWORK_OBSERVATIONS, "available protocol")?;
    let protocol_available = input.available_protocols.iter().any(|protocol| protocol == &input.protocol);
    if !protocol_available {
        push_diagnostic(&mut diagnostics, "requested port mapping protocol unavailable")?;
    }
    if input.mode == "mutate" {
        collect_required_optional_ref(input.requester_ref.as_deref(), "requester", &mut diagnostics)?;
        collect_required_optional_ref(input.node_identity_ref.as_deref(), "node identity", &mut diagnostics)?;
        collect_ref_diagnostics(&input.authority_refs, "authority", &mut diagnostics)?;
        collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
        collect_ref_diagnostics(&input.resource_refs, "resource", &mut diagnostics)?;
        collect_ref_diagnostics(&input.operator_evidence_refs, "operator evidence", &mut diagnostics)?;
        validate_port(input.external_port, "external port", &mut diagnostics)?;
        validate_port(input.internal_port, "internal port", &mut diagnostics)?;
        match input.duration_seconds {
            Some(duration) if duration <= MAX_PORT_DURATION_SECONDS => {}
            Some(_) => push_diagnostic(&mut diagnostics, "port mapping duration exceeds bound")?,
            None => push_diagnostic(&mut diagnostics, "port mapping mutation requires duration")?,
        }
    }
    let decision = if diagnostics.is_empty() {
        "pass"
    } else if input.mode == "probe" {
        "degraded"
    } else {
        "deny"
    }
    .to_string();
    let receipt_value = record("network-port-mapping-receipt-v1", vec![
        string(NETWORK_PORT_MAPPING_SCHEMA),
        record("decision", vec![string(&decision)]),
        record("mode", vec![string(&input.mode)]),
        record("protocol", vec![string(&input.protocol)]),
        record("requester", vec![optional_string_value(input.requester_ref.as_deref())]),
        record("node", vec![optional_string_value(input.node_identity_ref.as_deref())]),
        record("external-port", vec![optional_u64_value(input.external_port)]),
        record("internal-port", vec![optional_u64_value(input.internal_port)]),
        record("duration-seconds", vec![optional_u64_value(input.duration_seconds)]),
        record("available-protocols", vec![sequence(input.available_protocols.iter().map(string).collect())]),
        record("authority", vec![refs_value(&input.authority_refs)?]),
        record("policy", vec![refs_value(&input.policy_refs)?]),
        record("resource", vec![refs_value(&input.resource_refs)?]),
        record("operator-evidence", vec![refs_value(&input.operator_evidence_refs)?]),
        record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("probe-does-not-mutate", pass_fail(input.mode == "probe")),
            ("mutation-deny-by-default", pass_fail(input.mode != "mutate" || decision == "pass")),
            ("authority-policy-resource-explicit", pass_fail(input.mode != "mutate" || diagnostics.is_empty())),
        ]),
    ]);
    Ok(DiagnosticDecision {
        decision,
        diagnostics,
        receipt_value,
    })
}

pub fn watcher_snapshot_value(input: &NetworkWatcherInput) -> Result<DiagnosticDecision> {
    let mut diagnostics = Vec::new();
    validate_text(&input.node, "watcher node", &mut diagnostics)?;
    validate_text(&input.interface_state, "interface state", &mut diagnostics)?;
    validate_text(&input.address_state, "address state", &mut diagnostics)?;
    validate_text(&input.default_route, "default route", &mut diagnostics)?;
    validate_text(&input.relay_state, "relay state", &mut diagnostics)?;
    validate_text(&input.endpoint_state, "endpoint state", &mut diagnostics)?;
    if input.retained_event_count > input.observed_event_count {
        push_diagnostic(&mut diagnostics, "retained watcher event count exceeds observed event count")?;
    }
    if input.retained_event_count as usize > MAX_WATCHER_ITEMS {
        push_diagnostic(&mut diagnostics, "retained watcher events exceed latest-state bound")?;
    }
    collect_ref_diagnostics(&input.evidence_refs, "watcher evidence", &mut diagnostics)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "degraded" }.to_string();
    let receipt_value = record("network-watcher-snapshot-v1", vec![
        string(NETWORK_WATCHER_SNAPSHOT_SCHEMA),
        record("decision", vec![string(&decision)]),
        record("node", vec![string(&input.node)]),
        record("interface", vec![string(&input.interface_state)]),
        record("address", vec![string(&input.address_state)]),
        record("default-route", vec![string(&input.default_route)]),
        record("relay", vec![string(&input.relay_state)]),
        record("endpoint", vec![string(&input.endpoint_state)]),
        record("observed-events", vec![u64_value(input.observed_event_count)]),
        record("retained-events", vec![u64_value(input.retained_event_count)]),
        record("evidence", vec![refs_value(&input.evidence_refs)?]),
        record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("latest-state-only", "pass"),
            ("bounded-event-buffer", pass_fail(input.retained_event_count as usize <= MAX_WATCHER_ITEMS)),
            ("watcher-diagnostic-only", "pass"),
        ]),
    ]);
    Ok(DiagnosticDecision {
        decision,
        diagnostics,
        receipt_value,
    })
}

pub fn metrics_snapshot(input: &MetricsSnapshotInput) -> Result<MetricsSnapshotDecision> {
    let mut diagnostics = Vec::new();
    validate_text(&input.node, "metrics node", &mut diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&input.scrape_ref), "scrape", &mut diagnostics)?;
    collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
    collect_ref_diagnostics(&input.redaction_refs, "redaction", &mut diagnostics)?;
    validate_bounded_value_count(input.samples.len(), MAX_METRIC_SAMPLES, "metric sample")?;
    let openmetrics = render_openmetrics(input, &mut diagnostics)?;
    if openmetrics.len() > MAX_OPENMETRICS_BYTES {
        push_diagnostic(&mut diagnostics, "OpenMetrics snapshot exceeds byte bound")?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let metric_refs = input
        .samples
        .iter()
        .map(|sample| record("metric", vec![string(&sample.name), string(&sample.kind), u64_value(sample.value)]))
        .collect();
    let receipt_value = record("metrics-snapshot-receipt-v1", vec![
        string(METRICS_SNAPSHOT_SCHEMA),
        record("decision", vec![string(&decision)]),
        record("node", vec![string(&input.node)]),
        record("scrape", vec![string(&input.scrape_ref)]),
        record("policy", vec![refs_value(&input.policy_refs)?]),
        record("redaction", vec![refs_value(&input.redaction_refs)?]),
        record("metrics", vec![sequence(metric_refs)]),
        record("openmetrics-ref", vec![string(content_ref_from_bytes(openmetrics.as_bytes()))]),
        record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("labels-bounded", pass_fail(diagnostics.is_empty())),
            ("labels-redacted", pass_fail(diagnostics.is_empty())),
            ("metrics-do-not-grant-admission", "pass"),
        ]),
    ]);
    Ok(MetricsSnapshotDecision {
        decision,
        diagnostics,
        openmetrics,
        receipt_value,
    })
}

pub fn external_diagnostics_bridge_receipt(input: &ExternalDiagnosticsBridgeInput) -> Result<DiagnosticDecision> {
    let mut diagnostics = Vec::new();
    validate_status(&input.mode, &["push", "remote-request"], "external diagnostics mode")?;
    if !input.enabled {
        push_diagnostic(&mut diagnostics, "external diagnostics bridge disabled by default")?;
    } else {
        collect_required_optional_ref(input.target_service_ref.as_deref(), "target service", &mut diagnostics)?;
        collect_required_optional_ref(
            input.api_secret_provenance_ref.as_deref(),
            "api secret provenance",
            &mut diagnostics,
        )?;
        collect_required_optional_ref(input.expiry_ref.as_deref(), "bridge expiry", &mut diagnostics)?;
        collect_ref_diagnostics(&input.capability_refs, "capability", &mut diagnostics)?;
        collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
        collect_ref_diagnostics(&input.redaction_policy_refs, "redaction policy", &mut diagnostics)?;
        collect_ref_diagnostics(&input.operator_evidence_refs, "operator evidence", &mut diagnostics)?;
    }
    let decision = if input.enabled && diagnostics.is_empty() {
        "pass"
    } else {
        "deny"
    }
    .to_string();
    let receipt_value = record("external-diagnostics-bridge-receipt-v1", vec![
        string(EXTERNAL_DIAGNOSTICS_BRIDGE_SCHEMA),
        record("decision", vec![string(&decision)]),
        record("mode", vec![string(&input.mode)]),
        record("enabled", vec![string(if input.enabled { "true" } else { "false" })]),
        record("target-service", vec![optional_string_value(input.target_service_ref.as_deref())]),
        record("capability", vec![refs_value(&input.capability_refs)?]),
        record("policy", vec![refs_value(&input.policy_refs)?]),
        record("redaction-policy", vec![refs_value(&input.redaction_policy_refs)?]),
        record("api-secret-provenance", vec![optional_string_value(input.api_secret_provenance_ref.as_deref())]),
        record("operator-evidence", vec![refs_value(&input.operator_evidence_refs)?]),
        record("expiry", vec![optional_string_value(input.expiry_ref.as_deref())]),
        record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("disabled-by-default", pass_fail(!input.enabled || decision == "pass")),
            ("secret-redacted", "pass"),
            ("remote-requests-still-router-admitted", "pass"),
        ]),
    ]);
    Ok(DiagnosticDecision {
        decision,
        diagnostics,
        receipt_value,
    })
}

struct RouterReceiptInput<'a> {
    decision: &'a str,
    operation: &'a str,
    outcome: &'a str,
    alpn: &'a str,
    handler_kind: &'a str,
    generation: Option<u64>,
    previous_generation: Option<u64>,
    authority_refs: &'a [String],
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    evidence_refs: &'a [String],
    shutdown_evidence_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

fn router_receipt_value(input: RouterReceiptInput<'_>) -> Result<IOValue> {
    Ok(record("iroh-protocol-router-receipt-v1", vec![
        string(IROH_PROTOCOL_ROUTER_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("operation", vec![string(input.operation)]),
        record("outcome", vec![string(input.outcome)]),
        record("alpn", vec![string(input.alpn)]),
        record("handler", vec![string(input.handler_kind)]),
        record("generation", vec![optional_u64_value(input.generation)]),
        record("previous-generation", vec![optional_u64_value(input.previous_generation)]),
        record("authority", vec![refs_value(input.authority_refs)?]),
        record("policy", vec![refs_value(input.policy_refs)?]),
        record("resource", vec![refs_value(input.resource_refs)?]),
        record("evidence", vec![refs_value(input.evidence_refs)?]),
        record("shutdown-evidence", vec![optional_string_value(input.shutdown_evidence_ref)]),
        record("diagnostics", vec![strings_value(input.diagnostics)?]),
        checks_value(&[
            ("authority-policy-resource-explicit", pass_fail(input.decision == "pass")),
            ("generationed-router", "pass"),
            ("unsupported-alpn-denies-before-delivery", pass_fail(input.outcome != "unsupported-alpn")),
            ("transport-evidence-only", "pass"),
        ]),
    ]))
}

struct FramedReceiptInput<'a> {
    decision: &'a str,
    alpn: &'a str,
    peer: &'a str,
    node: &'a str,
    stream_id: &'a str,
    sequence: u64,
    declared_length: u64,
    declared_envelope_ref: &'a str,
    actual_envelope_ref: Option<&'a str>,
    limit_profile_ref: &'a str,
    authority_refs: &'a [String],
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    evidence_refs: &'a [String],
    diagnostics: &'a [String],
}

fn framed_receipt_value(input: FramedReceiptInput<'_>) -> Result<IOValue> {
    Ok(record("iroh-framed-envelope-receipt-v1", vec![
        string(IROH_FRAMED_ENVELOPE_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("alpn", vec![string(input.alpn)]),
        record("peer", vec![string(input.peer)]),
        record("node", vec![string(input.node)]),
        record("stream", vec![string(input.stream_id)]),
        record("sequence", vec![u64_value(input.sequence)]),
        record("length", vec![u64_value(input.declared_length)]),
        record("declared-envelope", vec![string(input.declared_envelope_ref)]),
        record("actual-envelope", vec![optional_string_value(input.actual_envelope_ref)]),
        record("limit-profile", vec![string(input.limit_profile_ref)]),
        record("authority", vec![refs_value(input.authority_refs)?]),
        record("policy", vec![refs_value(input.policy_refs)?]),
        record("resource", vec![refs_value(input.resource_refs)?]),
        record("evidence", vec![refs_value(input.evidence_refs)?]),
        record("diagnostics", vec![strings_value(input.diagnostics)?]),
        checks_value(&[
            ("canonical-preserves-frame", pass_fail(input.decision == "pass")),
            ("frame-limits-bound", pass_fail(input.decision == "pass")),
            ("declared-ref-matches-actual", pass_fail(input.decision == "pass")),
            ("transport-evidence-only", "pass"),
        ]),
    ]))
}

fn service_session_receipt_value(
    input: &ServiceSessionInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IOValue> {
    Ok(record("iroh-stream-session-receipt-v1", vec![
        string(IROH_STREAM_SESSION_SCHEMA),
        record("decision", vec![string(decision)]),
        record("service", vec![string(&input.service_id)]),
        record("operation", vec![string(&input.operation_id)]),
        record("interaction", vec![string(&input.interaction_kind)]),
        record("path", vec![string(&input.path_kind)]),
        record("request", vec![string(&input.request_ref)]),
        record("responses", vec![refs_value(&input.response_refs)?]),
        record("capability", vec![refs_value(&input.capability_refs)?]),
        record("policy", vec![refs_value(&input.policy_refs)?]),
        record("resource", vec![refs_value(&input.resource_refs)?]),
        record("alpn", vec![optional_string_value(input.alpn.as_deref())]),
        record("peer", vec![optional_string_value(input.peer.as_deref())]),
        record("node", vec![optional_string_value(input.node.as_deref())]),
        record("stream", vec![optional_string_value(input.stream_id.as_deref())]),
        record("frames", vec![refs_value(&input.frame_receipt_refs)?]),
        record("diagnostics", vec![strings_value(diagnostics)?]),
        checks_value(&[
            ("canonical-local-remote-model", pass_fail(decision == "pass")),
            (
                "remote-frames-bound",
                pass_fail(input.path_kind != "remote" || !input.frame_receipt_refs.is_empty()),
            ),
            ("postcard-not-canonical-boundary", "pass"),
        ]),
    ]))
}

fn descriptor_from_input(input: &RouterOperationInput) -> Result<ProtocolHandlerDescriptor> {
    Ok(ProtocolHandlerDescriptor {
        alpn: input.alpn.clone(),
        handler_kind: input.handler_kind.clone(),
        generation: input.generation,
        authority_refs: input.authority_refs.clone(),
        policy_refs: input.policy_refs.clone(),
        resource_refs: input.resource_refs.clone(),
        evidence_refs: input.evidence_refs.clone(),
        drain_policy: DEFAULT_ROUTER_DRAIN_POLICY.to_string(),
    })
}

fn collect_alpn_diagnostic(alpn: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_bounded_text(alpn, "ALPN", MAX_ALPN_BYTES, diagnostics)?;
    if alpn.bytes().all(is_alpn_byte) {
        Ok(())
    } else {
        push_diagnostic(diagnostics, "ALPN must use visible ASCII without spaces")
    }
}

fn collect_handler_diagnostic(handler: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_bounded_text(handler, "handler kind", MAX_HANDLER_KIND_BYTES, diagnostics)
}

fn validate_interaction_kind(kind: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    if matches!(kind, "unary" | "server-streaming" | "client-streaming" | "bidirectional-streaming") {
        Ok(())
    } else {
        push_diagnostic(diagnostics, format!("unsupported service interaction kind {kind}"))
    }
}

fn validate_path_kind(kind: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    if matches!(kind, "local" | "remote") {
        Ok(())
    } else {
        push_diagnostic(diagnostics, format!("unsupported service session path {kind}"))
    }
}

fn validate_status(value: &str, allowed: &[&str], label: &str) -> Result<()> {
    if allowed.iter().any(|allowed| allowed == &value) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {label} {value}")))
    }
}

fn validate_port(value: Option<u64>, label: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    match value {
        Some(port) if (MIN_PORT_NUMBER..=MAX_PORT_NUMBER).contains(&port) => Ok(()),
        Some(port) => push_diagnostic(diagnostics, format!("{label} {port} outside valid port range")),
        None => push_diagnostic(diagnostics, format!("{label} is required")),
    }
}

fn collect_required_optional_ref(value: Option<&str>, label: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    match value {
        Some(reference) => collect_ref_diagnostics(&[reference.to_string()], label, diagnostics),
        None => push_diagnostic(diagnostics, format!("{label} ref is required")),
    }
}

fn collect_ref_diagnostics(refs: &[String], label: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_bounded_value_count(refs.len(), MAX_REF_COUNT, label)?;
    for reference in refs {
        if let Err(error) = validate_content_ref(reference) {
            push_diagnostic(diagnostics, format!("invalid {label} ref {reference}: {error}"))?;
        }
    }
    Ok(())
}

fn validate_optional_ref(value: Option<&str>, label: &str) -> Result<()> {
    if let Some(reference) = value {
        validate_content_ref(reference)
            .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}")))?;
    }
    Ok(())
}

fn validate_bounded_text(value: &str, label: &str, maximum: usize, diagnostics: &mut Vec<String>) -> Result<()> {
    if value.trim().is_empty() {
        return push_diagnostic(diagnostics, format!("{label} must not be empty"));
    }
    if value.len() > maximum {
        return push_diagnostic(diagnostics, format!("{label} length {} exceeds bound {maximum}", value.len()));
    }
    Ok(())
}

fn validate_text(value: &str, label: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_bounded_text(value, label, MAX_SERVICE_ID_BYTES, diagnostics)
}

fn validate_bounded_value_count(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
    }
}

fn ensure_string_count(values: &[String], maximum: usize, label: &str) -> Result<()> {
    validate_bounded_value_count(values.len(), maximum, label)
}

fn push_diagnostic(diagnostics: &mut Vec<String>, diagnostic: impl Into<String>) -> Result<()> {
    let next = diagnostics
        .len()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("diagnostic count overflow"))?;
    validate_bounded_value_count(next, MAX_DIAGNOSTICS, "diagnostic")?;
    diagnostics.push(diagnostic.into());
    Ok(())
}

fn is_alpn_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/')
}

fn refs_value(refs: &[String]) -> Result<IOValue> {
    validate_bounded_value_count(refs.len(), MAX_REF_COUNT, "ref")?;
    Ok(sequence(refs.iter().map(string).collect()))
}

fn strings_value(values: &[String]) -> Result<IOValue> {
    ensure_string_count(values, MAX_DIAGNOSTICS, "string")?;
    Ok(sequence(values.iter().map(string).collect()))
}

fn optional_string_value(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn optional_u64_value(value: Option<u64>) -> IOValue {
    match value {
        Some(value) => record("some", vec![u64_value(value)]),
        None => record("none", Vec::new()),
    }
}

fn checks_value(checks: &[(&'static str, &'static str)]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn pass_fail(is_pass: bool) -> &'static str {
    if is_pass { "pass" } else { "fail" }
}

fn render_openmetrics(input: &MetricsSnapshotInput, diagnostics: &mut Vec<String>) -> Result<String> {
    let mut output = String::new();
    let mut names = BTreeSet::new();
    for sample in &input.samples {
        validate_metric_sample(sample, diagnostics)?;
        names.insert(sample.name.as_str());
        output.push_str("# TYPE ");
        output.push_str(&sample.name);
        output.push(' ');
        output.push_str(&sample.kind);
        output.push('\n');
        output.push_str(&sample.name);
        if !sample.labels.is_empty() {
            output.push('{');
            for (index, (key, value)) in sample.labels.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(key);
                output.push_str("=\"");
                output.push_str(value);
                output.push('"');
            }
            output.push('}');
        }
        output.push(' ');
        output.push_str(&sample.value.to_string());
        output.push('\n');
    }
    if names.is_empty() {
        push_diagnostic(diagnostics, "metrics snapshot requires at least one sample")?;
    }
    Ok(output)
}

fn validate_metric_sample(sample: &MetricSample, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_bounded_text(&sample.name, "metric name", MAX_METRIC_NAME_BYTES, diagnostics)?;
    if !sample.name.bytes().all(is_metric_name_byte) {
        push_diagnostic(diagnostics, "metric name contains unsupported characters")?;
    }
    validate_status(&sample.kind, &["counter", "gauge", "histogram"], "metric kind")?;
    validate_bounded_value_count(sample.labels.len(), MAX_METRIC_LABELS, "metric label")?;
    for (key, value) in &sample.labels {
        validate_bounded_text(key, "metric label key", MAX_METRIC_LABEL_BYTES, diagnostics)?;
        validate_bounded_text(value, "metric label value", MAX_METRIC_LABEL_BYTES, diagnostics)?;
        if label_leaks_sensitive_value(key, value) {
            push_diagnostic(diagnostics, format!("metric label {key} leaks sensitive or high-cardinality data"))?;
        }
    }
    Ok(())
}

fn is_metric_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':')
}

fn label_leaks_sensitive_value(key: &str, value: &str) -> bool {
    let key_lower = key.to_ascii_lowercase();
    let value_lower = value.to_ascii_lowercase();
    key_lower.contains("secret")
        || key_lower.contains("ticket")
        || key_lower.contains("path")
        || key_lower.contains("peer_id")
        || value_lower.contains("secret")
        || value_lower.contains("ticket")
        || value_lower.starts_with("/home/")
        || value_lower.starts_with("blake3:")
}

pub fn fixture_ref(label: &str) -> String {
    content_ref_from_bytes(label.as_bytes())
}

pub fn default_limit_profile_ref() -> String {
    fixture_ref(DEFAULT_LIMIT_PROFILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preserves_rail::to_text;

    const ROUTER_GENERATION_ONE: u64 = 1;
    const ROUTER_GENERATION_TWO: u64 = 2;
    const VALID_FRAME_SEQUENCE: u64 = 0;
    const VALID_FRAME_LIMIT: u64 = 1024;
    const OVERSIZED_FRAME_LENGTH: u64 = 1025;
    const RELAY_LATENCY_MS: u64 = 42;
    const WATCHER_OBSERVED_EVENTS: u64 = 7;
    const WATCHER_RETAINED_EVENTS: u64 = 1;
    const METRIC_VALUE: u64 = 3;
    const PORT_DURATION_SECONDS: u64 = 600;
    const EXTERNAL_PORT: u64 = 443;
    const INTERNAL_PORT: u64 = 8443;

    fn refs() -> Vec<String> {
        vec![fixture_ref("ref")]
    }

    fn router_input(operation: &str, generation: u64) -> RouterOperationInput {
        RouterOperationInput {
            operation: operation.to_string(),
            alpn: "molten/node-control/1".to_string(),
            handler_kind: "node-control".to_string(),
            generation,
            prior_generation: None,
            authority_refs: refs(),
            policy_refs: refs(),
            resource_refs: refs(),
            evidence_refs: refs(),
            shutdown_evidence_ref: None,
        }
    }

    fn installed_registry() -> ProtocolRegistry {
        evaluate_router_operation(&empty_protocol_registry(), &router_input("install", ROUTER_GENERATION_ONE))
            .expect("install")
            .registry
    }

    #[test]
    fn router_installs_replaces_removes_and_denies_unsupported_alpn() {
        let registry = empty_protocol_registry();
        let install =
            evaluate_router_operation(&registry, &router_input("install", ROUTER_GENERATION_ONE)).expect("install");
        assert_eq!(install.decision, "pass");
        assert!(install.registry.handlers.contains_key("molten/node-control/1"));

        let mut replace_input = router_input("replace", ROUTER_GENERATION_TWO);
        replace_input.prior_generation = Some(ROUTER_GENERATION_ONE);
        replace_input.shutdown_evidence_ref = Some(fixture_ref("shutdown"));
        let replace = evaluate_router_operation(&install.registry, &replace_input).expect("replace");
        assert_eq!(replace.outcome, "replaced");

        let mut remove_input = router_input("remove", ROUTER_GENERATION_TWO);
        remove_input.prior_generation = Some(ROUTER_GENERATION_TWO);
        remove_input.shutdown_evidence_ref = Some(fixture_ref("shutdown-two"));
        let remove = evaluate_router_operation(&replace.registry, &remove_input).expect("remove");
        assert_eq!(remove.outcome, "removed");
        assert!(remove.registry.handlers.is_empty());

        let unsupported = evaluate_router_operation(&remove.registry, &RouterOperationInput {
            operation: "unsupported-alpn".to_string(),
            authority_refs: Vec::new(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            ..router_input("unsupported-alpn", ROUTER_GENERATION_ONE)
        })
        .expect("unsupported");
        assert_eq!(unsupported.decision, "deny");
        assert!(unsupported.diagnostics.iter().any(|diagnostic| diagnostic.contains("unsupported ALPN")));
    }

    #[test]
    fn stale_router_generation_denies_without_mutation() {
        let registry = installed_registry();
        let mut replace_input = router_input("replace", ROUTER_GENERATION_TWO);
        replace_input.prior_generation = Some(ROUTER_GENERATION_TWO);
        replace_input.shutdown_evidence_ref = Some(fixture_ref("shutdown"));
        let denied = evaluate_router_operation(&registry, &replace_input).expect("stale deny");
        assert_eq!(denied.decision, "deny");
        assert_eq!(denied.registry, registry);
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale-generation")));
    }

    fn framed_input(registry: &ProtocolRegistry) -> FramedEnvelopeInput {
        let envelope = record("node-control-envelope", vec![string("status")]);
        let bytes = canonical_bytes(&envelope).expect("bytes");
        let declared = content_ref_from_bytes(&bytes);
        assert!(registry.handlers.contains_key("molten/node-control/1"));
        FramedEnvelopeInput {
            alpn: "molten/node-control/1".to_string(),
            peer: "peer-a".to_string(),
            node: "node-b".to_string(),
            stream_id: "stream-1".to_string(),
            sequence: VALID_FRAME_SEQUENCE,
            declared_length: bytes.len() as u64,
            declared_envelope_ref: declared,
            envelope_bytes: bytes,
            limit_profile_ref: default_limit_profile_ref(),
            authority_refs: refs(),
            policy_refs: refs(),
            resource_refs: refs(),
            evidence_refs: refs(),
            limits: FramedEnvelopeLimits {
                max_frame_bytes: VALID_FRAME_LIMIT,
                max_frames_per_session: MAX_SESSION_FRAMES,
                max_outstanding_frames: MAX_SESSION_FRAMES,
            },
        }
    }

    #[test]
    fn framed_envelope_passes_for_canonical_preserves_and_denies_bad_refs() {
        let registry = installed_registry();
        let input = framed_input(&registry);
        let pass = evaluate_framed_envelope(&registry, &input).expect("frame pass");
        assert_eq!(pass.decision, "pass");
        assert_eq!(pass.actual_envelope_ref.as_ref(), Some(&input.declared_envelope_ref));

        let bad = FramedEnvelopeInput {
            declared_envelope_ref: fixture_ref("wrong"),
            ..input
        };
        let deny = evaluate_framed_envelope(&registry, &bad).expect("frame deny");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("mismatch")));
    }

    #[test]
    fn framed_envelope_denies_oversized_before_payload_parse() {
        let registry = installed_registry();
        let input = FramedEnvelopeInput {
            declared_length: OVERSIZED_FRAME_LENGTH,
            limits: FramedEnvelopeLimits {
                max_frame_bytes: VALID_FRAME_LIMIT,
                max_frames_per_session: MAX_SESSION_FRAMES,
                max_outstanding_frames: MAX_SESSION_FRAMES,
            },
            envelope_bytes: b"not preserves".to_vec(),
            ..framed_input(&registry)
        };
        let deny = evaluate_framed_envelope(&registry, &input).expect("oversized deny");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("oversized frame")));
        assert!(deny.actual_envelope_ref.is_none());
    }

    #[test]
    fn service_session_uses_same_model_for_local_and_remote() {
        let local = ServiceSessionInput {
            service_id: "node-control".to_string(),
            operation_id: "status".to_string(),
            interaction_kind: "unary".to_string(),
            path_kind: "local".to_string(),
            request_ref: fixture_ref("request"),
            response_refs: refs(),
            capability_refs: refs(),
            policy_refs: refs(),
            resource_refs: refs(),
            alpn: None,
            peer: None,
            node: None,
            stream_id: None,
            frame_receipt_refs: Vec::new(),
        };
        let local_decision = evaluate_service_session(&local).expect("local");
        assert_eq!(local_decision.decision, "pass");

        let remote = ServiceSessionInput {
            path_kind: "remote".to_string(),
            alpn: Some("molten/node-control/1".to_string()),
            peer: Some("peer-a".to_string()),
            node: Some("node-b".to_string()),
            stream_id: Some("stream-1".to_string()),
            frame_receipt_refs: refs(),
            ..local
        };
        let remote_decision = evaluate_service_session(&remote).expect("remote");
        assert_eq!(remote_decision.decision, "pass");
        let text = to_text(&remote_decision.receipt_value).expect("text");
        assert!(text.contains("postcard-not-canonical-boundary"));
    }

    #[test]
    fn diagnostics_reports_live_only_observations_as_degraded() {
        let report = network_diagnostics_report(&NetworkDiagnosticsInput {
            nat_class: "cone".to_string(),
            udp_status: "pass".to_string(),
            direct_path_status: "pass".to_string(),
            relay_latency_ms: Some(RELAY_LATENCY_MS),
            port_map_protocols: vec!["pcp".to_string()],
            interface_refs: refs(),
            route_refs: refs(),
            live_observations_recorded: false,
            diagnostics: Vec::new(),
        })
        .expect("report");
        assert_eq!(report.decision, "degraded");
        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.contains("non-replayable")));
    }

    #[test]
    fn connectivity_probe_reports_relay_only_as_degraded_and_identity_mismatch_as_deny() {
        let probe = ConnectivityProbeInput {
            source_node: "node-a".to_string(),
            target_node: "node-b".to_string(),
            expected_endpoint_ref: fixture_ref("endpoint-a"),
            observed_endpoint_ref: Some(fixture_ref("endpoint-a")),
            direct_path_status: "deny".to_string(),
            relay_path_status: "pass".to_string(),
            timeout_ms: None,
            authority_refs: refs(),
            policy_refs: refs(),
            resource_refs: refs(),
            evidence_refs: refs(),
        };
        let degraded = connectivity_probe_receipt(&probe).expect("relay-only");
        assert_eq!(degraded.decision, "degraded");

        let mismatch = ConnectivityProbeInput {
            observed_endpoint_ref: Some(fixture_ref("other")),
            ..probe
        };
        let deny = connectivity_probe_receipt(&mismatch).expect("identity deny");
        assert_eq!(deny.decision, "deny");
    }

    #[test]
    fn port_mapping_denies_mutation_without_evidence_and_probe_does_not_mutate() {
        let deny = port_mapping_receipt(&PortMappingInput {
            mode: "mutate".to_string(),
            requester_ref: None,
            node_identity_ref: None,
            protocol: "pcp".to_string(),
            external_port: Some(EXTERNAL_PORT),
            internal_port: Some(INTERNAL_PORT),
            duration_seconds: Some(PORT_DURATION_SECONDS),
            authority_refs: Vec::new(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            operator_evidence_refs: Vec::new(),
            available_protocols: vec!["pcp".to_string()],
        })
        .expect("mutation deny");
        assert_eq!(deny.decision, "deny");

        let probe = port_mapping_receipt(&PortMappingInput {
            mode: "probe".to_string(),
            requester_ref: None,
            node_identity_ref: None,
            protocol: "pcp".to_string(),
            external_port: None,
            internal_port: None,
            duration_seconds: None,
            authority_refs: Vec::new(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            operator_evidence_refs: Vec::new(),
            available_protocols: vec!["pcp".to_string()],
        })
        .expect("probe pass");
        assert_eq!(probe.decision, "pass");
    }

    #[test]
    fn watcher_snapshot_keeps_latest_state_bounded() {
        let snapshot = watcher_snapshot_value(&NetworkWatcherInput {
            node: "node-a".to_string(),
            interface_state: "eth0-up".to_string(),
            address_state: "ipv6-ready".to_string(),
            default_route: "via-relay".to_string(),
            relay_state: "online".to_string(),
            endpoint_state: "listening".to_string(),
            observed_event_count: WATCHER_OBSERVED_EVENTS,
            retained_event_count: WATCHER_RETAINED_EVENTS,
            evidence_refs: refs(),
        })
        .expect("watcher");
        assert_eq!(snapshot.decision, "pass");
    }

    #[test]
    fn metrics_snapshot_renders_openmetrics_and_rejects_secret_labels() {
        let pass = metrics_snapshot(&MetricsSnapshotInput {
            node: "node-a".to_string(),
            scrape_ref: fixture_ref("scrape"),
            policy_refs: refs(),
            redaction_refs: refs(),
            samples: vec![MetricSample {
                name: "molten_node_queue_depth".to_string(),
                kind: "gauge".to_string(),
                value: METRIC_VALUE,
                labels: vec![("route".to_string(), "redacted".to_string())],
            }],
        })
        .expect("metrics pass");
        assert_eq!(pass.decision, "pass");
        assert!(pass.openmetrics.contains("molten_node_queue_depth"));

        let deny = metrics_snapshot(&MetricsSnapshotInput {
            samples: vec![MetricSample {
                name: "molten_secret".to_string(),
                kind: "counter".to_string(),
                value: METRIC_VALUE,
                labels: vec![("ticket".to_string(), "ticket:abc".to_string())],
            }],
            ..MetricsSnapshotInput {
                node: "node-a".to_string(),
                scrape_ref: fixture_ref("scrape"),
                policy_refs: refs(),
                redaction_refs: refs(),
                samples: Vec::new(),
            }
        })
        .expect("metrics deny");
        assert_eq!(deny.decision, "deny");
    }

    #[test]
    fn external_bridge_disabled_by_default_and_scoped_when_enabled() {
        let disabled = external_diagnostics_bridge_receipt(&ExternalDiagnosticsBridgeInput {
            enabled: false,
            mode: "push".to_string(),
            target_service_ref: None,
            capability_refs: Vec::new(),
            policy_refs: Vec::new(),
            redaction_policy_refs: Vec::new(),
            api_secret_provenance_ref: None,
            operator_evidence_refs: Vec::new(),
            expiry_ref: None,
        })
        .expect("disabled");
        assert_eq!(disabled.decision, "deny");

        let enabled = external_diagnostics_bridge_receipt(&ExternalDiagnosticsBridgeInput {
            enabled: true,
            mode: "remote-request".to_string(),
            target_service_ref: Some(fixture_ref("target")),
            capability_refs: refs(),
            policy_refs: refs(),
            redaction_policy_refs: refs(),
            api_secret_provenance_ref: Some(fixture_ref("secret-provenance")),
            operator_evidence_refs: refs(),
            expiry_ref: Some(fixture_ref("expiry")),
        })
        .expect("enabled");
        assert_eq!(enabled.decision, "pass");
    }
}
