//! Canonical projection of an admitted Linux observation window into fabric artifacts.
use super::*;

pub struct EbpfArtifacts {
    pub samples: Vec<Artifact<MetricSample>>,
    pub health: Artifact<HealthInput>,
    pub complete: bool,
}

// r[impl linux_ebpf_observability.projection]
pub fn canonical_ebpf_window(
    profile: &ObservationProfile,
    adapter: &EbpfProfile,
    window: &EbpfWindow,
    context: &ObservationContext,
    now: u64,
) -> crate::error::Result<EbpfArtifacts> {
    let projection = project_ebpf_window(profile, adapter, window, context, now)
        .map_err(|_| crate::error::Failure::invalid_harness("Linux observation window denied"))?;
    let mut samples = Vec::with_capacity(projection.samples.len());
    for sample in &projection.samples {
        let descriptor = adapter
            .mappings
            .iter()
            .find(|mapping| mapping.descriptor.descriptor_ref == sample.descriptor_ref)
            .ok_or_else(|| crate::error::Failure::invalid_harness("Linux observation descriptor missing"))?;
        let seed = metric_sample(profile, &descriptor.descriptor, sample, now)?;
        let mut canonical = seed.artifact;
        canonical.sample_ref = seed.artifact_ref;
        samples.push(metric_sample(profile, &descriptor.descriptor, &canonical, now)?);
    }
    Ok(EbpfArtifacts {
        samples,
        health: health_input(profile, &projection.health, now)?,
        complete: projection.complete,
    })
}
