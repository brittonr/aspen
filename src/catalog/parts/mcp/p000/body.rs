type Path = std::path::Path;
type IoValue = preserves::IOValue;
type PreservesRecord<T> = preserves::Record<T>;
type PreservesValue<T> = preserves::Value<T>;
type Filter = crate::catalog::Filter;
type VisibilityInput = crate::catalog::VisibilityInput;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

pub const READ_ONLY_TOOLS: &[&str] = &[
    "catalog.list",
    "catalog.view",
    "catalog.search",
    "catalog.deps",
    "catalog.dependents",
    "catalog.short_id",
    "list_artifacts",
    "view_artifact",
    "search_artifacts",
    "search_by_schema",
    "search_by_effect",
    "list_dependencies",
    "list_dependents",
    "view_receipts",
    "show_receipt",
    "search_receipts",
    "view_transcript",
    "search_transcripts",
    "impact_query",
    "explain_evidence",
    "show_release_snapshot",
    "list_upgrade_sessions",
    "list_provenance",
    "search_provenance",
    "search_retention_gc",
    "search_replay_evidence",
    "catalog.chunk_store",
    "search_chunk_store",
    "short_id_resolve",
];

const MAX_ARGS: usize = 512;
const MAX_REFS: usize = 4096;
const MAX_FILTERS: usize = 512;
const MAX_CHECKS: usize = 128;

const _: () = assert!(MAX_ARGS <= 10_000);
const _: () = assert!(MAX_REFS <= 100_000);
const _: () = assert!(MAX_FILTERS <= 10_000);
const _: () = assert!(MAX_CHECKS <= 1_000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub request_ref: String,
    pub tool: String,
    pub args: Vec<IoValue>,
    pub visibility: VisibilityInput,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub request: Request,
    pub decision: String,
    pub response_ref: String,
    pub response_value: IoValue,
    pub receipt_value: IoValue,
    pub catalog_receipt_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub receipt_ref: String,
    pub tool: String,
    pub decision: String,
    pub request_ref: String,
    pub response_ref: String,
    pub catalog_receipt_ref: Option<String>,
    pub value: IoValue,
}

struct BuildCallInput<'a> {
    request: Request,
    decision: &'a str,
    payload: Option<&'a IoValue>,
    result_ref: Option<&'a str>,
    catalog_receipt_value: Option<&'a IoValue>,
    diagnostics: Vec<String>,
    checks: &'a [(&'a str, &'a str)],
}

