pub struct LocalRuntimeReadback {
    pub descriptor_ref: String,
    pub activation_ref: String,
    pub runtime: RuntimeState,
    pub history: Vec<preserves::IOValue>,
    pub unresolved_effects: Vec<crate::runtime::RuntimeEvent>,
    pub canonical_image: Vec<u8>,
}

/// Effect-free process-restart recovery of an already activated selected profile.
pub fn read_local_logical_runtime(root: &DirectoryRoot) -> Result<LocalRuntimeReadback> {
    let store = RuntimeStore::open_mode(root, false)?;
    let descriptor_bytes =
        store.read("descriptor")?.ok_or_else(|| Failure::invalid_harness("no active runtime descriptor"))?;
    let (descriptor, canonical) = parse_canonical_snapshot_descriptor(&descriptor_bytes)?;
    if descriptor.profile_ref.as_str() != local_logical_profile_ref()?
        || descriptor.cohort != observe_local_logical_cohort()?
    {
        return Err(Failure::invalid_harness("activated runtime profile/cohort differs from observed process"));
    }
    if store.read("active-descriptor")?.as_deref() != Some(canonical.artifact_ref.as_bytes()) {
        return Err(Failure::invalid_harness("active runtime descriptor crossed"));
    }
    let canonical_image =
        store.read("image")?.ok_or_else(|| Failure::invalid_harness("active runtime image missing"))?;
    let image = LogicalRuntimeImage::from_bytes(&canonical_image)?;
    let projections = image.projections(&descriptor.cohort)?;
    for component in &descriptor.components {
        let bytes = projections
            .get(&component.kind)
            .ok_or_else(|| Failure::invalid_harness("active inventory missing"))?;
        if crate::preserves_rail::content_ref_from_bytes(bytes) != component.identity {
            return Err(Failure::invalid_harness("active runtime component identity changed"));
        }
    }
    let activation = store
        .read("activation")?
        .ok_or_else(|| Failure::invalid_harness("active runtime activation missing"))?;
    let decoded = crate::preserves_rail::strict_canonical_decode(&activation)?;
    let fields = crate::preserves_rail::simple_record_fields(&decoded.value, "runtime-spine-activation-v1", 6)?;
    let generation = fields[4]
        .as_u64()
        .ok_or_else(|| Failure::invalid_harness("activation generation missing"))?
        .map_err(store_error)?;
    if crate::preserves_rail::required_string_field(&fields[0], "activation descriptor")? != canonical.artifact_ref
        || crate::preserves_rail::required_string_field(&fields[1], "activation destination")?
            != super::local::destination_identity(root)?
        || crate::preserves_rail::required_string_field(&fields[2], "activation runtime state")?
            != image.snapshot.snapshot_ref()?
        || crate::preserves_rail::required_string_field(&fields[5], "activation effects")?
            != "unresolved-effects-not-dispatched"
        || generation == 0
    {
        return Err(Failure::invalid_harness("activation readback binding differs from reconstructed runtime"));
    }
    crate::preserves_rail::required_content_ref_string(&fields[3], "activation admission")?;
    Ok(LocalRuntimeReadback {
        descriptor_ref: canonical.artifact_ref,
        activation_ref: crate::preserves_rail::content_ref_from_bytes(&activation),
        runtime: RuntimeState::from_snapshot(image.snapshot)?,
        history: image.history,
        unresolved_effects: image.unresolved_effects,
        canonical_image,
    })
}

