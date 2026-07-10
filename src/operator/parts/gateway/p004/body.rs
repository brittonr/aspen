
const HTTP3_IROH_READBACK_ADAPTER_SCHEMA: &str = "molten.operator.http3-iroh-readback-adapter.v1";
const HTTP3_METHOD_GET: &str = "GET";
const HTTP3_METHOD_HEAD: &str = "HEAD";
const HTTP3_ADAPTER_DIAGNOSTIC_CAPACITY: usize = 12;
const HTTP3_STATUS_OK: u16 = 200;
const HTTP3_STATUS_FORBIDDEN: u16 = 403;

#[derive(Debug, Clone)]
pub struct Http3IrohReadbackInput<'a> {
    pub method: &'a str,
    pub route: &'a str,
    pub session_ref: &'a str,
    pub requester_ref: &'a str,
    pub object_ref: &'a str,
    pub requested_range: Option<Range>,
    pub manifest: Option<&'a ChunkManifest>,
    pub chunk_bytes: std::collections::BTreeMap<String, Vec<u8>>,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Http3IrohReadbackDecision {
    pub decision: String,
    pub status: u16,
    pub bytes: Vec<u8>,
    pub gateway_receipt_value: Option<IoValue>,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

pub fn handle_http3_iroh_readback(input: &Http3IrohReadbackInput<'_>) -> Result<Http3IrohReadbackDecision> {
    validate_http3_adapter_input(input)?;
    let mut diagnostics = Vec::with_capacity(HTTP3_ADAPTER_DIAGNOSTIC_CAPACITY);
    if !matches!(input.method, HTTP3_METHOD_GET | HTTP3_METHOD_HEAD) {
        diagnostics.push(format!("HTTP3-over-Iroh method {} is read-only unsupported", input.method));
    }
    if diagnostics.is_empty() {
        let read = ReadInput {
            object_ref: input.object_ref.to_string(),
            member: Some(input.route.to_string()),
            requested_range: input.requested_range,
            requester_ref: input.requester_ref.to_string(),
            manifest: input.manifest,
            visibility: input.visibility.clone(),
        };
        let gateway = if input.requested_range.is_some() && input.manifest.is_some() {
            let range = verify_range(&RangeVerificationInput {
                read,
                chunk_bytes: input.chunk_bytes.clone(),
            })?;
            if range.decision == "pass" {
                Http3GatewayOutcome {
                    decision: range.decision,
                    bytes: if input.method == HTTP3_METHOD_HEAD { Vec::new() } else { range.bytes },
                    receipt_value: range.receipt_value,
                    diagnostics: range.diagnostics,
                }
            } else {
                Http3GatewayOutcome {
                    decision: range.decision,
                    bytes: Vec::new(),
                    receipt_value: range.receipt_value,
                    diagnostics: range.diagnostics,
                }
            }
        } else {
            let decision = decide_readback(&read)?;
            Http3GatewayOutcome {
                decision: decision.decision,
                bytes: Vec::new(),
                receipt_value: decision.receipt_value,
                diagnostics: decision.diagnostics,
            }
        };
        diagnostics.extend(gateway.diagnostics.iter().cloned());
        let decision = if diagnostics.is_empty() && gateway.decision == "pass" { "pass" } else { "deny" };
        let status = http3_status_for_decision(decision);
        let receipt_value = http3_adapter_receipt_value(input, decision, status, Some(&gateway.receipt_value), &diagnostics)?;
        return Ok(Http3IrohReadbackDecision {
            decision: decision.to_string(),
            status,
            bytes: if decision == "pass" { gateway.bytes } else { Vec::new() },
            gateway_receipt_value: Some(gateway.receipt_value),
            diagnostics,
            receipt_value,
        });
    }
    let status = http3_status_for_decision("deny");
    let receipt_value = http3_adapter_receipt_value(input, "deny", status, None, &diagnostics)?;
    Ok(Http3IrohReadbackDecision {
        decision: "deny".to_string(),
        status,
        bytes: Vec::new(),
        gateway_receipt_value: None,
        diagnostics,
        receipt_value,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Http3GatewayOutcome {
    decision: String,
    bytes: Vec<u8>,
    receipt_value: IoValue,
    diagnostics: Vec<String>,
}

fn validate_http3_adapter_input(input: &Http3IrohReadbackInput<'_>) -> Result<()> {
    if input.route.trim().is_empty() {
        return Err(MoltenError::invalid_harness("HTTP3-over-Iroh route must not be empty"));
    }
    validate_content_ref(input.session_ref)?;
    validate_content_ref(input.requester_ref)?;
    validate_content_ref(input.object_ref)
}

fn http3_status_for_decision(decision: &str) -> u16 {
    if decision == "pass" {
        HTTP3_STATUS_OK
    } else {
        HTTP3_STATUS_FORBIDDEN
    }
}

fn http3_adapter_receipt_value(
    input: &Http3IrohReadbackInput<'_>,
    decision: &str,
    status: u16,
    gateway_receipt_value: Option<&IoValue>,
    diagnostics: &[String],
) -> Result<IoValue> {
    let gateway_receipt_ref = gateway_receipt_value.map(crate::preserves_rail::canonical_hash).transpose()?;
    Ok(record("http3-iroh-readback-adapter-v1", vec![
        string(HTTP3_IROH_READBACK_ADAPTER_SCHEMA),
        record("decision", vec![string(decision)]),
        record("status", vec![string(status.to_string())]),
        record("method", vec![string(input.method)]),
        record("route", vec![string(input.route)]),
        record("session", vec![string(input.session_ref)]),
        record("requester", vec![string(input.requester_ref)]),
        record("object", vec![string(input.object_ref)]),
        record("gateway-receipt", vec![optional_str(gateway_receipt_ref.as_deref())]),
        record("diagnostics", vec![string_sequence(diagnostics)]),
        record("checks", vec![sequence(vec![
            check_record("delegated-to-canonical-gateway", if gateway_receipt_ref.is_some() { "pass" } else { "fail" }),
            check_record("http-transport-is-not-authority", "pass"),
            check_record("preserves-identity-boundary", "pass"),
        ])]),
    ]))
}

fn optional_str(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), string)
}

fn string_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn check_record(name: &str, status: &str) -> IoValue {
    record("check", vec![string(name), string(status)])
}
