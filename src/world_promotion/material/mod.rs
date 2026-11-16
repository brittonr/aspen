//! Content-addressed effect-intent material under the node storage namespace.
//!
//! Publishing material makes it available; it grants no authority. Promotion
//! reads the material for every plan intent at application time.

use molten_core::world_promotion::WorldEffectIntent;
use molten_core::world_promotion::WorldEffectIntentRef;
use molten_node_host::node_state::RelativePath;
use molten_node_host::node_state::Scope;

use super::WorldEffectIntentMaterial;
use super::canonical_intent_material;
use super::parse_intent_material;
use crate::error::Failure;
use crate::error::Result;

const MATERIAL_DIRECTORY: &str = "world-promotion/intent-material";
const MAX_INTENT_MATERIAL_BYTES: u64 = 1 << 16;
const BLAKE3_PREFIX: &str = "blake3:";

/// Intent material addressed by the BLAKE3 of its canonical Preserves bytes.
pub struct LocalIntentMaterial {
    storage: Scope,
}

impl LocalIntentMaterial {
    pub const fn from_storage(storage: Scope) -> Self {
        Self { storage }
    }

    /// Stores canonical intent material and returns its intent reference.
    /// Republishing identical material is idempotent; a stored file whose bytes
    /// differ from its address is reported, never overwritten.
    pub fn publish(&mut self, material: &WorldEffectIntentMaterial) -> Result<WorldEffectIntentRef> {
        let canonical = canonical_intent_material(material)?;
        let intent_ref = WorldEffectIntentRef::new(canonical.record_ref.clone())
            .map_err(|error| Failure::invalid_harness(format!("invalid intent material reference: {error:?}")))?;
        let path = material_path(&intent_ref)?;
        if self.storage.try_exists(&path)? {
            return if self.storage.read(&path, MAX_INTENT_MATERIAL_BYTES)? == canonical.bytes {
                Ok(intent_ref)
            } else {
                Err(Failure::invalid_harness("stored intent material does not match its content address"))
            };
        }
        self.storage.create_dir_all(&RelativePath::parse(MATERIAL_DIRECTORY)?)?;
        self.storage.write(&path, &canonical.bytes)?;
        Ok(intent_ref)
    }

    /// Reads the material stored for one intent reference. Absent material,
    /// non-canonical bytes, and bytes whose content reference differs from the
    /// requested reference are all unavailable (`None`).
    pub fn read(&self, intent_ref: &WorldEffectIntentRef) -> Result<Option<WorldEffectIntent>> {
        let path = material_path(intent_ref)?;
        if !self.storage.try_exists(&path)? {
            return Ok(None);
        }
        let bytes = self.storage.read(&path, MAX_INTENT_MATERIAL_BYTES)?;
        Ok(parse_intent_material(&bytes).ok().filter(|intent| intent.intent_ref == *intent_ref))
    }
}

fn material_path(intent_ref: &WorldEffectIntentRef) -> Result<RelativePath> {
    let digest = intent_ref
        .as_str()
        .strip_prefix(BLAKE3_PREFIX)
        .ok_or_else(|| Failure::invalid_harness("intent material reference is not a BLAKE3 reference"))?;
    RelativePath::parse(&format!("{MATERIAL_DIRECTORY}/{digest}.preserves"))
}

#[cfg(test)]
mod tests {
    use molten_core::world_promotion::WorldIntentReleaseClass;
    use molten_core::world_promotion::WorldPromotionAdapterRef;
    use molten_core::world_promotion::WorldPromotionHandlerRef;
    use molten_core::world_promotion::WorldSemanticIntentRef;
    use molten_node_host::node_state::DirectoryRoot;
    use molten_node_host::node_state::NamespaceKind;

    use super::*;

    fn reference(label: &str) -> String {
        format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
    }

    fn intent() -> WorldEffectIntentMaterial {
        WorldEffectIntentMaterial {
            semantic_ref: WorldSemanticIntentRef::new(reference("semantic")).expect("semantic"),
            handler_ref: WorldPromotionHandlerRef::new(reference("handler")).expect("handler"),
            adapter_ref: WorldPromotionAdapterRef::new(reference("adapter")).expect("adapter"),
            release_class: WorldIntentReleaseClass::Release,
        }
    }

    fn store() -> (cap_tempfile::TempDir, LocalIntentMaterial) {
        let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
        let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone root"));
        root.create_layout().expect("state layout");
        let material = LocalIntentMaterial::from_storage(root.namespace(NamespaceKind::Storage).expect("storage"));
        (temporary, material)
    }

    // r[verify molten.world_promotion.transaction]
    #[test]
    fn published_material_reads_back_under_its_content_address() {
        let (_temporary, mut material) = store();
        let intent_ref = material.publish(&intent()).expect("publish");
        assert_eq!(material.publish(&intent()).expect("idempotent publish"), intent_ref);
        let read = material.read(&intent_ref).expect("read").expect("available");
        assert_eq!(read.intent_ref, intent_ref);
        assert_eq!(read.adapter_ref, intent().adapter_ref);
        assert_eq!(read.release_class, Some(WorldIntentReleaseClass::Release));
    }

    // r[verify molten.world_promotion.transaction]
    #[test]
    fn missing_tampered_or_readdressed_material_is_unavailable() {
        let (_temporary, mut material) = store();
        let intent_ref = material.publish(&intent()).expect("publish");
        let absent = WorldEffectIntentRef::new(reference("absent")).expect("absent ref");
        assert!(material.read(&absent).expect("read absent").is_none());

        let mut other = intent();
        other.adapter_ref = WorldPromotionAdapterRef::new(reference("other-adapter")).expect("adapter");
        let other_bytes = canonical_intent_material(&other).expect("other material").bytes;
        material.storage.write(&material_path(&intent_ref).expect("path"), &other_bytes).expect("readdress");
        assert!(material.read(&intent_ref).expect("read readdressed").is_none());
        assert!(material.publish(&intent()).is_err());

        material.storage.write(&material_path(&intent_ref).expect("path"), b"tampered").expect("tamper");
        assert!(material.read(&intent_ref).expect("read tampered").is_none());
    }
}
