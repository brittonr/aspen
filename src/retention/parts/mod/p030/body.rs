
pub fn reference_index_for_object(input: ReferenceIndexForObjectInput<'_>) -> Result<ReferenceIndex> {
    let root = open_capability_retention_root(input.root)?;
    reference_index_for_object_with_root(ReferenceIndexForObjectInput {
        root: &root,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retained_refs: input.retained_refs,
        remote_refs: input.remote_refs,
        is_complete: input.is_complete,
    })
}

pub fn reference_index_for_object_with_root(
    input: ReferenceIndexForObjectInput<'_, CapabilityRetentionRoot>,
) -> Result<ReferenceIndex> {
    ensure_store_with_root(input.root)?;
    let pins = pins_for_object_with_root(input.root, input.object_ref)?;
    let mut pin_refs = Vec::with_capacity(pins.len());
    for pin in &pins {
        push_bounded(&mut pin_refs, pin.pin_ref.clone(), MAX_RETENTION_REFS, "retention index pin refs")?;
    }
    let tombstone_refs = tombstone_refs_for_object_with_root(input.root, input.object_ref)?;
    let value = reference_index_value(&ReferenceIndexInput {
        object_ref: input.object_ref.to_string(),
        object_kind: input.object_kind.to_string(),
        pin_refs,
        retained_refs: input.retained_refs.to_vec(),
        tombstone_refs,
        remote_refs: input.remote_refs.to_vec(),
        is_complete: input.is_complete,
    })?;
    parse_reference_index(&value)
}
