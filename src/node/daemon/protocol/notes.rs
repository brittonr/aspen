//! Per-part protocol evidence diagnostics.

pub(super) fn note_gate_part(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    gate: &super::artifacts::ControlLiveWorkflowBundleGateReceipt,
    refs: &super::transcript::ReceiptRefs<'_>,
) {
    super::transcript::note_receipt_decision(
        diagnostics,
        "node control live workflow protocol gate",
        &gate.receipt_ref,
        &gate.decision,
        &gate.diagnostics,
    );
    super::transcript::note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol gate bundle",
        &gate.bundle_ref,
        refs.bundle,
    );
}

pub(super) fn note_apply_part(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    apply: &super::artifacts::ControlLiveWorkflowBundleApplyReceipt,
    refs: &super::transcript::ReceiptRefs<'_>,
) {
    super::transcript::note_receipt_decision(
        diagnostics,
        "node control live workflow protocol apply",
        &apply.receipt_ref,
        &apply.decision,
        &apply.diagnostics,
    );
    super::transcript::note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol apply bundle",
        &apply.bundle_ref,
        refs.bundle,
    );
    super::transcript::note_optional_ref_mismatch(
        diagnostics,
        "node control live workflow protocol apply gate",
        apply.gate_receipt_ref.as_deref(),
        refs.gate,
    );
}

pub(super) fn note_reconcile_part(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    reconcile: &super::artifacts::ControlLiveWorkflowBundleReconcileReceipt,
    refs: &super::transcript::ReceiptRefs<'_>,
) {
    super::transcript::note_receipt_decision(
        diagnostics,
        "node control live workflow protocol reconcile",
        &reconcile.receipt_ref,
        &reconcile.decision,
        &reconcile.diagnostics,
    );
    super::transcript::note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol reconcile apply",
        &reconcile.apply_receipt_ref,
        refs.apply,
    );
    super::transcript::note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol reconcile bundle",
        &reconcile.bundle_ref,
        refs.bundle,
    );
}

pub(super) fn note_ack_part(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    ack: &super::artifacts::ControlLiveWorkflowBundleAck,
    refs: &super::transcript::ReceiptRefs<'_>,
    expected: &super::transcript::ExpectedRefs<'_>,
) {
    if ack.receiver_decision != "pass" {
        diagnostics
            .push_item(format!("node control live workflow protocol ack receiver decision {}", ack.receiver_decision));
        diagnostics.extend_cloned_items(&ack.receiver_diagnostics);
    }
    if !ack.diagnostics.is_empty() {
        diagnostics.extend_cloned_items(&ack.diagnostics);
    }
    super::transcript::note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol ack apply",
        &ack.apply_receipt_ref,
        refs.apply,
    );
    super::transcript::note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol ack reconcile",
        &ack.reconcile_receipt_ref,
        refs.reconcile,
    );
    super::transcript::note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol ack bundle",
        &ack.bundle_ref,
        refs.bundle,
    );
    super::transcript::note_expected_ref(
        diagnostics,
        "node control live workflow protocol ack envelope",
        ack.envelope_ref.as_deref(),
        expected.envelope,
    );
    super::transcript::note_expected_ref(
        diagnostics,
        "node control live workflow protocol ack operation",
        ack.operation_ref.as_deref(),
        expected.operation,
    );
    super::transcript::note_expected_ref(
        diagnostics,
        "node control live workflow protocol ack request",
        ack.request_ref.as_deref(),
        expected.request,
    );
}