struct ResponseValueInput<'a> {
    tool: &'a str,
    decision: &'a str,
    request_ref: &'a str,
    result_ref: Option<&'a str>,
    payload: Option<&'a IoValue>,
    catalog_receipt_ref: Option<&'a str>,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct ReceiptValueInput<'a> {
    tool: &'a str,
    decision: &'a str,
    request_ref: &'a str,
    response_ref: &'a str,
    catalog_receipt_ref: Option<&'a str>,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

pub fn mcp_request_value(tool: &str, args: Vec<IoValue>) -> Result<IoValue> {
    validate_non_empty(tool, "catalog MCP tool")?;
    ensure_count_at_most(args.len(), MAX_ARGS, "catalog MCP args")?;
    Ok(record("catalog-mcp-request-v1", vec![
        string(crate::preserves_rail::CATALOG_MCP_REQUEST_SCHEMA),
        record("tool", vec![string(tool)]),
        record("args", vec![sequence(args)]),
        checks_value(&["read-only-surface", "no-registry-path-identity", "redacted-default"]),
    ]))
}

pub fn call(registry_root: &Path, ledger_root: Option<&Path>, request_value: &IoValue) -> Result<Call> {
    call_with_chunk_store(registry_root, ledger_root, None, request_value)
}

// r[impl molten.catalog.mcp_readonly_tools]
// r[impl molten.catalog.no_catalog_mutation_authority]
pub fn call_with_chunk_store(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    chunk_root: Option<&Path>,
    request_value: &IoValue,
) -> Result<Call> {
    let request = parse_mcp_request(request_value)?;
    if !READ_ONLY_TOOLS.contains(&request.tool.as_str()) {
        return build_call(BuildCallInput {
            request,
            decision: "deny",
            payload: None,
            result_ref: None,
            catalog_receipt_value: None,
            diagnostics: vec!["catalog MCP tool is not in the read-only allow-list".to_string()],
            checks: &[
                ("read-only-tool", "fail"),
                ("mutating-tools-denied", "pass"),
                ("visibility-check", "pass"),
                ("capability-check", "pass"),
            ],
        });
    }
    match dispatch_read_only(registry_root, ledger_root, chunk_root, &request) {
        Ok(payload) => build_call(BuildCallInput {
            request,
            decision: payload.decision.as_str(),
            payload: Some(&payload.value),
            result_ref: Some(&payload.result_ref),
            catalog_receipt_value: Some(&payload.catalog_receipt_value),
            diagnostics: payload.diagnostics,
            checks: &[
                ("read-only-tool", "pass"),
                ("catalog-core-receipt-bound", "pass"),
                ("visibility-check", "pass"),
                ("capability-check", "pass"),
                ("redacted-default", "pass"),
                ("full-ref-expansion", "pass"),
            ],
        }),
        Err(error) => build_call(BuildCallInput {
            request,
            decision: "deny",
            payload: None,
            result_ref: None,
            catalog_receipt_value: None,
            diagnostics: vec![error.to_string()],
            checks: &[
                ("read-only-tool", "pass"),
                ("fail-closed-diagnostics", "pass"),
                ("visibility-check", "pass"),
                ("capability-check", "pass"),
                ("redacted-default", "pass"),
            ],
        }),
    }
}

pub fn parse_mcp_request(value: &IoValue) -> Result<Request> {
    let fields = value
        .collect_simple_record("catalog-mcp-request-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <catalog-mcp-request-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::CATALOG_MCP_REQUEST_SCHEMA, "catalog MCP request")?;
    let checks = parse_checks(&fields[3])?;
    require_check(&checks, "read-only-surface", "catalog MCP request")?;
    let args = record_sequence(&fields[2], "args")?;
    let visibility = visibility_from_args(&args)?;
    Ok(Request {
        request_ref: canonical_hash(value)?,
        tool: record_string(&fields[1], "tool")?,
        args,
        visibility,
        value: value.clone(),
    })
}

pub fn parse_mcp_receipt(value: &IoValue) -> Result<Receipt> {
    let fields = value
        .collect_simple_record("catalog-mcp-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <catalog-mcp-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::CATALOG_MCP_RECEIPT_SCHEMA, "catalog MCP receipt")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "canonical-receipt", "catalog MCP receipt")?;
    Ok(Receipt {
        receipt_ref: canonical_hash(value)?,
        tool: record_string(&fields[1], "tool")?,
        decision: record_string(&fields[2], "decision")?,
        request_ref: record_ref(&fields[3], "request")?,
        response_ref: record_ref(&fields[4], "response")?,
        catalog_receipt_ref: record_optional_ref(&fields[5], "catalog-receipt")?,
        value: value.clone(),
    })
}

pub fn summary(value: &IoValue) -> Result<String> {
    if let Ok(receipt) = parse_mcp_receipt(value) {
        return Ok(format!(
            "catalog MCP receipt tool={} decision={} request={} response={}",
            receipt.tool, receipt.decision, receipt.request_ref, receipt.response_ref
        ));
    }
    if let Ok(request) = parse_mcp_request(value) {
        return Ok(format!("catalog MCP request tool={} ref={}", request.tool, request.request_ref));
    }
    if value.collect_simple_record("catalog-mcp-response-v1", Some(8)).is_some() {
        return Ok(format!("catalog MCP response ref={}", canonical_hash(value)?));
    }
    Err(MoltenError::invalid_harness("unsupported catalog MCP artifact for show"))
}

struct DispatchPayload {
    decision: String,
    result_ref: String,
    value: IoValue,
    catalog_receipt_value: IoValue,
    diagnostics: Vec<String>,
}

fn dispatch_read_only(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    chunk_root: Option<&Path>,
    request: &Request,
) -> Result<DispatchPayload> {
    let result = match request.tool.as_str() {
        "catalog.list" | "list_artifacts" => list_result(registry_root, ledger_root, request),
        "catalog.view" | "view_artifact" | "view_transcript" => view_result(registry_root, ledger_root, request),
        "search_by_schema" => schema_search_result(registry_root, ledger_root, request),
        "search_by_effect" => effect_search_result(registry_root, ledger_root, request),
        "list_upgrade_sessions" => upgrade_search_result(registry_root, ledger_root, request),
        "list_provenance" | "search_provenance" => provenance_search_result(registry_root, ledger_root, request),
        "search_retention_gc" => retention_search_result(registry_root, ledger_root, request),
        "search_replay_evidence" => replay_search_result(registry_root, ledger_root, request),
        "catalog.chunk_store" | "search_chunk_store" => chunk_store_result(chunk_root, request),
        "catalog.search" | "search_artifacts" => artifact_search_result(registry_root, ledger_root, request),
        "catalog.deps" | "list_dependencies" => deps_result(registry_root, ledger_root, request),
        "catalog.dependents" | "list_dependents" => dependents_result(registry_root, ledger_root, request),
        "impact_query" => impact_result(registry_root, ledger_root, request),
        "view_receipts" | "show_receipt" | "search_receipts" => receipts_result(registry_root, ledger_root, request),
        "search_transcripts" => transcript_search_result(registry_root, ledger_root, request),
        "show_release_snapshot" => release_snapshot_result(registry_root, ledger_root, request),
        "explain_evidence" => artifact_search_result(registry_root, ledger_root, request),
        "catalog.short_id" | "short_id_resolve" => short_id_result(registry_root, ledger_root, request),
        _ => Err(MoltenError::invalid_harness(format!(
            "catalog MCP tool {} is not in the read-only dispatch allow-list",
            request.tool
        ))),
    };
    result.map(DispatchPayload::from)
}

fn list_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let kind = optional_arg_string(&request.args, "kind");
    crate::catalog::list(registry_root, ledger_root, &crate::catalog::ListInput {
        kind,
        visibility: request.visibility.clone(),
    })
    .map(CoreResult::Query)
}

fn view_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let reference = required_arg_string(&request.args, "reference")?;
    let should_include_payload = arg_bool(&request.args, "payload", false)?;
    let should_redact_payload = arg_bool(&request.args, "redacted", true)?;
    crate::catalog::view(registry_root, ledger_root, &crate::catalog::ViewInput {
        reference,
        include_payload: should_include_payload,
        redacted: should_redact_payload,
        visibility: request.visibility.clone(),
    })
    .map(CoreResult::Query)
}
