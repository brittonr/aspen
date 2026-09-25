
#[derive(Debug, Default)]
struct TransferStep {
    envelope_ref: Option<String>,
    operation_ref: Option<String>,
    send_receipt_ref: Option<String>,
    send_receipt_value: Option<IoValue>,
    diagnostics: Vec<String>,
}

struct FinishInput<'a> {
    input: &'a ControlLiveWorkflowBundleApplyInput<'a>,
    verified: ControlLiveWorkflowBundleVerify,
    expected: LiveWorkflowBundleExpectedInput<'a>,
    gate_receipt_ref: Option<String>,
    import_receipt_ref: Option<String>,
    imported_refs: Vec<String>,
    envelope_ref: Option<String>,
    operation_ref: Option<String>,
    send_receipt_ref: Option<String>,
    send_receipt_value: Option<IoValue>,
    diagnostics: Vec<String>,
}
