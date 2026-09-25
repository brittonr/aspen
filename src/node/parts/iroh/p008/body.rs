
pub fn evaluate_router_operation(
    registry: &ProtocolRegistry,
    input: &RouterOperationInput,
) -> crate::error::Result<RouterDecision> {
    let evaluation = RouterEvaluator::new(registry, input).evaluate()?;
    let registry_entry_ref = lookup_alpn_registry_entry(&input.alpn)?.map(|entry| entry.entry_ref);
    let decision = if evaluation.diagnostics.is_empty() {
        "pass"
    } else {
        "deny"
    }
    .to_string();
    let receipt_value = router_receipt_value(RouterReceiptInput {
        decision: &decision,
        operation: &input.operation,
        outcome: &evaluation.mutation.outcome,
        alpn: &input.alpn,
        handler_kind: &input.handler_kind,
        owner_namespace: &input.owner_namespace,
        handler_profile: &input.handler_profile,
        registry_entry_ref: registry_entry_ref.as_deref(),
        generation: evaluation.mutation.generation,
        previous_generation: evaluation.mutation.previous_generation,
        authority_refs: &input.authority_refs,
        policy_refs: &input.policy_refs,
        resource_refs: &input.resource_refs,
        evidence_refs: &input.evidence_refs,
        shutdown_evidence_ref: input.shutdown_evidence_ref.as_deref(),
        diagnostics: &evaluation.diagnostics,
    })?;
    Ok(RouterDecision {
        decision,
        operation: input.operation.clone(),
        alpn: input.alpn.clone(),
        outcome: evaluation.mutation.outcome,
        generation: evaluation.mutation.generation,
        previous_generation: evaluation.mutation.previous_generation,
        diagnostics: evaluation.diagnostics,
        registry: evaluation.mutation.registry,
        registry_entry_ref,
        receipt_value,
    })
}

struct FrameEvaluation {
    actual_ref: Option<String>,
    diagnostics: Vec<String>,
}

struct FrameEvaluator<'a> {
    registry: &'a ProtocolRegistry,
    input: &'a FramedEnvelopeInput,
    diagnostics: DiagnosticLog,
}
