//! The lineage producer identity and the context refs bound into each link, including refs found in
//! details.

pub(super) fn lineage_producer() -> crate::error::Result<crate::evidence_chain::ChainProducer> {
    Ok(crate::evidence_chain::ChainProducer::new(
        "molten-chunk-lineage",
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("chunk-lineage-producer-key", vec![
            crate::preserves_rail::string("molten"),
        ]))?,
    ))
}

pub(super) fn lineage_context_refs(
    manifest: &crate::blocks::ChunkManifest,
    receipt: &crate::blocks::ChunkReceipt,
) -> crate::error::Result<Vec<crate::evidence_chain::ChainContextRef>> {
    let mut refs = vec![
        crate::evidence_chain::ChainContextRef::new("manifest", manifest.manifest_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("chunk-root", manifest.root_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("metadata", manifest.metadata_ref.clone()),
        crate::evidence_chain::ChainContextRef::new(
            "operation",
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("chunk-lineage-operation", vec![
                crate::preserves_rail::string(&receipt.operation),
            ]))?,
        ),
    ];
    for chunk in &manifest.chunks {
        crate::bounded::push(
            &mut refs,
            crate::evidence_chain::ChainContextRef::new("chunk", chunk.chunk_ref.clone()),
            crate::blocks::limits::MAX_CHUNK_STORE_CONTEXT_REFS,
            "chunk lineage context refs",
        )?;
    }
    for detail in &receipt.details {
        collect_detail_context_refs(DetailContextRefsInput {
            value: detail,
            refs: &mut refs,
        })?;
    }
    Ok(refs)
}

struct DetailContextRefsInput<'a> {
    value: &'a preserves::IOValue,
    refs: &'a mut Vec<crate::evidence_chain::ChainContextRef>,
}

fn collect_detail_context_refs(input: DetailContextRefsInput<'_>) -> crate::error::Result<()> {
    let mut pending = Vec::with_capacity(1);
    crate::bounded::push(
        &mut pending,
        input.value.clone(),
        crate::blocks::limits::MAX_CHUNK_STORE_CONTEXT_REFS,
        "chunk lineage detail scan values",
    )?;
    let refs = input.refs;
    while let Some(current) = pending.pop() {
        if let Some(text) = current.as_string() {
            collect_detail_context_refs_push_text(DetailTextInput {
                text: text.into_owned(),
                refs,
            })?;
            continue;
        }
        collect_detail_context_refs_push_children(DetailChildInput {
            value: &current,
            pending: &mut pending,
        })?;
    }
    Ok(())
}

struct DetailTextInput<'a> {
    text: String,
    refs: &'a mut Vec<crate::evidence_chain::ChainContextRef>,
}

fn collect_detail_context_refs_push_text(input: DetailTextInput<'_>) -> crate::error::Result<()> {
    if crate::preserves_rail::validate_content_ref(&input.text).is_ok() {
        crate::bounded::push(
            input.refs,
            crate::evidence_chain::ChainContextRef::new("detail-ref", input.text),
            crate::blocks::limits::MAX_CHUNK_STORE_CONTEXT_REFS,
            "chunk lineage detail refs",
        )?;
    } else if input.text.starts_with("iroh-local-chunk:") {
        crate::bounded::push(
            input.refs,
            crate::evidence_chain::ChainContextRef::new(
                "ticket",
                crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("iroh-ticket", vec![
                    crate::preserves_rail::string(input.text),
                ]))?,
            ),
            crate::blocks::limits::MAX_CHUNK_STORE_CONTEXT_REFS,
            "chunk lineage detail refs",
        )?;
    }
    Ok(())
}

struct DetailChildInput<'a> {
    value: &'a preserves::IOValue,
    pending: &'a mut Vec<preserves::IOValue>,
}

fn collect_detail_context_refs_push_children(input: DetailChildInput<'_>) -> crate::error::Result<()> {
    if let Some(sequence) = input.value.collect_sequence() {
        for item in sequence.iter().rev() {
            collect_detail_context_refs_push_child(DetailPushInput {
                values: input.pending,
                value: crate::preserves_rail::value_to_iovalue(item),
            })?;
        }
        return Ok(());
    }
    match input.value.value_class() {
        preserves::ValueClass::Atomic(_) | preserves::ValueClass::Embedded => {}
        preserves::ValueClass::Compound(preserves::CompoundClass::Record)
        | preserves::ValueClass::Compound(preserves::CompoundClass::Sequence)
        | preserves::ValueClass::Compound(preserves::CompoundClass::Set) => {
            let mut children = Vec::new();
            for child in input.value.iter() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: &mut children,
                    value: crate::preserves_rail::value_to_iovalue(&child),
                })?;
            }
            for child in children.into_iter().rev() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: input.pending,
                    value: child,
                })?;
            }
        }
        preserves::ValueClass::Compound(preserves::CompoundClass::Dictionary) => {
            let mut children = Vec::new();
            for (key, value) in input.value.entries() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: &mut children,
                    value: crate::preserves_rail::value_to_iovalue(&key),
                })?;
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: &mut children,
                    value: crate::preserves_rail::value_to_iovalue(&value),
                })?;
            }
            for child in children.into_iter().rev() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: input.pending,
                    value: child,
                })?;
            }
        }
    }
    Ok(())
}

struct DetailPushInput<'a> {
    values: &'a mut Vec<preserves::IOValue>,
    value: preserves::IOValue,
}

fn collect_detail_context_refs_push_child(input: DetailPushInput<'_>) -> crate::error::Result<()> {
    crate::bounded::push(
        input.values,
        input.value,
        crate::blocks::limits::MAX_CHUNK_STORE_CONTEXT_REFS,
        "chunk lineage detail scan values",
    )
}
