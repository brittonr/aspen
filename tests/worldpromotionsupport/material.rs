use std::path::Path;

use molten::world_promotion::WorldEffectIntentMaterial;
use molten::world_promotion::canonical_intent_material;
use molten_core::world_promotion::WorldIntentReleaseClass;
use molten_core::world_promotion::WorldPromotionAdapterRef;
use molten_core::world_promotion::WorldPromotionHandlerRef;
use molten_core::world_promotion::WorldSemanticIntentRef;
use serde_json::json;

use super::reference;
use super::run_success;
use super::write;

/// The content reference of the release intent's material when its adapter is `adapter`.
pub fn reference_for_adapter(adapter: &str) -> String {
    canonical_intent_material(&WorldEffectIntentMaterial {
        semantic_ref: WorldSemanticIntentRef::new(reference("semantic")).expect("semantic"),
        handler_ref: WorldPromotionHandlerRef::new(reference("handler")).expect("handler"),
        adapter_ref: WorldPromotionAdapterRef::new(reference(adapter)).expect("adapter"),
        release_class: WorldIntentReleaseClass::Release,
    })
    .expect("intent material")
    .record_ref
}

/// Publishes the release intent's material with `adapter` through the public
/// command and returns the printed intent reference.
pub fn publish_intent(root: &Path, adapter: &str) -> String {
    let intent_path = root.join(format!("intent-{adapter}.json"));
    write(
        &intent_path,
        json!({
            "semantic_ref": reference("semantic"),
            "handler_ref": reference("handler"),
            "adapter_ref": reference(adapter),
            "release_class": "release",
        }),
    );
    let stdout = run_success(&[
        "publish-intent",
        "--state-root",
        root.to_str().expect("path"),
        "--intent",
        intent_path.to_str().expect("path"),
    ]);
    assert!(stdout.contains("authority_granted=false\n"), "stdout={stdout}");
    let intent_ref = stdout
        .lines()
        .find_map(|line| line.strip_prefix("intent_ref="))
        .expect("intent_ref line")
        .to_string();
    assert_eq!(intent_ref, reference_for_adapter(adapter));
    intent_ref
}
