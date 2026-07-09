
fn chunk_store_result(chunk_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let Some(chunk_root) = chunk_root else {
        return Err(MoltenError::invalid_harness(
            "catalog MCP chunk-store tool requires a chunk store root supplied by the caller",
        ));
    };
    crate::catalog::chunk_store(chunk_root, &crate::catalog::ChunkStoreInput {
        visibility: request.visibility.clone(),
    })
    .map(CoreResult::Query)
}

fn deps_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let graph_input = graph_input(request)?;
    crate::catalog::dependencies(registry_root, ledger_root, &graph_input).map(CoreResult::Query)
}

fn dependents_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let graph_input = graph_input(request)?;
    crate::catalog::dependents(registry_root, ledger_root, &graph_input).map(CoreResult::Query)
}

fn impact_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let graph_input = graph_input(request)?;
    crate::catalog::impact(registry_root, ledger_root, &graph_input).map(CoreResult::Query)
}

fn receipts_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let graph_input = graph_input(request)?;
    crate::catalog::receipts(registry_root, ledger_root, &graph_input).map(CoreResult::Query)
}

fn graph_input(request: &Request) -> Result<crate::catalog::GraphInput> {
    Ok(crate::catalog::GraphInput {
        reference: required_arg_string(&request.args, "reference")?,
        transitive: arg_bool(&request.args, "transitive", false)?,
        visibility: request.visibility.clone(),
    })
}

fn short_id_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let prefix = required_arg_string(&request.args, "prefix")?;
    let min_length =
        usize::try_from(arg_u64(&request.args, "min-length", crate::catalog::DEFAULT_SHORT_ID_MIN_LENGTH as u64)?)
            .map_err(|error| MoltenError::invalid_harness(format!("catalog MCP min-length is unsupported: {error}")))?;
    crate::catalog::resolve_short_id(registry_root, ledger_root, &crate::catalog::ShortIdInput {
        prefix,
        min_length,
        visibility: request.visibility.clone(),
    })
    .map(CoreResult::ShortId)
}

fn schema_search_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let schema_ref = required_arg_string(&request.args, "schema-ref")?;
    let mut filters = filters_from_args(&request.args)?;
    push_bounded(&mut filters, Filter::SchemaRef(schema_ref), MAX_FILTERS, "catalog MCP filters")?;
    search_result(registry_root, ledger_root, request, filters)
}

fn effect_search_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let effect_ref = required_arg_string(&request.args, "effect-ref")?;
    let mut filters = filters_from_args(&request.args)?;
    push_bounded(&mut filters, Filter::EffectRef(effect_ref), MAX_FILTERS, "catalog MCP filters")?;
    search_result(registry_root, ledger_root, request, filters)
}

fn upgrade_search_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let mut filters = filters_from_args(&request.args)?;
    if filters.is_empty() {
        push_bounded(&mut filters, Filter::UpgradeStatus("planned".to_string()), MAX_FILTERS, "catalog MCP filters")?;
    }
    search_result(registry_root, ledger_root, request, filters)
}

fn transcript_search_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let mut filters = filters_from_args(&request.args)?;
    if filters.is_empty() {
        push_bounded(&mut filters, Filter::TranscriptStatus("pass".to_string()), MAX_FILTERS, "catalog MCP filters")?;
    }
    search_result(registry_root, ledger_root, request, filters)
}

fn provenance_search_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let mut filters = filters_from_args(&request.args)?;
    push_bounded(&mut filters, Filter::Text("provenance:".to_string()), MAX_FILTERS, "catalog MCP filters")?;
    if let Some(trust_state) = optional_arg_string(&request.args, "trust-state") {
        push_bounded(
            &mut filters,
            Filter::Text(format!("provenance-trust-state:{trust_state}")),
            MAX_FILTERS,
            "catalog MCP filters",
        )?;
    }
    if let Some(decision) = optional_arg_string(&request.args, "decision") {
        push_bounded(
            &mut filters,
            Filter::Text(format!("provenance-decision:{decision}")),
            MAX_FILTERS,
            "catalog MCP filters",
        )?;
    }
    search_result(registry_root, ledger_root, request, filters)
}

