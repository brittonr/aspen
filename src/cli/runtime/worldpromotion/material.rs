use std::path::Path;

use molten::error::Failure;
use molten::error::Result;
use molten::world_promotion::LocalIntentMaterial;
use molten::world_promotion::WorldEffectIntentMaterial;
use molten_core::world_promotion::WorldEffectIntent;
use molten_core::world_promotion::WorldEffectIntentRef;
use molten_core::world_promotion::WorldIntentReleaseClass;
use molten_core::world_promotion::WorldPromotionAdapterRef;
use molten_core::world_promotion::WorldPromotionHandlerRef;
use molten_core::world_promotion::WorldSemanticIntentRef;
use molten_node_host::node_state::DirectoryRoot;
use molten_node_host::node_state::NamespaceKind;
use serde::Deserialize;

/// One intent named by a promotion request document.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct IntentDocument {
    intent_ref: String,
    semantic_ref: String,
    handler_ref: String,
    adapter_ref: String,
    release_class: Option<String>,
}

/// Intent material to publish. Its intent reference is derived from the
/// canonical bytes, so the document cannot name one.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntentMaterialDocument {
    semantic_ref: String,
    handler_ref: String,
    adapter_ref: String,
    release_class: String,
}

pub(super) fn parse_intent(document: IntentDocument) -> Result<WorldEffectIntent> {
    Ok(WorldEffectIntent {
        intent_ref: WorldEffectIntentRef::new(document.intent_ref).map_err(super::reference_error)?,
        semantic_ref: WorldSemanticIntentRef::new(document.semantic_ref).map_err(super::reference_error)?,
        handler_ref: WorldPromotionHandlerRef::new(document.handler_ref).map_err(super::reference_error)?,
        adapter_ref: WorldPromotionAdapterRef::new(document.adapter_ref).map_err(super::reference_error)?,
        release_class: document.release_class.as_deref().map(parse_release_class).transpose()?,
    })
}

/// Publishes canonical intent material into the state root's content-addressed
/// material store. Availability grants no authority: promotion still requires
/// the signed closure to name the reference and the plan to match the material.
pub(super) fn publish_intent(state_root: &Path, intent_path: &Path) -> Result<()> {
    let document: IntentMaterialDocument = serde_json::from_slice(&std::fs::read(intent_path)?)
        .map_err(|error| Failure::invalid_harness(format!("parse intent material: {error}")))?;
    let intent = WorldEffectIntentMaterial {
        semantic_ref: WorldSemanticIntentRef::new(document.semantic_ref).map_err(super::reference_error)?,
        handler_ref: WorldPromotionHandlerRef::new(document.handler_ref).map_err(super::reference_error)?,
        adapter_ref: WorldPromotionAdapterRef::new(document.adapter_ref).map_err(super::reference_error)?,
        release_class: parse_release_class(&document.release_class)?,
    };
    let root = DirectoryRoot::open_existing(state_root)?;
    let mut material = LocalIntentMaterial::from_storage(root.namespace(NamespaceKind::Storage)?);
    let intent_ref = material.publish(&intent)?;
    println!("intent_ref={intent_ref}");
    println!("authority_granted=false");
    Ok(())
}

fn parse_release_class(value: &str) -> Result<WorldIntentReleaseClass> {
    WorldIntentReleaseClass::parse(value).ok_or_else(|| Failure::invalid_harness("unsupported intent release class"))
}
