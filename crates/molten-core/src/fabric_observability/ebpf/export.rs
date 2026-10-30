use super::super::*;
use super::*;
use crate::fabric_observability;
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EbpfProfileExport {
    pub schema: String,
    pub profile_ref: String,
    pub binding: EbpfBinding,
    pub mappings: Vec<EbpfMappingExport>,
    pub max_records: usize,
    pub max_bytes: usize,
    pub max_streams: usize,
    pub max_queue_records: usize,
    pub max_queue_bytes: u64,
    pub max_series: usize,
    pub max_events: usize,
    pub window_ticks: u64,
    pub freshness_ticks: u64,
    pub required: bool,
    pub loss_policy: String,
    pub event_detail: String,
    pub redaction: String,
    pub scope: String,
    pub non_claims: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EbpfMappingExport {
    pub signal: EbpfSignal,
    pub descriptor_ref: String,
    pub aggregation: String,
    pub labels: Vec<String>,
}
// r[impl linux_ebpf_observability.profile]
pub fn checked_ebpf_profile(
    export: EbpfProfileExport,
    profile: &ObservationProfile,
    descriptors: &[MetricDescriptor],
) -> Result<EbpfProfile, EbpfIssue> {
    validate_export(&export, profile)?;
    if descriptors.len() > 6 || descriptors.len() != export.mappings.len() {
        return Err(EbpfIssue::Mapping);
    }
    let mut mappings = Vec::with_capacity(export.mappings.len());
    for mapping in export.mappings {
        if mapping.aggregation != "sum" || !mapping.labels.is_empty() {
            return Err(EbpfIssue::Mapping);
        }
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.descriptor_ref == mapping.descriptor_ref)
            .ok_or(EbpfIssue::Mapping)?;
        mappings.push(EbpfMapping {
            signal: mapping.signal,
            descriptor: descriptor.clone(),
        });
    }
    let adapter = EbpfProfile {
        binding: export.binding,
        mappings,
        max_records: export.max_records,
        max_bytes: export.max_bytes,
        max_streams: export.max_streams,
        window_ticks: export.window_ticks,
        freshness_ticks: export.freshness_ticks,
        required: export.required,
    };
    validate_ebpf_profile(profile, &adapter)?;
    Ok(adapter)
}

fn validate_export(export: &EbpfProfileExport, profile: &ObservationProfile) -> Result<(), EbpfIssue> {
    if export.schema != "molten.linux-ebpf-observation.profile.v1"
        || export.profile_ref != profile.profile_ref
        || export.scope != "local-component"
        || export.loss_policy != "incomplete-on-any-loss-or-missing-accounting"
        || export.event_detail != "status-only"
        || export.redaction != "deny-unmapped-and-all-free-text"
    {
        return Err(EbpfIssue::Mapping);
    }
    if !export
        .non_claims
        .iter()
        .map(String::as_str)
        .eq(REQUIRED_OBSERVABILITY_NON_CLAIMS.iter().map(|claim| claim.as_str()))
    {
        return Err(EbpfIssue::Canonical);
    }
    if export.mappings.is_empty()
        || export.mappings.len() > 6
        || export.max_events != 1
        || export.max_series == 0
        || export.max_series > 6
        || export.max_series > profile.bounds.max_series
        || export.mappings.len() > export.max_series
        || export.max_queue_records < export.max_records
        || export.max_queue_records > 65536
        || export.max_queue_bytes < export.max_bytes as u64
        || export.max_queue_bytes > profile.bounds.max_queued_bytes
        || export.max_queue_bytes > 4 * 1024 * 1024
        || export.window_ticks > 1_000_000_000_000
        || export.freshness_ticks > 1_000_000_000_000
    {
        return Err(EbpfIssue::Bounds);
    }
    Ok(())
}
pub fn ebpf_observation_profile(
    export: &EbpfProfileExport,
) -> Result<fabric_observability::ObservationProfile, EbpfIssue> {
    if !crate::fabric::valid_blake3_ref(&export.profile_ref) {
        return Err(EbpfIssue::Canonical);
    }
    Ok(fabric_observability::ObservationProfile {
        schema: fabric_observability::OBSERVATION_PROFILE_SCHEMA.into(),
        profile_id: "linux-ebpf".into(),
        profile_ref: export.profile_ref.clone(),
        bounds: fabric_observability::ObservationBounds {
            max_descriptors: 6,
            max_labels_per_sample: 1,
            max_label_name_bytes: 32,
            max_label_value_bytes: 64,
            max_series: 6,
            max_events: 1,
            max_event_detail_bytes: 128,
            max_queued_bytes: 4194304,
            max_snapshots: 1,
            max_scan_items: 1,
            max_findings: 1,
            max_diagnostics: 16,
            min_export_interval_ticks: 1,
        },
        redaction_rules: Vec::new(),
        non_claims: fabric_observability::REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    })
}

pub fn ebpf_metric_descriptors(
    export: &EbpfProfileExport,
) -> Result<Vec<fabric_observability::MetricDescriptor>, EbpfIssue> {
    if export.mappings.is_empty() || export.mappings.len() > 6 {
        return Err(EbpfIssue::Bounds);
    }
    if !crate::fabric::valid_blake3_ref(&export.profile_ref)
        || export.mappings.iter().any(|mapping| !crate::fabric::valid_blake3_ref(&mapping.descriptor_ref))
    {
        return Err(EbpfIssue::Mapping);
    }
    Ok(export
        .mappings
        .iter()
        .map(|mapping| {
            let name = match mapping.signal {
                fabric_observability::EbpfSignal::Process => "linux-ebpf-process-exits",
                fabric_observability::EbpfSignal::Scheduler => "linux-ebpf-scheduler-switches",
                fabric_observability::EbpfSignal::Block => "linux-ebpf-current-task-block-issues",
                fabric_observability::EbpfSignal::Network => "linux-ebpf-current-task-network-queues",
                fabric_observability::EbpfSignal::Pressure => "linux-ebpf-runnable-preemptions",
                fabric_observability::EbpfSignal::AdapterHealth => "linux-ebpf-observed-source-events",
            };
            fabric_observability::MetricDescriptor {
                schema: fabric_observability::METRIC_DESCRIPTOR_SCHEMA.into(),
                descriptor_id: name.into(),
                descriptor_ref: mapping.descriptor_ref.clone(),
                profile_ref: export.profile_ref.clone(),
                name: name.into(),
                unit: "count".into(),
                kind: fabric_observability::MetricKind::Counter,
                aggregation: fabric_observability::MetricAggregation::Sum,
                allowed_label_names: Vec::new(),
                description: "Current-task-cgroup scoped kernel event count; not application correctness or PSI".into(),
            }
        })
        .collect())
}
