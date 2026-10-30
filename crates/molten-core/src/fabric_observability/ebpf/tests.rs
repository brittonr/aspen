use super::*;
use crate::fabric_observability;

fn synthetic_ref(index: u8) -> String {
    format!("blake3:{index:064x}")
}

pub(super) fn fixture() -> (EbpfProfile, EbpfWindow) {
    let binding = EbpfBinding {
        machine: "fixture-machine".into(),
        generation: "fixture-generation".into(),
        pack: format!("onix:blake3:bpf-pack:{:064x}", 20),
        program: synthetic_ref(21),
        object: synthetic_ref(22),
        kernel: format!("onix:blake3:kernel-build:{:064x}", 23),
        btf: synthetic_ref(24),
        schema: EBPF_WINDOW_SCHEMA.into(),
        scope: synthetic_ref(25),
        accounting: synthetic_ref(26),
        adapter: synthetic_ref(27),
    };
    let signals = [
        EbpfSignal::Process,
        EbpfSignal::Scheduler,
        EbpfSignal::Block,
        EbpfSignal::Network,
        EbpfSignal::Pressure,
        EbpfSignal::AdapterHealth,
    ];
    let mappings = signals
        .iter()
        .enumerate()
        .map(|(index, signal)| EbpfMapping {
            signal: *signal,
            descriptor: fabric_observability::MetricDescriptor {
                schema: fabric_observability::METRIC_DESCRIPTOR_SCHEMA.into(),
                descriptor_id: format!("signal-{index}"),
                descriptor_ref: format!("fixture-descriptor-{index}"),
                profile_ref: "fixture-profile".into(),
                name: format!("signal_{index}"),
                unit: "count".into(),
                kind: fabric_observability::MetricKind::Counter,
                aggregation: fabric_observability::MetricAggregation::Sum,
                allowed_label_names: Vec::new(),
                description: "Synthetic normalized records; not live BPF evidence".into(),
            },
        })
        .collect();
    let records = signals
        .iter()
        .enumerate()
        .map(|(index, signal)| EbpfRecord {
            stream: index as u32,
            sequence: 0,
            tick: 5,
            signal: *signal,
            value: (index as u64) + 1,
        })
        .collect();
    let profile = EbpfProfile {
        binding: binding.clone(),
        mappings,
        max_records: 32,
        max_bytes: 4096,
        max_streams: 8,
        window_ticks: 10,
        freshness_ticks: 5,
        required: true,
    };
    let window = EbpfWindow {
        schema: EBPF_WINDOW_SCHEMA.into(),
        binding,
        generation: 1,
        start: 0,
        end: 10,
        records,
        encoded_bytes: 192,
        accounting: Some(EbpfAccounting {
            attempts: 6,
            submitted: 6,
            producer_dropped: 0,
            consumer_dropped: 0,
            pending: 0,
        }),
        final_close_succeeded: true,
        cancelled: false,
        cross_stream_total_order_claimed: false,
    };
    (profile, window)
}

// r[verify linux_ebpf_observability.ordering]
#[test]
fn concurrent_delivery_preserves_all_six_signal_aggregates() {
    let (profile, mut window) = fixture();
    let expected = std::collections::BTreeMap::from([
        (EbpfSignal::Process, 1),
        (EbpfSignal::Scheduler, 2),
        (EbpfSignal::Block, 3),
        (EbpfSignal::Network, 4),
        (EbpfSignal::Pressure, 5),
        (EbpfSignal::AdapterHealth, 6),
    ]);
    assert_eq!(aggregate(&profile, &window), Ok(expected.clone()));
    window.records.reverse();
    assert_eq!(aggregate(&profile, &window), Ok(expected));
    assert_eq!(accounting_complete(&window), Ok(true));
}

// r[verify linux_ebpf_observability.failure]
#[test]
fn accounting_loss_cancellation_and_close_cannot_claim_complete() {
    let (_, mut window) = fixture();
    window.accounting.as_mut().unwrap().attempts = 7;
    window.accounting.as_mut().unwrap().producer_dropped = 1;
    assert_eq!(accounting_complete(&window), Ok(false));
    window.accounting.as_mut().unwrap().attempts = 6;
    window.accounting.as_mut().unwrap().producer_dropped = 0;
    window.cancelled = true;
    assert_eq!(accounting_complete(&window), Ok(false));
    window.cancelled = false;
    window.final_close_succeeded = false;
    assert_eq!(accounting_complete(&window), Ok(false));
    window.accounting = None;
    assert_eq!(accounting_complete(&window), Err(EbpfIssue::Accounting));
}

// r[verify linux_ebpf_observability.verification]
#[test]
fn record_sequence_overflow_and_unknown_signal_are_denied() {
    let (mut profile, mut window) = fixture();
    let mut next = window.records[0].clone();
    next.sequence = 2;
    window.records.push(next);
    assert_eq!(aggregate(&profile, &window), Err(EbpfIssue::Sequence));
    window.records.last_mut().unwrap().sequence = 1;
    window.records.last_mut().unwrap().value = u64::MAX;
    assert_eq!(aggregate(&profile, &window), Err(EbpfIssue::Overflow));
    window.records.pop();
    profile.mappings.pop();
    assert_eq!(aggregate(&profile, &window), Err(EbpfIssue::Mapping));
}

// r[verify linux_ebpf_observability.admission]
#[test]
fn mismatched_generation_schema_scope_and_accounting_are_denied() {
    let (profile, window) = fixture();
    for field in ["generation", "schema", "scope", "accounting"] {
        let mut drift = window.clone();
        match field {
            "generation" => drift.binding.generation.push('x'),
            "schema" => drift.binding.schema.push('x'),
            "scope" => drift.binding.scope.push('x'),
            _ => drift.binding.accounting.push('x'),
        }
        assert_eq!(validate_binding(&profile, &drift), Err(EbpfIssue::Binding));
    }
}

// r[verify linux_ebpf_observability.verification]
#[test]
fn stale_overbound_and_total_order_claims_are_denied() {
    let (profile, mut window) = fixture();
    assert_eq!(validate_window(&profile, &window, 16), Err(EbpfIssue::Stale));
    window.encoded_bytes = profile.max_bytes + 1;
    assert_eq!(validate_window(&profile, &window, 10), Err(EbpfIssue::Bounds));
    window.encoded_bytes = 192;
    window.cross_stream_total_order_claimed = true;
    assert_eq!(validate_window(&profile, &window, 10), Err(EbpfIssue::Ordering));
    window.cross_stream_total_order_claimed = false;
    window.records[0].tick = window.end;
    assert_eq!(aggregate(&profile, &window), Err(EbpfIssue::Stale));
}

// r[verify linux_ebpf_observability.admission]
#[test]
fn pack_and_kernel_content_digests_cannot_substitute_for_canonical_roles() {
    let (profile, _) = fixture();
    let mut binding = profile.binding.clone();
    binding.pack = synthetic_ref(20);
    assert_eq!(validate_ebpf_binding(&binding), Err(EbpfIssue::Binding));
    binding = profile.binding.clone();
    binding.pack = binding.kernel.clone();
    assert_eq!(validate_ebpf_binding(&binding), Err(EbpfIssue::Binding));
    binding = profile.binding;
    binding.kernel = synthetic_ref(23);
    assert_eq!(validate_ebpf_binding(&binding), Err(EbpfIssue::Binding));
}
