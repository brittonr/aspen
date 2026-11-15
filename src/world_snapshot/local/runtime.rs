use molten_core::world_snapshot::SnapshotComponent;
use molten_core::world_snapshot::SnapshotComponentKind;
use molten_core::world_snapshot::SnapshotRestoreStep;
use redb::ReadableDatabase;
use redb::ReadableTable;

use super::local::LocalLogicalSnapshot;
use super::localprofile::LogicalRuntimeImage;
use super::*;
use crate::error::Failure;
use crate::error::Result;
use crate::local_store::DurableStoreRoot;
use crate::local_store::Locator;
use crate::node_state::DirectoryRoot;
use crate::runtime::RuntimeState;

const TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("logical_runtime_restore_v1");
const STATE_STEPS: &[SnapshotComponentKind] = &[
    SnapshotComponentKind::DurableState,
    SnapshotComponentKind::Tasks,
    SnapshotComponentKind::History,
    SnapshotComponentKind::Effects,
    SnapshotComponentKind::Scheduler,
    SnapshotComponentKind::Time,
    SnapshotComponentKind::Entropy,
];

struct Activation<'a> {
    descriptor_ref: &'a str,
    descriptor: &'a [u8],
    image: &'a [u8],
    activation: &'a [u8],
    generation: u64,
}

pub(super) struct RuntimeStore {
    database: redb::Database,
}

impl RuntimeStore {
    pub(super) fn open(root: &DirectoryRoot) -> Result<Self> {
        Self::open_mode(root, true)
    }

    fn open_mode(root: &DirectoryRoot, allow_create: bool) -> Result<Self> {
        let store = DurableStoreRoot::from_ledger(&root.ledger_store()?, &Locator::parse("world-snapshot")?)?;
        let path = Locator::parse("runtime.redb")?;
        let is_existing = store.root().try_exists(&path)?;
        if !is_existing && !allow_create {
            return Err(Failure::invalid_harness("no retained logical runtime store"));
        }
        let file = if is_existing {
            store.root().open_database_file(&path)?
        } else {
            store.root().create_database_file(&path)?
        };
        if is_existing && file.metadata().map_err(store_error)?.len() == 0 {
            return Err(Failure::invalid_harness("existing runtime store is empty; reconciliation required"));
        }
        let mut builder = redb::Database::builder();
        if is_existing {
            builder.set_repair_callback(|repair| repair.abort());
        }
        let database = builder.create_file(file).map_err(store_error)?;
        if is_existing {
            database.begin_read().map_err(store_error)?.open_table(TABLE).map_err(store_error)?;
        } else {
            let write = database.begin_write().map_err(store_error)?;
            {
                write.open_table(TABLE).map_err(store_error)?;
            }
            write.commit().map_err(store_error)?;
        }
        Ok(Self { database })
    }

    pub(super) fn verify_handle(&self) -> Result<()> {
        self.database.begin_read().map_err(store_error)?.open_table(TABLE).map_err(store_error)?;
        Ok(())
    }

    fn read(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let read = self.database.begin_read().map_err(store_error)?;
        let table = read.open_table(TABLE).map_err(store_error)?;
        let value = table.get(key).map_err(store_error)?;
        let Some(value) = value else {
            return Ok(None);
        };
        let length = u64::try_from(value.value().len()).map_err(store_error)?;
        if length > MAX_LOCAL_LOGICAL_IMAGE_BYTES {
            return Err(Failure::invalid_harness("stored runtime state exceeds byte bound"));
        }
        Ok(Some(value.value().to_vec()))
    }

    fn stage(&self, descriptor: &str, component: &SnapshotComponent, bytes: &[u8]) -> Result<()> {
        let key = format!("staged/{descriptor}/{}", component.kind.as_str());
        let write = self.database.begin_write().map_err(store_error)?;
        {
            let mut table = write.open_table(TABLE).map_err(store_error)?;
            if let Some(prior) = table.get(key.as_str()).map_err(store_error)?
                && prior.value() != bytes
            {
                return Err(Failure::invalid_harness("staged runtime component changed"));
            }
            table.insert(key.as_str(), bytes).map_err(store_error)?;
        }
        write.commit().map_err(store_error)?;
        if self.read(&key)?.as_deref() != Some(bytes) {
            return Err(Failure::invalid_harness("staged runtime component readback differs before activation"));
        }
        Ok(())
    }

    fn activate(&self, input: Activation<'_>, commit_started: &mut bool) -> Result<()> {
        let Activation {
            descriptor_ref,
            descriptor,
            image,
            activation,
            generation,
        } = input;
        let write = self.database.begin_write().map_err(store_error)?;
        {
            let mut table = write.open_table(TABLE).map_err(store_error)?;
            if let Some(active) = table.get("active-descriptor").map_err(store_error)?
                && active.value() != descriptor_ref.as_bytes()
            {
                return Err(Failure::invalid_harness(
                    "destination already has a different active world; replacement requires separate authority",
                ));
            }
            if let Some(previous) = table.get("activation").map_err(store_error)? {
                let decoded = crate::preserves_rail::strict_canonical_decode(previous.value())?;
                let fields =
                    crate::preserves_rail::simple_record_fields(&decoded.value, "runtime-spine-activation-v1", 6)?;
                let previous_generation = fields[4]
                    .as_u64()
                    .ok_or_else(|| Failure::invalid_harness("retained activation generation missing"))?
                    .map_err(store_error)?;
                if crate::preserves_rail::required_string_field(&fields[0], "retained activation descriptor")?
                    != descriptor_ref
                    || previous_generation == 0
                    || generation < previous_generation
                    || (generation == previous_generation && previous.value() != activation)
                {
                    return Err(Failure::invalid_harness(
                        "retained activation admission/generation cannot regress or change in place",
                    ));
                }
                if previous.value() == activation {
                    return Ok(());
                }
            }
            table.insert("descriptor", descriptor).map_err(store_error)?;
            table.insert("image", image).map_err(store_error)?;
            table.insert("activation", activation).map_err(store_error)?;
            table.insert("active-descriptor", descriptor_ref.as_bytes()).map_err(store_error)?;
        }
        *commit_started = true;
        write.commit().map_err(store_error)
    }
}

