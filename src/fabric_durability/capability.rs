impl RedbDurableStateAdapter {
    /// Open a live journal below an already admitted storage capability.
    pub fn open_root(
        root: crate::local_store::DurableStoreRoot,
        profile: DurableProfile,
        descriptor: DurableNamespaceDescriptor,
    ) -> crate::error::Result<Self> {
        validate_live_descriptor(&profile, &descriptor)?;
        Self::open_root_with(root, profile, descriptor, DatabaseSource::File, PublicationSource::Linux)
    }
}

fn validate_live_descriptor(profile: &DurableProfile, descriptor: &DurableNamespaceDescriptor) -> crate::error::Result<()> {
    if profile.profile.adapter_kind != DurableAdapterKind::LiveRedb {
        return Err(crate::error::Failure::invalid_harness("Redb durability adapter requires a live Redb profile"));
    }
    validate_namespace_descriptor(&profile.profile, descriptor)
        .map_err(|issues| adapter_validation_error("namespace", &issues))
}
