use super::*;

pub const EBPF_WINDOW_SCHEMA: &str = "onixos.bpf-observation.window.v1";

// r[impl linux_ebpf_observability.profile]
pub fn validate_ebpf_profile(
    profile: &super::super::ObservationProfile,
    adapter: &EbpfProfile,
) -> Result<(), EbpfIssue> {
    validate_ebpf_binding(&adapter.binding)?;
    if adapter.max_records == 0
        || adapter.max_records > 65536
        || adapter.max_bytes == 0
        || adapter.max_bytes > 4 * 1024 * 1024
        || adapter.max_streams == 0
        || adapter.max_streams > 4096
        || adapter.window_ticks == 0
        || adapter.freshness_ticks == 0
    {
        return Err(EbpfIssue::Bounds);
    }
    if !super::super::validate_observation_profile(profile).is_empty()
        || adapter.mappings.len() > profile.bounds.max_descriptors
        || adapter.mappings.len() > profile.bounds.max_series
        || adapter
            .mappings
            .iter()
            .any(|mapping| !super::super::validate_metric_descriptor(profile, &mapping.descriptor).is_empty())
    {
        return Err(EbpfIssue::Canonical);
    }
    if adapter.mappings.is_empty() || adapter.mappings.len() > 6 {
        return Err(EbpfIssue::Mapping);
    }
    let signals: std::collections::BTreeSet<_> = adapter.mappings.iter().map(|mapping| mapping.signal).collect();
    let descriptors: std::collections::BTreeSet<_> =
        adapter.mappings.iter().map(|mapping| &mapping.descriptor.descriptor_ref).collect();
    if signals.len() != adapter.mappings.len()
        || descriptors.len() != adapter.mappings.len()
        || adapter.mappings.iter().any(|mapping| {
            mapping.descriptor.aggregation != super::super::MetricAggregation::Sum
                || !mapping.descriptor.allowed_label_names.is_empty()
        })
    {
        return Err(EbpfIssue::Mapping);
    }
    Ok(())
}

pub fn validate_ebpf_binding(binding: &EbpfBinding) -> Result<(), EbpfIssue> {
    if !crate::fabric::valid_fabric_token(&binding.machine)
        || !crate::fabric::valid_fabric_token(&binding.generation)
        || binding.machine.len() > 256
        || binding.generation.len() > 256
        || binding.schema != EBPF_WINDOW_SCHEMA
    {
        return Err(EbpfIssue::Binding);
    }
    if !role_ref(&binding.pack, "onix:blake3:bpf-pack:") || !role_ref(&binding.kernel, "onix:blake3:kernel-build:") {
        return Err(EbpfIssue::Binding);
    }
    let refs = [
        &binding.program,
        &binding.object,
        &binding.btf,
        &binding.scope,
        &binding.accounting,
        &binding.adapter,
    ];
    if refs.iter().any(|value| !crate::fabric::valid_blake3_ref(value)) {
        return Err(EbpfIssue::Binding);
    }
    Ok(())
}

// r[impl linux_ebpf_observability.admission]
pub fn admit_ebpf_window(adapter: &EbpfProfile, window: &EbpfWindow, epoch: u64, now: u64) -> Result<bool, EbpfIssue> {
    super::validate_binding(adapter, window)?;
    super::validate_window(adapter, window, now)?;
    if window.generation == 0 || window.generation != epoch {
        return Err(EbpfIssue::Binding);
    }
    super::aggregate(adapter, window)?;
    super::accounting_complete(window)
}

fn role_ref(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    })
}