impl LogicalSnapshotRestorePort for LocalLogicalSnapshot {
    fn restore_component(
        &mut self,
        step: SnapshotRestoreStep,
        component: &SnapshotComponent,
        materialization: &SnapshotMaterializationObservation,
    ) -> Result<SnapshotStepObservation> {
        if self.state.borrow().store.is_none() {
            self.verified_image()?;
            let store = RuntimeStore::open(&self.root)?;
            self.state.borrow_mut().store = Some(store);
        }
        let state = self.state.borrow();
        let bytes = state
            .verified
            .get(&component.kind)
            .ok_or_else(|| Failure::invalid_harness("unverified runtime component"))?;
        let identity = crate::preserves_rail::content_ref_from_bytes(bytes);
        if identity != materialization.observation_ref || identity != component.identity {
            return Err(Failure::invalid_harness("runtime component materialization changed"));
        }
        state
            .store
            .as_ref()
            .ok_or_else(|| Failure::invalid_harness("live runtime store handle was not recreated"))?
            .stage(&state.descriptor_ref, component, bytes)?;
        drop(state);
        self.state.borrow_mut().restored.insert(component.kind);
        Ok(SnapshotStepObservation {
            step,
            observation_ref: identity,
        })
    }

    fn activate(&mut self, descriptor_ref: &str) -> Result<String> {
        let image = self.verified_image()?;
        let image_bytes = image.canonical_bytes()?;
        let runtime_state_ref = image.snapshot.snapshot_ref()?;
        let runtime = RuntimeState::from_snapshot(image.snapshot)?;
        let mut state = self.state.borrow_mut();
        if state.restored.len() != STATE_STEPS.len() || STATE_STEPS.iter().any(|kind| !state.restored.contains(kind)) {
            return Err(Failure::invalid_harness("runtime reconstruction is incomplete"));
        }
        let descriptor =
            state.descriptor.as_ref().ok_or_else(|| Failure::invalid_harness("restore descriptor missing"))?;
        let descriptor_bytes = canonical_snapshot_descriptor(descriptor)?.bytes;
        let store = state.store.as_ref().ok_or_else(|| Failure::invalid_harness("runtime handle missing"))?;
        // A different active descriptor is a known pre-effect refusal, not an
        // uncertain commit. The database lock is held for this adapter lifetime.
        if store.read("active-descriptor")?.is_some_and(|active| active != descriptor_ref.as_bytes()) {
            return Err(Failure::invalid_harness("destination already has a different active world"));
        }
        let admission = self.admitted(descriptor, descriptor_ref)?;
        if state.admission_ref.as_ref() != Some(&admission.admission_ref) {
            return Err(Failure::invalid_harness("runtime activation currentness changed"));
        }
        let activation = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
            "runtime-spine-activation-v1",
            vec![
                crate::preserves_rail::string(descriptor_ref),
                crate::preserves_rail::string(&self.destination_ref),
                crate::preserves_rail::string(runtime_state_ref),
                crate::preserves_rail::string(&admission.admission_ref),
                crate::preserves_rail::u64_value(admission.generation),
                crate::preserves_rail::string("unresolved-effects-not-dispatched"),
            ],
        ))?;
        {
            let super::local::RestoreState {
                store,
                activation_attempted,
                ..
            } = &mut *state;
            store.as_ref().ok_or_else(|| Failure::invalid_harness("runtime handle missing"))?.activate(
                Activation {
                    descriptor_ref,
                    descriptor: &descriptor_bytes,
                    image: &image_bytes,
                    activation: &activation,
                    generation: admission.generation,
                },
                activation_attempted,
            )?;
        }
        state.activated = true;
        state.runtime = Some(runtime);
        // Reopen/read through the actual typed store, not the bundle or supplied descriptor.
        let stored = state
            .store
            .as_ref()
            .ok_or_else(|| Failure::invalid_harness("runtime handle missing"))?
            .read("image")?
            .ok_or_else(|| Failure::invalid_harness("activated image missing after commit"))?;
        if stored != image_bytes {
            return Err(Failure::invalid_harness("activated image readback differs"));
        }
        Ok(crate::preserves_rail::content_ref_from_bytes(&activation))
    }
}

include!("runtime/readback.rs");

fn store_error(error: impl std::fmt::Display) -> Failure {
    Failure::invalid_harness(format!("logical runtime Redb store: {error}"))
}
