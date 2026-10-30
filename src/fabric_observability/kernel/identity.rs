use super::*;

// r[impl linux_ebpf_observability.ordering]
// r[impl linux_ebpf_observability.projection]
pub fn canonical_ebpf_window_ref(window: &EbpfWindow) -> crate::error::Result<String> {
    if window.records.len() > 65536 || validate_ebpf_binding(&window.binding).is_err() {
        return Err(crate::error::Failure::invalid_harness("Linux window evidence bound denied"));
    }
    let mut records: Vec<_> = window.records.iter().collect();
    records.sort_unstable_by_key(|record| (record.stream, record.sequence));
    if records
        .windows(2)
        .any(|pair| pair[0].stream == pair[1].stream && pair[0].sequence == pair[1].sequence)
    {
        return Err(crate::error::Failure::invalid_harness("Linux evidence duplicate source fact denied"));
    }
    let records = records
        .into_iter()
        .map(|record| {
            crate::preserves_rail::record("linux-ebpf-source-record-v1", vec![
                crate::preserves_rail::u64_value(u64::from(record.stream)),
                crate::preserves_rail::u64_value(record.sequence),
                crate::preserves_rail::u64_value(record.tick),
                crate::preserves_rail::string(record.signal.as_str()),
                crate::preserves_rail::u64_value(record.value),
            ])
        })
        .collect();
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("linux-ebpf-window-evidence-v1", vec![
        binding_value(&window.binding),
        crate::preserves_rail::u64_value(window.generation),
        crate::preserves_rail::u64_value(window.start),
        crate::preserves_rail::u64_value(window.end),
        crate::preserves_rail::sequence(records),
        accounting_value(window.accounting.as_ref())?,
        crate::preserves_rail::bool_value(window.final_close_succeeded),
        crate::preserves_rail::bool_value(window.cancelled),
        crate::preserves_rail::bool_value(window.cross_stream_total_order_claimed),
    ]))
}

pub fn canonical_ebpf_binding_ref(binding: &EbpfBinding) -> crate::error::Result<String> {
    validate_ebpf_binding(binding)
        .map_err(|_| crate::error::Failure::invalid_harness("Linux upstream identity role denied"))?;
    crate::preserves_rail::canonical_hash(&binding_value(binding))
}

fn binding_value(binding: &EbpfBinding) -> preserves::IOValue {
    crate::preserves_rail::record("linux-ebpf-exact-binding-v1", vec![
        crate::preserves_rail::string(&binding.machine),
        crate::preserves_rail::string(&binding.generation),
        crate::preserves_rail::string(&binding.pack),
        crate::preserves_rail::string(&binding.program),
        crate::preserves_rail::string(&binding.object),
        crate::preserves_rail::string(&binding.kernel),
        crate::preserves_rail::string(&binding.btf),
        crate::preserves_rail::string(&binding.schema),
        crate::preserves_rail::string(&binding.scope),
        crate::preserves_rail::string(&binding.accounting),
        crate::preserves_rail::string(&binding.adapter),
    ])
}

fn accounting_value(accounting: Option<&EbpfAccounting>) -> crate::error::Result<preserves::IOValue> {
    let accounting =
        accounting.ok_or_else(|| crate::error::Failure::invalid_harness("Linux evidence accounting unavailable"))?;
    Ok(crate::preserves_rail::record("linux-ebpf-accounting-v1", vec![
        crate::preserves_rail::u64_value(accounting.attempts),
        crate::preserves_rail::u64_value(accounting.submitted),
        crate::preserves_rail::u64_value(accounting.producer_dropped),
        crate::preserves_rail::u64_value(accounting.consumer_dropped),
        crate::preserves_rail::u64_value(accounting.pending),
    ]))
}