fn retention_search_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let mut filters = filters_from_args(&request.args)?;
    push_bounded(&mut filters, Filter::Text("retention-gc:".to_string()), MAX_FILTERS, "catalog MCP filters")?;
    push_optional_text_filter(&mut filters, &request.args, "stage", "retention-gc")?;
    push_optional_text_filter(&mut filters, &request.args, "object-ref", "retention-gc-object")?;
    push_optional_text_filter(&mut filters, &request.args, "subsystem", "retention-gc-subsystem")?;
    push_optional_text_filter(&mut filters, &request.args, "decision", "retention-gc-decision")?;
    push_optional_text_filter(&mut filters, &request.args, "plan-ref", "retention-gc-plan")?;
    push_optional_text_filter(&mut filters, &request.args, "apply-ref", "retention-gc-apply")?;
    push_optional_text_filter(&mut filters, &request.args, "execution-ref", "retention-gc-execution")?;
    search_result(registry_root, ledger_root, request, filters)
}

fn replay_search_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let mut filters = filters_from_args(&request.args)?;
    push_bounded(&mut filters, Filter::Text("deterministic-replay:".to_string()), MAX_FILTERS, "catalog MCP filters")?;
    push_optional_text_filter(&mut filters, &request.args, "stage", "deterministic-replay")?;
    push_optional_text_filter(&mut filters, &request.args, "decision", "replay-decision")?;
    push_optional_text_filter(&mut filters, &request.args, "divergence", "replay-divergence")?;
    push_optional_text_filter(&mut filters, &request.args, "actor-id", "replay-actor")?;
    push_optional_text_filter(&mut filters, &request.args, "handler-profile-ref", "replay-handler-profile")?;
    push_optional_text_filter(&mut filters, &request.args, "expected-report-ref", "replay-expected-report")?;
    push_optional_text_filter(&mut filters, &request.args, "actual-report-ref", "replay-actual-report")?;
    push_optional_text_filter(&mut filters, &request.args, "final-state-ref", "replay-final-state")?;
    push_optional_text_filter(&mut filters, &request.args, "expected-ref", "replay-expected-ref")?;
    push_optional_text_filter(&mut filters, &request.args, "actual-ref", "replay-actual-ref")?;
    push_optional_text_filter(&mut filters, &request.args, "expected-output-ref", "replay-expected-output")?;
    push_optional_text_filter(&mut filters, &request.args, "actual-output-ref", "replay-actual-output")?;
    push_optional_text_filter(&mut filters, &request.args, "expected-effect-log-ref", "replay-expected-effect-log")?;
    push_optional_text_filter(&mut filters, &request.args, "actual-effect-log-ref", "replay-actual-effect-log")?;
    push_optional_text_filter(&mut filters, &request.args, "expected-final-state-ref", "replay-expected-final-state")?;
    push_optional_text_filter(&mut filters, &request.args, "actual-final-state-ref", "replay-actual-final-state")?;
    push_optional_text_filter(
        &mut filters,
        &request.args,
        "release-replay-verify-ref",
        "release-dogfood-replay-verify",
    )?;
    push_optional_text_filter(&mut filters, &request.args, "release-replay-index-ref", "release-dogfood-replay-index")?;
    search_result(registry_root, ledger_root, request, filters)
}

fn artifact_search_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let filters = filters_from_args(&request.args)?;
    search_result(registry_root, ledger_root, request, filters)
}

fn search_result(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    request: &Request,
    filters: Vec<Filter>,
) -> Result<CoreResult> {
    crate::catalog::search(registry_root, ledger_root, &crate::catalog::SearchInput {
        root_refs: arg_strings(&request.args, "root")?,
        include_dependencies: arg_bool(&request.args, "include-dependencies", true)?,
        include_dependents: arg_bool(&request.args, "include-dependents", true)?,
        filters,
        visibility: request.visibility.clone(),
    })
    .map(CoreResult::Query)
}

