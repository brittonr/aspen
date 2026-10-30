//! Pure decoding of the actual provider status export, not observation records.
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OnixBpfStatus {
    pub schema_version: u64,
    pub machine_id: String,
    pub active_generation: Option<String>,
    pub generations: Vec<OnixBpfGeneration>,
    pub read_only: bool,
    pub mutable_handles_exposed: bool,
    pub credentials_exposed: bool,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OnixBpfGeneration {
    pub generation: String,
    pub state: String,
    pub pack_identity: String,
    pub kernel_build_identity: String,
    pub btf_blake3: String,
    pub module_blake3: String,
    pub object_blake3: String,
    pub loader_blake3: String,
    pub attach_target: String,
    pub verifier_class: String,
    pub observation_loss: u64,
    pub cleanup_class: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfReadFailure {
    Missing,
    Permission,
    OverBound,
    Malformed,
    Binding,
    Cancelled,
    Closed,
}
pub const ONIX_BPF_STATUS_MAX_BYTES: u64 = 256 * 1024;
const MAX_GENERATIONS: usize = 32;

pub fn validate_onix_bpf_status(status: &OnixBpfStatus) -> Result<(), EbpfReadFailure> {
    if status.schema_version != 1 || !status.read_only || status.mutable_handles_exposed || status.credentials_exposed {
        return Err(EbpfReadFailure::Binding);
    }
    if status.generations.len() > MAX_GENERATIONS {
        return Err(EbpfReadFailure::OverBound);
    }
    if status.machine_id.is_empty() || status.machine_id.len() > 256 {
        return Err(EbpfReadFailure::Binding);
    }
    let mut generations = std::collections::BTreeSet::new();
    for generation in &status.generations {
        if !generations.insert(&generation.generation)
            || !matches!(generation.state.as_str(), "staged" | "active" | "retained" | "degraded" | "cleanup-failed")
        {
            return Err(EbpfReadFailure::Binding);
        }
        let fields = [
            &generation.generation,
            &generation.state,
            &generation.pack_identity,
            &generation.kernel_build_identity,
            &generation.btf_blake3,
            &generation.module_blake3,
            &generation.object_blake3,
            &generation.loader_blake3,
            &generation.attach_target,
            &generation.verifier_class,
            &generation.cleanup_class,
        ];
        if fields.iter().any(|field| field.is_empty() || field.len() > 256) {
            return Err(EbpfReadFailure::Binding);
        }
    }
    if let Some(active) = &status.active_generation
        && status.generations.iter().filter(|generation| &generation.generation == active).count() != 1
    {
        return Err(EbpfReadFailure::Binding);
    }
    Ok(())
}

/// Independently selected expectations; never learned from the report itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnixBpfStatusExpectation {
    pub machine: String,
    pub generation: String,
    pub pack: String,
    pub kernel: String,
    pub btf: String,
    pub object: String,
    pub loader: String,
    pub attachment: String,
}

/// Status admission is NOT observation endpoint admission: schema, source scope,
/// program identity, windows and accounting are absent in this provider export.
pub fn admit_onix_bpf_status(
    status: &OnixBpfStatus,
    expected: &OnixBpfStatusExpectation,
) -> Result<(), EbpfReadFailure> {
    validate_onix_bpf_status(status)?;
    if status.machine_id != expected.machine || status.active_generation.as_ref() != Some(&expected.generation) {
        return Err(EbpfReadFailure::Binding);
    }
    let generation = status
        .generations
        .iter()
        .find(|generation| generation.generation == expected.generation)
        .ok_or(EbpfReadFailure::Binding)?;
    if generation.state != "active"
        || generation.pack_identity != expected.pack
        || generation.kernel_build_identity != expected.kernel
        || generation.btf_blake3 != expected.btf
        || generation.object_blake3 != expected.object
        || generation.loader_blake3 != expected.loader
        || generation.attach_target != expected.attachment
    {
        return Err(EbpfReadFailure::Binding);
    }
    Ok(())
}
