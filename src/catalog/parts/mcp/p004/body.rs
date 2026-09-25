
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