fn build_call(input: BuildCallInput<'_>) -> Result<Call> {
    validate_decision(input.decision)?;
    let catalog_receipt_ref = input.catalog_receipt_value.map(canonical_hash).transpose()?;
    let response_value = response_value(&ResponseValueInput {
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
    push_bounded(&mut refs, input.request.request_ref.clone(), MAX_REFS, "catalog MCP call refs")?;
    push_bounded(&mut refs, response_ref.clone(), MAX_REFS, "catalog MCP call refs")?;
    if let Some(result_ref) = input.result_ref {
        push_bounded(&mut refs, result_ref.to_string(), MAX_REFS, "catalog MCP call refs")?;
    }
    if let Some(catalog_receipt_ref) = catalog_receipt_ref.as_ref() {
        push_bounded(&mut refs, catalog_receipt_ref.clone(), MAX_REFS, "catalog MCP call refs")?;
    }
    let receipt_value = receipt_value(&ReceiptValueInput {
        tool: &input.request.tool,
        decision: input.decision,
        request_ref: &input.request.request_ref,
        response_ref: &response_ref,
        catalog_receipt_ref: catalog_receipt_ref.as_deref(),
        refs: &refs,
        diagnostics: &input.diagnostics,
        checks: input.checks,
    })?;
    Ok(Call {
        request: input.request,
        decision: input.decision.to_string(),
        response_ref,
        response_value,
        receipt_value,
        catalog_receipt_ref,
    })
}

fn response_value(input: &ResponseValueInput<'_>) -> Result<IoValue> {
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
        string(crate::preserves_rail::CATALOG_MCP_RESPONSE_SCHEMA),
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

fn receipt_value(input: &ReceiptValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.tool, "catalog MCP receipt tool")?;
    validate_decision(input.decision)?;
    validate_ref(input.request_ref, "catalog MCP receipt request ref")?;
    validate_ref(input.response_ref, "catalog MCP receipt response ref")?;
    if let Some(catalog_receipt_ref) = input.catalog_receipt_ref {
        validate_ref(catalog_receipt_ref, "catalog MCP receipt catalog receipt ref")?;
    }
    validate_refs(input.refs, "catalog MCP receipt ref")?;
    ensure_count_at_most(input.checks.len(), MAX_CHECKS, "catalog MCP receipt checks")?;
    let mut all_checks = Vec::new();
    push_bounded(&mut all_checks, ("canonical-receipt", "pass"), MAX_CHECKS, "catalog MCP receipt checks")?;
    push_bounded(&mut all_checks, ("mutating-tools-denied", "pass"), MAX_CHECKS, "catalog MCP receipt checks")?;
    for check in input.checks {
        push_bounded(&mut all_checks, *check, MAX_CHECKS, "catalog MCP receipt checks")?;
    }
    Ok(record("catalog-mcp-receipt-v1", vec![
        string(crate::preserves_rail::CATALOG_MCP_RECEIPT_SCHEMA),
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
    Query(crate::catalog::QueryResult),
    ShortId(crate::catalog::ShortIdResolution),
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

fn filters_from_args(args: &[IoValue]) -> Result<Vec<Filter>> {
    let mut filters = Vec::new();
    append_filter_args(&mut filters, arg_strings(args, "ref")?, Filter::Ref)?;
    append_filter_args(&mut filters, arg_strings(args, "kind")?, Filter::ArtifactKind)?;
    append_filter_args(&mut filters, arg_strings(args, "ledger-kind")?, Filter::LedgerKind)?;
    append_filter_args(&mut filters, arg_strings(args, "schema-ref")?, Filter::SchemaRef)?;
    append_filter_args(&mut filters, arg_strings(args, "structural-fingerprint")?, Filter::StructuralFingerprint)?;
    append_filter_args(&mut filters, arg_strings(args, "effect-ref")?, Filter::EffectRef)?;
    append_filter_args(&mut filters, arg_strings(args, "policy-ref")?, Filter::PolicyRef)?;
    append_filter_args(&mut filters, arg_strings(args, "capability-ref")?, Filter::CapabilityRef)?;
    append_filter_args(&mut filters, arg_strings(args, "evidence-ref")?, Filter::EvidenceRef)?;
    append_filter_args(&mut filters, arg_strings(args, "dependency-ref")?, Filter::DependencyRef)?;
    append_filter_args(&mut filters, arg_strings(args, "dependent-ref")?, Filter::DependentRef)?;
    append_filter_args(&mut filters, arg_strings(args, "receipt-operation")?, Filter::ReceiptOperation)?;
    append_filter_args(&mut filters, arg_strings(args, "receipt-decision")?, Filter::ReceiptDecision)?;
    append_filter_args(&mut filters, arg_strings(args, "transcript-status")?, Filter::TranscriptStatus)?;
    append_filter_args(&mut filters, arg_strings(args, "upgrade-status")?, Filter::UpgradeStatus)?;
    append_filter_args(&mut filters, arg_strings(args, "text")?, Filter::Text)?;
    Ok(filters)
}
