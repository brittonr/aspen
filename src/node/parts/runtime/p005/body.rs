#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutdownAdmissionInput<'a> {
    pub request: &'a ControlRequest,
    pub startup_receipt_ref: &'a str,
    pub adapter_receipts: &'a [NodeAdapterReceiptRef],
    pub has_active_lock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutdownPlan {
    pub startup_receipt_ref: String,
    pub adapter_receipts: Vec<NodeAdapterReceiptRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutdownAdmission {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub plan: Option<ShutdownPlan>,
}

const MAX_SHUTDOWN_ADMISSION_ADAPTERS: usize = MAX_NODE_ADAPTERS;

const _: () = assert!(MAX_SHUTDOWN_ADMISSION_ADAPTERS >= REQUIRED_RUNTIME_ADAPTERS.len());

// r[impl molten.audit_f01.admission]
pub fn admit_node_shutdown(input: &ShutdownAdmissionInput<'_>) -> Result<ShutdownAdmission> {
    let mut diagnostics = Vec::new();
    push_request_findings(input, &mut diagnostics)?;
    push_adapter_findings(input, &mut diagnostics)?;
    if diagnostics.is_empty() {
        return Ok(ShutdownAdmission {
            decision: "pass".to_string(),
            diagnostics,
            plan: Some(ShutdownPlan {
                startup_receipt_ref: input.startup_receipt_ref.to_string(),
                adapter_receipts: input.adapter_receipts.iter().rev().cloned().collect(),
            }),
        });
    }
    Ok(ShutdownAdmission {
        decision: "deny".to_string(),
        diagnostics,
        plan: None,
    })
}

fn push_request_findings(
    input: &ShutdownAdmissionInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if input.request.operation != "shutdown" {
        push_diagnostic(
            diagnostics,
            format!("node shutdown admission requires shutdown operation, got {}", input.request.operation),
        )?;
    }
    if input.request.authority_refs.is_empty() {
        push_diagnostic(diagnostics, "node control authority refs missing".to_string())?;
    }
    if input.request.policy_refs.is_empty() {
        push_diagnostic(diagnostics, "node control policy refs missing".to_string())?;
    }
    if input.request.resource_refs.is_empty() {
        push_diagnostic(diagnostics, "node control resource refs missing".to_string())?;
    }
    if input.has_active_lock {
        return Ok(());
    }
    push_diagnostic(diagnostics, "node shutdown admission requires an active node lock".to_string())
}

fn push_adapter_findings(
    input: &ShutdownAdmissionInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    ensure_count_at_most(
        input.adapter_receipts.len(),
        MAX_SHUTDOWN_ADMISSION_ADAPTERS,
        "node shutdown admission adapters",
    )?;
    if input.adapter_receipts.is_empty() {
        push_diagnostic(
            diagnostics,
            "node shutdown admission requires startup adapter evidence".to_string(),
        )?;
    }
    if let Err(error) = validate_ref(input.startup_receipt_ref, "node shutdown admission startup receipt ref") {
        push_diagnostic(
            diagnostics,
            format!("node shutdown admission startup receipt ref invalid: {error}"),
        )?;
    }
    for adapter in input.adapter_receipts {
        push_adapter_finding(diagnostics, adapter)?;
    }
    Ok(())
}

fn push_adapter_finding(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    adapter: &NodeAdapterReceiptRef,
) -> Result<()> {
    if let Err(error) = validate_adapter_name(&adapter.name) {
        push_diagnostic(
            diagnostics,
            format!("node shutdown admission adapter {} binding invalid: {error}", adapter.name),
        )?;
    }
    if let Err(error) = validate_ref(&adapter.receipt_ref, "node shutdown admission adapter receipt ref") {
        push_diagnostic(
            diagnostics,
            format!("node shutdown admission adapter {} receipt ref invalid: {error}", adapter.name),
        )?;
    }
    Ok(())
}

fn push_diagnostic(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    diagnostic: String,
) -> Result<()> {
    push_bounded(diagnostics, diagnostic, MAX_NODE_DIAGNOSTICS, "node shutdown admission diagnostics")
}
