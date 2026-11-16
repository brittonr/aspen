use super::*;

pub const WORLD_EFFECT_INTENT_MATERIAL_SCHEMA: &str = "molten.world-effect-intent-material.v1";
const INTENT_MATERIAL_RECORD: &str = "world-effect-intent-material";
const MATERIAL_FIELDS: u64 = 5;

/// The content of one effect intent. The BLAKE3 content reference of its
/// canonical record is the intent reference that requests, signed closures, and
/// reservations bind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEffectIntentMaterial {
    pub semantic_ref: WorldSemanticIntentRef,
    pub handler_ref: WorldPromotionHandlerRef,
    pub adapter_ref: WorldPromotionAdapterRef,
    pub release_class: WorldIntentReleaseClass,
}

// r[impl molten.world_promotion.plan]
pub fn canonical_intent_material(material: &WorldEffectIntentMaterial) -> Result<CanonicalWorldPromotionRecord> {
    canonical(crate::preserves_rail::record(INTENT_MATERIAL_RECORD, vec![
        crate::preserves_rail::string(WORLD_EFFECT_INTENT_MATERIAL_SCHEMA),
        named("semantic-ref", string(material.semantic_ref.as_str())),
        named("handler-ref", string(material.handler_ref.as_str())),
        named("adapter-ref", string(material.adapter_ref.as_str())),
        named("release-class", string(material.release_class.as_str())),
    ]))
}

/// Parses canonical intent material into the intent it addresses; the intent
/// reference is derived from the bytes, never read from them.
pub fn parse_intent_material(bytes: &[u8]) -> Result<WorldEffectIntent> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = crate::preserves_rail::simple_record_fields(&decoded.value, INTENT_MATERIAL_RECORD, MATERIAL_FIELDS)?;
    require_schema(&fields[0], WORLD_EFFECT_INTENT_MATERIAL_SCHEMA)?;
    let material = WorldEffectIntentMaterial {
        semantic_ref: WorldSemanticIntentRef::new(content_ref(&fields[1], "semantic-ref")?).map_err(reference_error)?,
        handler_ref: WorldPromotionHandlerRef::new(content_ref(&fields[2], "handler-ref")?).map_err(reference_error)?,
        adapter_ref: WorldPromotionAdapterRef::new(content_ref(&fields[3], "adapter-ref")?).map_err(reference_error)?,
        release_class: WorldIntentReleaseClass::parse(&string_field(&fields[4], "release-class")?)
            .ok_or_else(|| Failure::invalid_harness("unsupported world intent release class"))?,
    };
    let canonical = canonical_intent_material(&material)?;
    if canonical.bytes != decoded.canonical_bytes || canonical.bytes != bytes {
        return Err(Failure::invalid_harness("world intent material bytes are not canonical"));
    }
    Ok(WorldEffectIntent {
        intent_ref: WorldEffectIntentRef::new(canonical.record_ref).map_err(reference_error)?,
        semantic_ref: material.semantic_ref,
        handler_ref: material.handler_ref,
        adapter_ref: material.adapter_ref,
        release_class: Some(material.release_class),
    })
}
