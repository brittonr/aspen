fn receipt_diagnostics(
    receipts: &[ParsedFaultReceipt],
    descriptors: &[ParsedFaultDescriptor],
    descriptor_refs: &[String],
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let descriptor_ref_set = descriptor_refs.iter().map(String::as_str).collect::<OrderedSet<_>>();
    for receipt in receipts {
        push_if(
            &mut diagnostics,
            !descriptor_ref_set.contains(receipt.descriptor_ref.as_str()),
            "vm-fault-receipt-descriptor-missing",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.host_support != "supported",
            "vm-fault-unavailable-cannot-pass",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.pre_fault_refs.is_empty(),
            "vm-fault-pass-missing-pre-ref",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.injection_refs.is_empty(),
            "vm-fault-pass-missing-injection-ref",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.child_refs.is_empty(),
            "vm-fault-log-only-pass",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.post_fault_refs.is_empty(),
            "vm-fault-pass-missing-post-ref",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision != "pass" && receipt.diagnostics.is_empty(),
            "vm-fault-deny-missing-diagnostic",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "deny"
                && receipt.replay_status == "denial-observed-before-mutation"
                && receipt.child_refs.is_empty(),
            "vm-fault-observed-denial-missing-child-evidence",
        )?;
        push_if(&mut diagnostics, receipt.log_refs.is_empty(), "vm-fault-missing-log-ref")?;
        push_if(&mut diagnostics, receipt.replay_status.trim().is_empty(), "vm-fault-missing-replay-status")?;
        push_if(&mut diagnostics, receipt.caveats.is_empty(), "vm-fault-missing-caveats")?;
        validate_fault_expected_outcome(receipt, descriptors, descriptor_refs, &mut diagnostics)?;
    }
    Ok(diagnostics)
}

fn validate_fault_expected_outcome(
    receipt: &ParsedFaultReceipt,
    descriptors: &[ParsedFaultDescriptor],
    descriptor_refs: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    let Some((descriptor, _)) = descriptors
        .iter()
        .zip(descriptor_refs.iter())
        .find(|(_, descriptor_ref)| descriptor_ref.as_str() == receipt.descriptor_ref)
    else {
        return Ok(());
    };
    push_if(
        diagnostics,
        descriptor.expected_outcome == "unavailable" && receipt.decision == "pass",
        "vm-fault-unavailable-expected-cannot-pass",
    )?;
    push_if(
        diagnostics,
        descriptor.fault_kind == "log-only-pass" && receipt.decision == "pass",
        "vm-fault-log-only-pass",
    )?;
    Ok(())
}
