mod admission;
mod export;
mod lifecycle;
mod model;
mod status;
pub use admission::*;
pub use export::*;
pub use lifecycle::*;
pub use model::*;
pub use status::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod readiness;

// r[impl linux_ebpf_observability.admission]
// r[impl linux_ebpf_observability.projection]
pub fn project_ebpf_window(
    profile: &super::ObservationProfile,
    adapter: &EbpfProfile,
    window: &EbpfWindow,
    context: &super::ObservationContext,
    now: u64,
) -> Result<EbpfProjection, EbpfIssue> {
    validate_binding(adapter, window)?;
    validate_window(adapter, window, now)?;
    validate_ebpf_profile(profile, adapter)?;
    if !super::validate_context(profile, context).is_empty() {
        return Err(EbpfIssue::Canonical);
    }
    if window.generation != context.generation {
        return Err(EbpfIssue::Binding);
    }
    if context.scope != super::ClaimScope::LocalComponent || context.resource_ref != adapter.binding.scope {
        return Err(EbpfIssue::Scope);
    }
    if context.observed_tick != window.end
        || context.valid_until_tick < now
        || context.valid_until_tick > window.end.saturating_add(adapter.freshness_ticks)
    {
        return Err(EbpfIssue::Stale);
    }
    let totals = aggregate(adapter, window)?;
    let complete = accounting_complete(window)?;
    let mut samples = Vec::with_capacity(totals.len());
    for (signal, value) in totals {
        let mapping = adapter.mappings.iter().find(|mapping| mapping.signal == signal).ok_or(EbpfIssue::Mapping)?;
        let sample = super::MetricSample {
            schema: super::METRIC_SAMPLE_SCHEMA.to_string(),
            // A finite declaration seed; the shell derives its observation
            // identity from canonical packed Preserves, not Rust/debug bytes.
            sample_ref: mapping.descriptor.descriptor_ref.clone(),
            descriptor_ref: mapping.descriptor.descriptor_ref.clone(),
            context: context.clone(),
            labels: Vec::new(),
            value: i64::try_from(value).map_err(|_| EbpfIssue::Overflow)?,
        };
        samples.push(
            super::validate_metric_sample(profile, &mapping.descriptor, &sample, now)
                .map_err(|_| EbpfIssue::Canonical)?,
        );
    }
    let health = super::HealthInput {
        schema: super::HEALTH_INPUT_SCHEMA.to_string(),
        health_ref: context.source_ref.clone(),
        context: context.clone(),
        state: if complete {
            super::HealthState::Healthy
        } else {
            super::HealthState::Unavailable
        },
        diagnostic_refs: Vec::new(),
    };
    Ok(EbpfProjection {
        samples,
        health,
        complete,
    })
}

fn validate_binding(adapter: &EbpfProfile, window: &EbpfWindow) -> Result<(), EbpfIssue> {
    let binding = &adapter.binding;
    validate_ebpf_binding(binding)?;
    if binding != &window.binding || window.schema != EBPF_WINDOW_SCHEMA {
        return Err(EbpfIssue::Binding);
    }
    if adapter.mappings.is_empty() || adapter.mappings.len() > 6 {
        return Err(EbpfIssue::Mapping);
    }
    let signals: std::collections::BTreeSet<_> = adapter.mappings.iter().map(|mapping| mapping.signal).collect();
    let descriptors: std::collections::BTreeSet<_> =
        adapter.mappings.iter().map(|mapping| &mapping.descriptor.descriptor_ref).collect();
    if signals.len() != adapter.mappings.len()
        || descriptors.len() != adapter.mappings.len()
        || adapter
            .mappings
            .iter()
            .any(|mapping| mapping.descriptor.aggregation != super::MetricAggregation::Sum)
    {
        return Err(EbpfIssue::Mapping);
    }
    Ok(())
}

fn validate_window(adapter: &EbpfProfile, window: &EbpfWindow, now: u64) -> Result<(), EbpfIssue> {
    if adapter.max_records == 0
        || adapter.max_records > 65536
        || adapter.max_bytes == 0
        || adapter.max_bytes > 4 * 1024 * 1024
        || adapter.max_streams == 0
        || adapter.max_streams > 4096
        || window.records.len() > adapter.max_records
        || window.encoded_bytes > adapter.max_bytes
    {
        return Err(EbpfIssue::Bounds);
    }
    if adapter.window_ticks == 0
        || adapter.freshness_ticks == 0
        || window.end.checked_sub(window.start) != Some(adapter.window_ticks)
        || now < window.end
        || now - window.end > adapter.freshness_ticks
    {
        return Err(EbpfIssue::Stale);
    }
    if window.cross_stream_total_order_claimed {
        return Err(EbpfIssue::Ordering);
    }
    Ok(())
}

// Source-local order is validated independently. Cross-stream permutation does
// not change aggregates; timestamps never manufacture a causal total order.
fn aggregate(
    adapter: &EbpfProfile,
    window: &EbpfWindow,
) -> Result<std::collections::BTreeMap<EbpfSignal, u64>, EbpfIssue> {
    let mut records: Vec<_> = window.records.iter().collect();
    records.sort_unstable_by_key(|record| (record.stream, record.sequence));
    let mut previous: Option<(u32, u64)> = None;
    let mut stream_count = 0;
    let mut totals: std::collections::BTreeMap<EbpfSignal, u64> =
        adapter.mappings.iter().map(|mapping| (mapping.signal, 0)).collect();
    for record in records {
        if record.tick < window.start || record.tick >= window.end {
            return Err(EbpfIssue::Stale);
        }
        if !adapter.mappings.iter().any(|mapping| mapping.signal == record.signal) {
            return Err(EbpfIssue::Mapping);
        }
        match previous {
            Some((stream, sequence)) if stream == record.stream => {
                if sequence.checked_add(1) != Some(record.sequence) {
                    return Err(EbpfIssue::Sequence);
                }
            }
            _ => {
                stream_count += 1;
                if stream_count > adapter.max_streams {
                    return Err(EbpfIssue::Bounds);
                }
            }
        }
        previous = Some((record.stream, record.sequence));
        let total = totals.entry(record.signal).or_default();
        *total = total.checked_add(record.value).ok_or(EbpfIssue::Overflow)?;
    }
    Ok(totals)
}

fn accounting_complete(window: &EbpfWindow) -> Result<bool, EbpfIssue> {
    let accounting = window.accounting.as_ref().ok_or(EbpfIssue::Accounting)?;
    if accounting.submitted.checked_add(accounting.producer_dropped) != Some(accounting.attempts)
        || u64::try_from(window.records.len())
            .ok()
            .and_then(|count| count.checked_add(accounting.consumer_dropped))
            .and_then(|count| count.checked_add(accounting.pending))
            != Some(accounting.submitted)
    {
        return Err(EbpfIssue::Accounting);
    }
    Ok(window.final_close_succeeded
        && !window.cancelled
        && accounting.producer_dropped == 0
        && accounting.consumer_dropped == 0
        && accounting.pending == 0)
}
