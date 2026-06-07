use std::path::Path;

use preserves::IOValue;
use preserves::Record;
use preserves::Value;

use crate::catalog;
use crate::catalog::CatalogFilter;
use crate::catalog::CatalogVisibilityInput;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::CATALOG_MCP_RECEIPT_SCHEMA;
use crate::preserves_rail::CATALOG_MCP_REQUEST_SCHEMA;
use crate::preserves_rail::CATALOG_MCP_RESPONSE_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::value_to_iovalue;

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
    "view_transcript",
    "list_upgrade_sessions",
    "list_provenance",
    "search_provenance",
    "short_id_resolve",
];

const MAX_CATALOG_MCP_ARGS: usize = 512;
const MAX_CATALOG_MCP_REFS: usize = 4096;
const MAX_CATALOG_MCP_FILTERS: usize = 512;
const MAX_CATALOG_MCP_CHECKS: usize = 128;

const _: () = assert!(MAX_CATALOG_MCP_ARGS <= 10_000);
const _: () = assert!(MAX_CATALOG_MCP_REFS <= 100_000);
const _: () = assert!(MAX_CATALOG_MCP_FILTERS <= 10_000);
const _: () = assert!(MAX_CATALOG_MCP_CHECKS <= 1_000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogMcpRequest {
    pub request_ref: String,
    pub tool: String,
    pub args: Vec<IOValue>,
    pub visibility: CatalogVisibilityInput,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogMcpCall {
    pub request: CatalogMcpRequest,
    pub decision: String,
    pub response_ref: String,
    pub response_value: IOValue,
    pub receipt_value: IOValue,
    pub catalog_receipt_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogMcpReceipt {
    pub receipt_ref: String,
    pub tool: String,
    pub decision: String,
    pub request_ref: String,
    pub response_ref: String,
    pub catalog_receipt_ref: Option<String>,
    pub value: IOValue,
}

struct BuildCallInput<'a> {
    request: CatalogMcpRequest,
    decision: &'a str,
    payload: Option<&'a IOValue>,
    result_ref: Option<&'a str>,
    catalog_receipt_value: Option<&'a IOValue>,
    diagnostics: Vec<String>,
    checks: &'a [(&'a str, &'a str)],
}

struct McpResponseValueInput<'a> {
    tool: &'a str,
    decision: &'a str,
    request_ref: &'a str,
    result_ref: Option<&'a str>,
    payload: Option<&'a IOValue>,
    catalog_receipt_ref: Option<&'a str>,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct McpReceiptValueInput<'a> {
    tool: &'a str,
    decision: &'a str,
    request_ref: &'a str,
    response_ref: &'a str,
    catalog_receipt_ref: Option<&'a str>,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

pub fn mcp_request_value(tool: &str, args: Vec<IOValue>) -> Result<IOValue> {
    validate_non_empty(tool, "catalog MCP tool")?;
    ensure_count_at_most(args.len(), MAX_CATALOG_MCP_ARGS, "catalog MCP args")?;
    Ok(record("catalog-mcp-request-v1", vec![
        string(CATALOG_MCP_REQUEST_SCHEMA),
        record("tool", vec![string(tool)]),
        record("args", vec![sequence(args)]),
        checks_value(&["read-only-surface", "no-registry-path-identity", "redacted-default"]),
    ]))
}

pub fn call(registry_root: &Path, ledger_root: Option<&Path>, request_value: &IOValue) -> Result<CatalogMcpCall> {
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
    match dispatch_read_only(registry_root, ledger_root, &request) {
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

pub fn parse_mcp_request(value: &IOValue) -> Result<CatalogMcpRequest> {
    let fields = value
        .collect_simple_record("catalog-mcp-request-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <catalog-mcp-request-v1 ...>"))?;
    require_schema(&fields[0], CATALOG_MCP_REQUEST_SCHEMA, "catalog MCP request")?;
    let checks = parse_checks(&fields[3])?;
    require_check(&checks, "read-only-surface", "catalog MCP request")?;
    let args = record_sequence(&fields[2], "args")?;
    let visibility = visibility_from_args(&args)?;
    Ok(CatalogMcpRequest {
        request_ref: canonical_hash(value)?,
        tool: record_string(&fields[1], "tool")?,
        args,
        visibility,
        value: value.clone(),
    })
}

pub fn parse_mcp_receipt(value: &IOValue) -> Result<CatalogMcpReceipt> {
    let fields = value
        .collect_simple_record("catalog-mcp-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <catalog-mcp-receipt-v1 ...>"))?;
    require_schema(&fields[0], CATALOG_MCP_RECEIPT_SCHEMA, "catalog MCP receipt")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "canonical-receipt", "catalog MCP receipt")?;
    Ok(CatalogMcpReceipt {
        receipt_ref: canonical_hash(value)?,
        tool: record_string(&fields[1], "tool")?,
        decision: record_string(&fields[2], "decision")?,
        request_ref: record_ref(&fields[3], "request")?,
        response_ref: record_ref(&fields[4], "response")?,
        catalog_receipt_ref: record_optional_ref(&fields[5], "catalog-receipt")?,
        value: value.clone(),
    })
}

pub fn catalog_mcp_summary(value: &IOValue) -> Result<String> {
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
    value: IOValue,
    catalog_receipt_value: IOValue,
    diagnostics: Vec<String>,
}

fn dispatch_read_only(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    request: &CatalogMcpRequest,
) -> Result<DispatchPayload> {
    let result = match request.tool.as_str() {
        "catalog.list" | "list_artifacts" => {
            let kind = optional_arg_string(&request.args, "kind");
            catalog::list(registry_root, ledger_root, &catalog::CatalogListInput {
                kind,
                visibility: request.visibility.clone(),
            })
            .map(CoreResult::Query)
        }
        "catalog.view" | "view_artifact" | "view_transcript" => required_arg_string(&request.args, "reference")
            .and_then(|reference| {
                let should_include_payload = arg_bool(&request.args, "payload", false)?;
                let should_redact_payload = arg_bool(&request.args, "redacted", true)?;
                catalog::view(registry_root, ledger_root, &catalog::CatalogViewInput {
                    reference,
                    include_payload: should_include_payload,
                    redacted: should_redact_payload,
                    visibility: request.visibility.clone(),
                })
                .map(CoreResult::Query)
            }),
        "search_by_schema" => required_arg_string(&request.args, "schema-ref").and_then(|schema_ref| {
            let mut filters = filters_from_args(&request.args)?;
            push_bounded(
                &mut filters,
                CatalogFilter::SchemaRef(schema_ref),
                MAX_CATALOG_MCP_FILTERS,
                "catalog MCP filters",
            )?;
            let root_refs = arg_strings(&request.args, "root")?;
            catalog::search(registry_root, ledger_root, &catalog::CatalogSearchInput {
                root_refs,
                include_dependencies: arg_bool(&request.args, "include-dependencies", true)?,
                include_dependents: arg_bool(&request.args, "include-dependents", true)?,
                filters,
                visibility: request.visibility.clone(),
            })
            .map(CoreResult::Query)
        }),
        "search_by_effect" => required_arg_string(&request.args, "effect-ref").and_then(|effect_ref| {
            let mut filters = filters_from_args(&request.args)?;
            push_bounded(
                &mut filters,
                CatalogFilter::EffectRef(effect_ref),
                MAX_CATALOG_MCP_FILTERS,
                "catalog MCP filters",
            )?;
            let root_refs = arg_strings(&request.args, "root")?;
            catalog::search(registry_root, ledger_root, &catalog::CatalogSearchInput {
                root_refs,
                include_dependencies: arg_bool(&request.args, "include-dependencies", true)?,
                include_dependents: arg_bool(&request.args, "include-dependents", true)?,
                filters,
                visibility: request.visibility.clone(),
            })
            .map(CoreResult::Query)
        }),
        "list_upgrade_sessions" => {
            let mut filters = filters_from_args(&request.args)?;
            if filters.is_empty() {
                push_bounded(
                    &mut filters,
                    CatalogFilter::UpgradeStatus("planned".to_string()),
                    MAX_CATALOG_MCP_FILTERS,
                    "catalog MCP filters",
                )?;
            }
            catalog::search(registry_root, ledger_root, &catalog::CatalogSearchInput {
                root_refs: arg_strings(&request.args, "root")?,
                include_dependencies: arg_bool(&request.args, "include-dependencies", true)?,
                include_dependents: arg_bool(&request.args, "include-dependents", true)?,
                filters,
                visibility: request.visibility.clone(),
            })
            .map(CoreResult::Query)
        }
        "list_provenance" | "search_provenance" => {
            let mut filters = filters_from_args(&request.args)?;
            push_bounded(
                &mut filters,
                CatalogFilter::Text("provenance:".to_string()),
                MAX_CATALOG_MCP_FILTERS,
                "catalog MCP filters",
            )?;
            if let Some(trust_state) = optional_arg_string(&request.args, "trust-state") {
                push_bounded(
                    &mut filters,
                    CatalogFilter::Text(format!("provenance-trust-state:{trust_state}")),
                    MAX_CATALOG_MCP_FILTERS,
                    "catalog MCP filters",
                )?;
            }
            if let Some(decision) = optional_arg_string(&request.args, "decision") {
                push_bounded(
                    &mut filters,
                    CatalogFilter::Text(format!("provenance-decision:{decision}")),
                    MAX_CATALOG_MCP_FILTERS,
                    "catalog MCP filters",
                )?;
            }
            catalog::search(registry_root, ledger_root, &catalog::CatalogSearchInput {
                root_refs: arg_strings(&request.args, "root")?,
                include_dependencies: arg_bool(&request.args, "include-dependencies", true)?,
                include_dependents: arg_bool(&request.args, "include-dependents", true)?,
                filters,
                visibility: request.visibility.clone(),
            })
            .map(CoreResult::Query)
        }
        "catalog.search" | "search_artifacts" => {
            let filters = filters_from_args(&request.args)?;
            let root_refs = arg_strings(&request.args, "root")?;
            let should_include_dependencies = arg_bool(&request.args, "include-dependencies", true)?;
            let should_include_dependents = arg_bool(&request.args, "include-dependents", true)?;
            catalog::search(registry_root, ledger_root, &catalog::CatalogSearchInput {
                root_refs,
                include_dependencies: should_include_dependencies,
                include_dependents: should_include_dependents,
                filters,
                visibility: request.visibility.clone(),
            })
            .map(CoreResult::Query)
        }
        "catalog.deps" | "list_dependencies" => required_arg_string(&request.args, "reference").and_then(|reference| {
            let should_expand_transitively = arg_bool(&request.args, "transitive", false)?;
            catalog::dependencies(registry_root, ledger_root, &catalog::CatalogGraphInput {
                reference,
                transitive: should_expand_transitively,
                visibility: request.visibility.clone(),
            })
            .map(CoreResult::Query)
        }),
        "catalog.dependents" | "list_dependents" => {
            required_arg_string(&request.args, "reference").and_then(|reference| {
                let should_expand_transitively = arg_bool(&request.args, "transitive", false)?;
                catalog::dependents(registry_root, ledger_root, &catalog::CatalogGraphInput {
                    reference,
                    transitive: should_expand_transitively,
                    visibility: request.visibility.clone(),
                })
                .map(CoreResult::Query)
            })
        }
        "view_receipts" => required_arg_string(&request.args, "reference").and_then(|reference| {
            let should_expand_transitively = arg_bool(&request.args, "transitive", false)?;
            catalog::receipts(registry_root, ledger_root, &catalog::CatalogGraphInput {
                reference,
                transitive: should_expand_transitively,
                visibility: request.visibility.clone(),
            })
            .map(CoreResult::Query)
        }),
        "catalog.short_id" | "short_id_resolve" => required_arg_string(&request.args, "prefix").and_then(|prefix| {
            let min_length =
                usize::try_from(arg_u64(&request.args, "min-length", catalog::DEFAULT_SHORT_ID_MIN_LENGTH as u64)?)
                    .map_err(|error| {
                        MoltenError::invalid_harness(format!("catalog MCP min-length is unsupported: {error}"))
                    })?;
            catalog::resolve_short_id(registry_root, ledger_root, &catalog::CatalogShortIdInput {
                prefix,
                min_length,
                visibility: request.visibility.clone(),
            })
            .map(CoreResult::ShortId)
        }),
        _ => Err(MoltenError::invalid_harness(format!(
            "catalog MCP tool {} is not in the read-only dispatch allow-list",
            request.tool
        ))),
    };
    result.map(DispatchPayload::from)
}

fn build_call(input: BuildCallInput<'_>) -> Result<CatalogMcpCall> {
    validate_decision(input.decision)?;
    let catalog_receipt_ref = input.catalog_receipt_value.map(canonical_hash).transpose()?;
    let response_value = mcp_response_value(&McpResponseValueInput {
        tool: &input.request.tool,
        decision: input.decision,
        request_ref: &input.request.request_ref,
        result_ref: input.result_ref,
        payload: input.payload,
        catalog_receipt_ref: catalog_receipt_ref.as_deref(),
        diagnostics: &input.diagnostics,
        checks: input.checks,
    })?;
    let response_ref = canonical_hash(&response_value)?;
    let mut refs = Vec::new();
    push_bounded(&mut refs, input.request.request_ref.clone(), MAX_CATALOG_MCP_REFS, "catalog MCP call refs")?;
    push_bounded(&mut refs, response_ref.clone(), MAX_CATALOG_MCP_REFS, "catalog MCP call refs")?;
    if let Some(result_ref) = input.result_ref {
        push_bounded(&mut refs, result_ref.to_string(), MAX_CATALOG_MCP_REFS, "catalog MCP call refs")?;
    }
    if let Some(catalog_receipt_ref) = catalog_receipt_ref.as_ref() {
        push_bounded(&mut refs, catalog_receipt_ref.clone(), MAX_CATALOG_MCP_REFS, "catalog MCP call refs")?;
    }
    let receipt_value = mcp_receipt_value(&McpReceiptValueInput {
        tool: &input.request.tool,
        decision: input.decision,
        request_ref: &input.request.request_ref,
        response_ref: &response_ref,
        catalog_receipt_ref: catalog_receipt_ref.as_deref(),
        refs: &refs,
        diagnostics: &input.diagnostics,
        checks: input.checks,
    })?;
    Ok(CatalogMcpCall {
        request: input.request,
        decision: input.decision.to_string(),
        response_ref,
        response_value,
        receipt_value,
        catalog_receipt_ref,
    })
}

fn mcp_response_value(input: &McpResponseValueInput<'_>) -> Result<IOValue> {
    validate_non_empty(input.tool, "catalog MCP response tool")?;
    validate_decision(input.decision)?;
    validate_ref(input.request_ref, "catalog MCP response request ref")?;
    if let Some(result_ref) = input.result_ref {
        validate_ref(result_ref, "catalog MCP response result ref")?;
    }
    if let Some(catalog_receipt_ref) = input.catalog_receipt_ref {
        validate_ref(catalog_receipt_ref, "catalog MCP response catalog receipt ref")?;
    }
    Ok(record("catalog-mcp-response-v1", vec![
        string(CATALOG_MCP_RESPONSE_SCHEMA),
        record("tool", vec![string(input.tool)]),
        record("decision", vec![string(input.decision)]),
        record("request", vec![string(input.request_ref)]),
        record("result", vec![optional_ref_value(input.result_ref)]),
        record("payload", vec![input.payload.cloned().unwrap_or_else(|| record("none", Vec::new()))]),
        record("catalog-receipt", vec![optional_ref_value(input.catalog_receipt_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(input.checks),
    ]))
}

fn mcp_receipt_value(input: &McpReceiptValueInput<'_>) -> Result<IOValue> {
    validate_non_empty(input.tool, "catalog MCP receipt tool")?;
    validate_decision(input.decision)?;
    validate_ref(input.request_ref, "catalog MCP receipt request ref")?;
    validate_ref(input.response_ref, "catalog MCP receipt response ref")?;
    if let Some(catalog_receipt_ref) = input.catalog_receipt_ref {
        validate_ref(catalog_receipt_ref, "catalog MCP receipt catalog receipt ref")?;
    }
    validate_refs(input.refs, "catalog MCP receipt ref")?;
    ensure_count_at_most(input.checks.len(), MAX_CATALOG_MCP_CHECKS, "catalog MCP receipt checks")?;
    let mut all_checks = Vec::new();
    push_bounded(&mut all_checks, ("canonical-receipt", "pass"), MAX_CATALOG_MCP_CHECKS, "catalog MCP receipt checks")?;
    push_bounded(
        &mut all_checks,
        ("mutating-tools-denied", "pass"),
        MAX_CATALOG_MCP_CHECKS,
        "catalog MCP receipt checks",
    )?;
    for check in input.checks {
        push_bounded(&mut all_checks, *check, MAX_CATALOG_MCP_CHECKS, "catalog MCP receipt checks")?;
    }
    Ok(record("catalog-mcp-receipt-v1", vec![
        string(CATALOG_MCP_RECEIPT_SCHEMA),
        record("tool", vec![string(input.tool)]),
        record("decision", vec![string(input.decision)]),
        record("request", vec![string(input.request_ref)]),
        record("response", vec![string(input.response_ref)]),
        record("catalog-receipt", vec![optional_ref_value(input.catalog_receipt_ref)]),
        record("refs", vec![refs_sequence(&sorted_unique(input.refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(&all_checks),
    ]))
}

enum CoreResult {
    Query(catalog::CatalogQueryResult),
    ShortId(catalog::CatalogShortIdResolution),
}

impl From<CoreResult> for DispatchPayload {
    fn from(result: CoreResult) -> Self {
        match result {
            CoreResult::Query(result) => Self {
                decision: result.decision,
                result_ref: result.result_ref,
                value: result.value,
                catalog_receipt_value: result.receipt_value,
                diagnostics: result.diagnostics,
            },
            CoreResult::ShortId(result) => {
                let result_ref = canonical_hash(&result.value).unwrap_or_else(|_| result.prefix.clone());
                Self {
                    decision: result.decision,
                    result_ref,
                    value: result.value,
                    catalog_receipt_value: result.receipt_value,
                    diagnostics: Vec::new(),
                }
            }
        }
    }
}

fn filters_from_args(args: &[IOValue]) -> Result<Vec<CatalogFilter>> {
    let mut filters = Vec::new();
    append_filter_args(&mut filters, arg_strings(args, "ref")?, CatalogFilter::Ref)?;
    append_filter_args(&mut filters, arg_strings(args, "kind")?, CatalogFilter::ArtifactKind)?;
    append_filter_args(&mut filters, arg_strings(args, "ledger-kind")?, CatalogFilter::LedgerKind)?;
    append_filter_args(&mut filters, arg_strings(args, "schema-ref")?, CatalogFilter::SchemaRef)?;
    append_filter_args(
        &mut filters,
        arg_strings(args, "structural-fingerprint")?,
        CatalogFilter::StructuralFingerprint,
    )?;
    append_filter_args(&mut filters, arg_strings(args, "effect-ref")?, CatalogFilter::EffectRef)?;
    append_filter_args(&mut filters, arg_strings(args, "policy-ref")?, CatalogFilter::PolicyRef)?;
    append_filter_args(&mut filters, arg_strings(args, "capability-ref")?, CatalogFilter::CapabilityRef)?;
    append_filter_args(&mut filters, arg_strings(args, "evidence-ref")?, CatalogFilter::EvidenceRef)?;
    append_filter_args(&mut filters, arg_strings(args, "dependency-ref")?, CatalogFilter::DependencyRef)?;
    append_filter_args(&mut filters, arg_strings(args, "dependent-ref")?, CatalogFilter::DependentRef)?;
    append_filter_args(&mut filters, arg_strings(args, "receipt-operation")?, CatalogFilter::ReceiptOperation)?;
    append_filter_args(&mut filters, arg_strings(args, "receipt-decision")?, CatalogFilter::ReceiptDecision)?;
    append_filter_args(&mut filters, arg_strings(args, "transcript-status")?, CatalogFilter::TranscriptStatus)?;
    append_filter_args(&mut filters, arg_strings(args, "upgrade-status")?, CatalogFilter::UpgradeStatus)?;
    append_filter_args(&mut filters, arg_strings(args, "text")?, CatalogFilter::Text)?;
    Ok(filters)
}

fn append_filter_args(
    filters: &mut impl crate::bounded::VecSink<CatalogFilter>,
    values: Vec<String>,
    convert: impl Fn(String) -> CatalogFilter,
) -> Result<()> {
    for value in values {
        push_bounded(&mut *filters, convert(value), MAX_CATALOG_MCP_FILTERS, "catalog MCP filters")?;
    }
    Ok(())
}

fn visibility_from_args(args: &[IOValue]) -> Result<CatalogVisibilityInput> {
    Ok(CatalogVisibilityInput {
        policy_refs: arg_strings(args, "policy-ref")?,
        capability_refs: arg_strings(args, "capability-ref")?,
        hidden_refs: arg_strings(args, "hidden-ref")?,
        redaction_profile_ref: optional_arg_string(args, "redaction-profile-ref"),
    })
}

fn required_arg_string(args: &[IOValue], label: &str) -> Result<String> {
    optional_arg_string(args, label)
        .ok_or_else(|| MoltenError::invalid_harness(format!("catalog MCP request missing required arg <{label} ...>")))
}

fn optional_arg_string(args: &[IOValue], label: &str) -> Option<String> {
    args.iter().find_map(|arg| {
        arg.collect_simple_record(label, Some(1))
            .and_then(|fields| fields[0].as_string().map(|value| value.into_owned()))
    })
}

fn arg_strings(args: &[IOValue], label: &str) -> Result<Vec<String>> {
    ensure_count_at_most(args.len(), MAX_CATALOG_MCP_ARGS, "catalog MCP args")?;
    let mut values = Vec::new();
    for arg in args {
        if let Some(fields) = arg.collect_simple_record(label, Some(1)) {
            push_bounded(
                &mut values,
                required_string(&fields[0], label)?,
                MAX_CATALOG_MCP_ARGS,
                "catalog MCP arg strings",
            )?;
        }
    }
    Ok(values)
}

fn arg_bool(args: &[IOValue], label: &str, default: bool) -> Result<bool> {
    for arg in args {
        if let Some(fields) = arg.collect_simple_record(label, Some(1)) {
            return fields[0]
                .as_boolean()
                .ok_or_else(|| MoltenError::invalid_harness(format!("catalog MCP arg {label} must be bool")));
        }
    }
    Ok(default)
}

fn arg_u64(args: &[IOValue], label: &str, default: u64) -> Result<u64> {
    for arg in args {
        if let Some(fields) = arg.collect_simple_record(label, Some(1)) {
            return fields[0]
                .as_u64()
                .ok_or_else(|| MoltenError::invalid_harness(format!("catalog MCP arg {label} must be u64")))?
                .map_err(|error| {
                    MoltenError::invalid_harness(format!("catalog MCP arg {label} is out of range: {error}"))
                });
        }
    }
    Ok(default)
}

fn refs_sequence(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn checks_value(names: &[&str]) -> IOValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "catalog MCP checks")?;
    ensure_count_at_most(items.len(), MAX_CATALOG_MCP_CHECKS, "catalog MCP checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "catalog MCP check name")?;
        let status = required_string(&check[1], "catalog MCP check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("catalog MCP check {name} has status {status}")));
        }
        push_bounded(&mut parsed, name, MAX_CATALOG_MCP_CHECKS, "catalog MCP checks")?;
    }
    Ok(parsed)
}

fn require_check(checks: &[String], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &Value<IOValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IOValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IOValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IOValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IOValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_ref(&fields[0], label)
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&fields[0])
}

fn record_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<IOValue>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    ensure_count_at_most(items.len(), MAX_CATALOG_MCP_ARGS, label)?;
    let mut values = Vec::new();
    for item in items.iter() {
        push_bounded(&mut values, value_to_iovalue(item), MAX_CATALOG_MCP_ARGS, label)?;
    }
    Ok(values)
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &Value<IOValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported catalog MCP decision {decision}")))
    }
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    if value_ref.starts_with("blake3:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{field} must be a blake3 ref, got {value_ref}")))
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_CATALOG_MCP_REFS, field)?;
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> Result<usize> {
    let total = left
        .checked_add(right)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    Ok(total)
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    checked_count_sum(values.item_count(), 1, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<std::collections::BTreeSet<_>>().into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::artifacts;
    use crate::ledger;
    use crate::preserves_rail::parse_text;
    use crate::preserves_rail::to_text;

    #[test]
    fn readonly_list_view_search_match_catalog_core_and_bind_receipts() {
        let registry = temp_dir("catalog-mcp-readonly");
        let base = install_fixture(&registry, "schema", parse_text("<schema \"mcp\">").expect("schema"), &[], &[]);
        let doc = install_fixture(
            &registry,
            "doc",
            parse_text("<doc \"visible\">").expect("doc"),
            std::slice::from_ref(&base.artifact_ref),
            &[],
        );
        let request = mcp_request_value("catalog.search", vec![
            record("kind", vec![string("doc")]),
            record("dependency-ref", vec![string(&base.artifact_ref)]),
            record("text", vec![string("visible")]),
        ])
        .expect("request");
        let call = call(&registry, None, &request).expect("mcp call");
        assert_eq!(call.decision, "pass");
        assert!(call.catalog_receipt_ref.is_some());
        let response_text = to_text(&call.response_value).expect("response text");
        assert!(response_text.contains(&doc.artifact_ref));
        let receipt = parse_mcp_receipt(&call.receipt_value).expect("mcp receipt");
        assert_eq!(receipt.tool, "catalog.search");
    }

    #[test]
    fn unison_named_tools_search_schema_effect_and_receipts() {
        let registry = temp_dir("catalog-mcp-unison-tools");
        let schema_ref = test_ref("schema-ref");
        let effect_ref = test_ref("effect-ref");
        let base = install_fixture(&registry, "schema", parse_text("<schema \"alias\">").expect("schema"), &[], &[]);
        let doc = artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
            kind: "doc".to_string(),
            payload: parse_text("<doc \"alias-visible\">").expect("doc"),
            schema_refs: vec![schema_ref.clone()],
            dependency_refs: vec![base.artifact_ref.clone()],
            effect_manifest_ref: Some(effect_ref.clone()),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install doc");
        let schema_request =
            mcp_request_value("search_by_schema", vec![record("schema-ref", vec![string(&schema_ref)])])
                .expect("schema request");
        let schema_call = call(&registry, None, &schema_request).expect("schema call");
        assert!(to_text(&schema_call.response_value).expect("schema response").contains(&doc.artifact_ref));
        let effect_request =
            mcp_request_value("search_by_effect", vec![record("effect-ref", vec![string(&effect_ref)])])
                .expect("effect request");
        let effect_call = call(&registry, None, &effect_request).expect("effect call");
        assert!(to_text(&effect_call.response_value).expect("effect response").contains(&doc.artifact_ref));
        let deps_request =
            mcp_request_value("list_dependencies", vec![record("reference", vec![string(&doc.artifact_ref)])])
                .expect("deps request");
        let deps_call = call(&registry, None, &deps_request).expect("deps call");
        assert!(to_text(&deps_call.response_value).expect("deps response").contains(&base.artifact_ref));
        let receipts_request =
            mcp_request_value("view_receipts", vec![record("reference", vec![string(&doc.artifact_ref)])])
                .expect("receipts request");
        let receipts_call = call(&registry, None, &receipts_request).expect("receipts call");
        assert!(to_text(&receipts_call.response_value).expect("receipts response").contains("artifact-receipt-v1"));
        let short_request =
            mcp_request_value("short_id_resolve", vec![record("prefix", vec![string(&doc.artifact_ref[7..19])])])
                .expect("short request");
        let short_call = call(&registry, None, &short_request).expect("short call");
        assert_eq!(short_call.decision, "pass");
    }

    #[test]
    fn provenance_named_tools_search_trust_state_and_decision() {
        let root = temp_dir("catalog-mcp-provenance");
        let registry = root.join("registry");
        let ledger_root = root.join("ledger");
        let artifact_ref = test_ref("provenance-artifact");
        let provenance_record = crate::provenance::synthetic_reviewed_provenance_record(&artifact_ref).expect("record");
        let evaluation = crate::provenance::evaluate_provenance(&crate::provenance::ProvenanceEvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&provenance_record),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("evaluate provenance");
        ledger::import_artifact(&ledger_root, &provenance_record).expect("import provenance record");
        ledger::import_artifact(&ledger_root, &evaluation.receipt_value).expect("import provenance receipt");
        let record_request =
            mcp_request_value("list_provenance", vec![record("trust-state", vec![string("reviewed")])])
                .expect("provenance record request");
        let record_call = call(&registry, Some(&ledger_root), &record_request).expect("provenance record call");
        assert_eq!(record_call.decision, "pass");
        let record_text = to_text(&record_call.response_value).expect("provenance record response");
        assert!(record_text.contains("provenance:record"));
        let receipt_request = mcp_request_value("search_provenance", vec![record("decision", vec![string("pass")])])
            .expect("provenance receipt request");
        let receipt_call = call(&registry, Some(&ledger_root), &receipt_request).expect("provenance receipt call");
        assert_eq!(receipt_call.decision, "pass");
        let receipt_text = to_text(&receipt_call.response_value).expect("provenance receipt response");
        assert!(receipt_text.contains("provenance:receipt"));
    }

    #[test]
    fn hidden_refs_stay_hidden_and_redacted_view_is_default() {
        let registry = temp_dir("catalog-mcp-hidden");
        let secret =
            install_fixture(&registry, "doc", parse_text("<doc <secret \"hidden-value\">>").expect("secret"), &[], &[]);
        let hidden_request = mcp_request_value("catalog.search", vec![
            record("text", vec![string("hidden-value")]),
            record("hidden-ref", vec![string(&secret.artifact_ref)]),
        ])
        .expect("hidden request");
        let hidden = call(&registry, None, &hidden_request).expect("hidden call");
        assert_eq!(hidden.decision, "pass");
        assert!(!to_text(&hidden.response_value).expect("hidden response").contains(&secret.artifact_ref));
        let view_request = mcp_request_value("catalog.view", vec![
            record("reference", vec![string(&secret.artifact_ref)]),
            record("payload", vec![crate::preserves_rail::bool_value(true)]),
        ])
        .expect("view request");
        let viewed = call(&registry, None, &view_request).expect("view call");
        let text = to_text(&viewed.response_value).expect("view response");
        assert!(text.contains("redaction-marker-v1"));
        assert!(!text.contains("hidden-value"));
    }

    #[test]
    fn short_id_ambiguity_denies_and_mutating_tools_fail_closed() {
        let registry = temp_dir("catalog-mcp-deny");
        install_fixture(&registry, "doc", parse_text("<doc \"a\">").expect("a"), &[], &[]);
        install_fixture(&registry, "doc", parse_text("<doc \"b\">").expect("b"), &[], &[]);
        let short = mcp_request_value("catalog.short_id", vec![
            record("prefix", vec![string("blake3:")]),
            record("min-length", vec![crate::preserves_rail::u64_value(0)]),
        ])
        .expect("short request");
        let short_call = call(&registry, None, &short).expect("short call");
        assert_eq!(short_call.decision, "deny");
        assert!(to_text(&short_call.response_value).expect("short response").contains("ambiguous"));
        let mutate = mcp_request_value("catalog.install", vec![record("kind", vec![string("doc")])]).expect("mutate");
        let denied = call(&registry, None, &mutate).expect("mutating call denied");
        assert_eq!(denied.decision, "deny");
        assert!(to_text(&denied.receipt_value).expect("denial receipt").contains("mutating-tools-denied"));
    }

    #[hegel::test(test_cases = 10)]
    fn hegel_mcp_calls_are_deterministic_and_readonly(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let registry = temp_dir("catalog-mcp-hegel");
        let label = format!("payload-{salt}");
        install_fixture(&registry, "doc", record("doc", vec![string(&label)]), &[], &[]);
        let request = mcp_request_value("catalog.list", vec![record("kind", vec![string("doc")])]).expect("request");
        let first = call(&registry, None, &request).expect("first call");
        let second = call(&registry, None, &request).expect("second call");
        assert_eq!(first.response_ref, second.response_ref);
        let denied = call(&registry, None, &mcp_request_value("catalog.delete", Vec::new()).expect("mutating request"))
            .expect("denied mutating");
        assert_eq!(denied.decision, "deny");
    }

    fn install_fixture(
        root: &Path,
        kind: &str,
        payload: IOValue,
        dependency_refs: &[String],
        schema_refs: &[String],
    ) -> artifacts::ArtifactInstall {
        artifacts::install_artifact(root, &artifacts::ArtifactInstallInput {
            kind: kind.to_string(),
            payload,
            schema_refs: schema_refs.to_vec(),
            dependency_refs: dependency_refs.to_vec(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install fixture")
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("catalog-mcp-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
