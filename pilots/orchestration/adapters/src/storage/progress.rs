// SPDX-License-Identifier: AGPL-3.0-or-later
//! Controller progress rows share the controller state's Redb transaction.
use molten_batch_application::Error;
use molten_batch_core::*;
use molten_orchestrator_codec as codec;
use redb::Durability;
use redb::ReadableDatabase;
use redb::ReadableTable;
use redb::Table;
use redb::WriteTransaction;

use super::Disk;
use super::BLOBS;
use super::SNAPSHOTS;
use crate::progress::Journal;
use crate::progress::JournalOwner;
use crate::progress::OWNER_SCHEMA;
use crate::progress::STORAGE_SCHEMA;
use crate::verify_state;

const JOURNAL_PROJECTION: &str = "controller-progress.v1.projection";
const OWNER_PROJECTION: &str = "controller-progress-owner.v1.projection";

fn decode_owner(canonical: Option<&[u8]>, reconstruction: Option<&[u8]>) -> Result<Option<JournalOwner>, Error> {
    match (canonical, reconstruction) {
        (Some(a), Some(b)) => codec::decode(OWNER_SCHEMA, a, b).map(Some).map_err(crate::invalid),
        (None, None) => Ok(None),
        _ => Err(Error::Invalid("incomplete durable progress owner".into())),
    }
}
fn write_owner(table: &mut Table<'_, &str, &[u8]>, owner: &JournalOwner) -> Result<(), Error> {
    let encoded = codec::encode(OWNER_SCHEMA, owner).map_err(crate::invalid)?;
    table.insert(OWNER_SCHEMA, encoded.canonical.as_slice()).map_err(crate::invalid)?;
    table.insert(OWNER_PROJECTION, encoded.projection.as_slice()).map_err(crate::invalid)?;
    Ok(())
}
impl Disk {
    // r[impl molten.observations.stream]
    /// Called only after whole-state CAS matched inside this write transaction.
    pub(super) fn stage_progress(
        &self,
        tx: &WriteTransaction,
        table: &mut Table<'_, &str, &[u8]>,
        previous: &State,
        state: &State,
        old_ref: Id,
        new_ref: Id,
    ) -> Result<bool, Error> {
        let mut owner = {
            let a = table.get(OWNER_SCHEMA).map_err(crate::invalid)?;
            let b = table.get(OWNER_PROJECTION).map_err(crate::invalid)?;
            decode_owner(a.as_ref().map(|g| g.value()), b.as_ref().map(|g| g.value()))?.ok_or(Error::Unavailable)?
        };
        let payloads = progress_delta(Some(previous), state)?;
        let journal = {
            let a = table.get(STORAGE_SCHEMA).map_err(crate::invalid)?.ok_or(Error::Unavailable)?;
            let b = table.get(JOURNAL_PROJECTION).map_err(crate::invalid)?.ok_or(Error::Unavailable)?;
            // Even a state-only transition checks the exact authoritative pair.
            // Hashing its bytes does not reconstruct/revalidate retained events.
            owner.validate(old_ref, a.value(), b.value())?;
            if payloads.is_empty() {
                None
            } else {
                let journal: Journal = codec::decode(STORAGE_SCHEMA, a.value(), b.value()).map_err(crate::invalid)?;
                journal.validate(previous, owner.projected_state_ref)?;
                Some(journal)
            }
        };
        let mut publishes_output = false;
        if let Some(mut journal) = journal {
            let blobs = tx.open_table(BLOBS).map_err(crate::invalid)?;
            for payload in &payloads {
                if let ProgressPayload::AcceptedOutput { output, .. } = payload {
                    publishes_output = true;
                    if blobs.get(output.hex().as_str()).map_err(crate::invalid)?.is_none() {
                        return Err(Error::Unavailable);
                    }
                }
            }
            journal.advance(state, new_ref, payloads)?;
            let encoded = codec::encode(STORAGE_SCHEMA, &journal).map_err(crate::invalid)?;
            table.insert(STORAGE_SCHEMA, encoded.canonical.as_slice()).map_err(crate::invalid)?;
            table.insert(JOURNAL_PROJECTION, encoded.projection.as_slice()).map_err(crate::invalid)?;
            owner = JournalOwner::new(new_ref, &encoded);
        } else {
            // No event publication: keep the journal bytes, retained window and
            // stream position untouched, but atomically advance its state owner.
            owner.state_ref = new_ref;
        }
        write_owner(table, &owner)?;
        Ok(publishes_output)
    }

    pub(super) fn initialize_progress(&self) -> Result<(), Error> {
        let mut tx = self.db.begin_write().map_err(crate::invalid)?;
        tx.set_durability(Durability::Immediate).map_err(crate::invalid)?;
        {
            let mut table = tx.open_table(SNAPSHOTS).map_err(crate::invalid)?;
            let (state, state_ref): (State, Id) = {
                let a = table.get("controller-state.v2").map_err(crate::invalid)?.ok_or(Error::Unavailable)?;
                let b =
                    table.get("controller-state.v2.projection").map_err(crate::invalid)?.ok_or(Error::Unavailable)?;
                (
                    codec::decode("controller-state.v2", a.value(), b.value()).map_err(crate::invalid)?,
                    codec::digest(a.value()),
                )
            };
            verify_state(&state)?;
            let owner = {
                let a = table.get(OWNER_SCHEMA).map_err(crate::invalid)?;
                let b = table.get(OWNER_PROJECTION).map_err(crate::invalid)?;
                decode_owner(a.as_ref().map(|g| g.value()), b.as_ref().map(|g| g.value()))?
            };
            let (new_owner, new_journal) = {
                let a = table.get(STORAGE_SCHEMA).map_err(crate::invalid)?;
                let b = table.get(JOURNAL_PROJECTION).map_err(crate::invalid)?;
                match (a, b) {
                    (Some(a), Some(b)) => {
                        let journal: Journal =
                            codec::decode(STORAGE_SCHEMA, a.value(), b.value()).map_err(crate::invalid)?;
                        if let Some(owner) = &owner {
                            owner.validate(state_ref, a.value(), b.value())?;
                            journal.validate(&state, owner.projected_state_ref)?;
                            (None, None)
                        } else {
                            // Forward migration from the original monolithic v1
                            // pair is allowed only when it binds this exact state.
                            journal.validate(&state, state_ref)?;
                            (
                                Some(JournalOwner {
                                    state_ref,
                                    projected_state_ref: state_ref,
                                    journal_ref: codec::digest(a.value()),
                                    reconstruction_digest: codec::digest(b.value()),
                                }),
                                None,
                            )
                        }
                    }
                    (None, None) if owner.is_none() => {
                        let journal = Journal::bootstrap(&state, state_ref)?;
                        let encoded = codec::encode(STORAGE_SCHEMA, &journal).map_err(crate::invalid)?;
                        (Some(JournalOwner::new(state_ref, &encoded)), Some(encoded))
                    }
                    _ => return Err(Error::Invalid("incomplete durable progress journal/owner".into())),
                }
            };
            if let Some(encoded) = new_journal {
                table.insert(STORAGE_SCHEMA, encoded.canonical.as_slice()).map_err(crate::invalid)?;
                table.insert(JOURNAL_PROJECTION, encoded.projection.as_slice()).map_err(crate::invalid)?;
            }
            if let Some(owner) = new_owner {
                write_owner(&mut table, &owner)?;
            }
        }
        tx.commit().map_err(|_| Error::Unknown)
    }

    // r[impl molten.observations.cursors]
    pub(super) fn progress_view(&self) -> Result<(State, Journal, Id), Error> {
        let tx = self.db.begin_read().map_err(crate::invalid)?;
        let table = tx.open_table(SNAPSHOTS).map_err(crate::invalid)?;
        let a = table.get("controller-state.v2").map_err(crate::invalid)?.ok_or(Error::Unavailable)?;
        let b = table.get("controller-state.v2.projection").map_err(crate::invalid)?.ok_or(Error::Unavailable)?;
        let state = codec::decode("controller-state.v2", a.value(), b.value()).map_err(crate::invalid)?;
        verify_state(&state)?;
        let state_ref = codec::digest(a.value());
        let a = table.get(OWNER_SCHEMA).map_err(crate::invalid)?;
        let b = table.get(OWNER_PROJECTION).map_err(crate::invalid)?;
        let owner =
            decode_owner(a.as_ref().map(|g| g.value()), b.as_ref().map(|g| g.value()))?.ok_or(Error::Unavailable)?;
        let a = table.get(STORAGE_SCHEMA).map_err(crate::invalid)?.ok_or(Error::Unavailable)?;
        let b = table.get(JOURNAL_PROJECTION).map_err(crate::invalid)?.ok_or(Error::Unavailable)?;
        owner.validate(state_ref, a.value(), b.value())?;
        let journal: Journal = codec::decode(STORAGE_SCHEMA, a.value(), b.value()).map_err(crate::invalid)?;
        journal.validate(&state, owner.projected_state_ref)?;
        Ok((state, journal, state_ref))
    }
}

#[cfg(test)]
mod tests {
    use molten_batch_application::Commit;
    use molten_batch_application::StateStore;

    use super::*;
    use crate::CapabilityRoot;
    use crate::RedbControllerStore;

    fn initial() -> State {
        State::empty(Id([1; 32]), Id([2; 32]), 1, 3).unwrap()
    }
    fn journal_pair(disk: &Disk) -> (Vec<u8>, Vec<u8>) {
        let tx = disk.db.begin_read().unwrap();
        let table = tx.open_table(SNAPSHOTS).unwrap();
        let canonical = table.get(STORAGE_SCHEMA).unwrap().unwrap().value().to_vec();
        let projection = table.get(JOURNAL_PROJECTION).unwrap().unwrap().value().to_vec();
        (canonical, projection)
    }
    fn remove_owner(disk: &Disk, canonical: bool) {
        let mut tx = disk.db.begin_write().unwrap();
        tx.set_durability(Durability::Immediate).unwrap();
        {
            let mut table = tx.open_table(SNAPSHOTS).unwrap();
            if canonical {
                table.remove(OWNER_SCHEMA).unwrap();
            }
            table.remove(OWNER_PROJECTION).unwrap();
        }
        tx.commit().unwrap();
    }

    // r[verify molten.observations.stream]
    // r[verify molten.observations.cursors]
    #[test]
    fn state_only_commit_keeps_journal_bytes_and_reopens_current_owner() {
        let temp = tempfile::tempdir().unwrap();
        let root = CapabilityRoot::open(temp.path()).unwrap();
        let disk = Disk::open(&root).unwrap();
        let store = RedbControllerStore::open(disk.clone(), initial()).unwrap();
        let before = journal_pair(&disk);
        let old = store.load().unwrap();
        let next = apply(&old, &Command::Tick).unwrap();
        assert_eq!(store.compare_commit(&old, &next).unwrap(), Commit::Applied);
        assert_eq!(journal_pair(&disk), before);
        let owner: JournalOwner = disk.load(OWNER_SCHEMA).unwrap();
        let next_ref = codec::identity("controller-state.v2", &next).unwrap();
        assert_eq!(owner.state_ref, next_ref);
        assert_eq!(owner.projected_state_ref, codec::identity("controller-state.v2", &old).unwrap());
        owner.validate(next_ref, &before.0, &before.1).unwrap();
        let (observed, _, observed_ref) = disk.progress_view().unwrap();
        assert_eq!(observed, next);
        assert_eq!(observed_ref, next_ref);
        drop((store, disk));
        let disk = Disk::open(&root).unwrap();
        let store = RedbControllerStore::open(disk.clone(), initial()).unwrap();
        assert_eq!(store.load().unwrap(), next);
        assert_eq!(journal_pair(&disk), before);
        assert_eq!(disk.progress_view().unwrap().2, next_ref);
    }

    // r[verify molten.observations.stream]
    #[test]
    fn state_only_commit_rejects_changed_journal_and_preserves_state() {
        let temp = tempfile::tempdir().unwrap();
        let disk = Disk::open(&CapabilityRoot::open(temp.path()).unwrap()).unwrap();
        let store = RedbControllerStore::open(disk.clone(), initial()).unwrap();
        let old = store.load().unwrap();
        let next = apply(&old, &Command::Tick).unwrap();
        let mut pair = journal_pair(&disk);
        pair.1.push(b' ');
        let mut tx = disk.db.begin_write().unwrap();
        tx.set_durability(Durability::Immediate).unwrap();
        tx.open_table(SNAPSHOTS).unwrap().insert(JOURNAL_PROJECTION, pair.1.as_slice()).unwrap();
        tx.commit().unwrap();
        assert!(matches!(store.compare_commit(&old, &next), Err(Error::Invalid(_))));
        assert_eq!(store.load().unwrap(), old);
        assert!(matches!(disk.progress_view(), Err(Error::Invalid(_))));
    }

    // r[verify molten.observations.stream]
    #[test]
    fn matching_legacy_pair_migrates_without_republishing_and_partial_owner_denies() {
        let temp = tempfile::tempdir().unwrap();
        let disk = Disk::open(&CapabilityRoot::open(temp.path()).unwrap()).unwrap();
        let store = RedbControllerStore::open(disk.clone(), initial()).unwrap();
        let before = journal_pair(&disk);
        remove_owner(&disk, true);
        disk.initialize_progress().unwrap();
        assert_eq!(journal_pair(&disk), before);
        let (state, _, state_ref) = disk.progress_view().unwrap();
        assert_eq!(state, initial());
        assert_eq!(state_ref, codec::identity("controller-state.v2", &initial()).unwrap());
        remove_owner(&disk, false);
        assert!(matches!(disk.initialize_progress(), Err(Error::Invalid(_))));
        let next = apply(&state, &Command::Tick).unwrap();
        assert!(matches!(store.compare_commit(&state, &next), Err(Error::Invalid(_))));
        assert_eq!(store.load().unwrap(), state);
    }

    // r[verify molten.observations.stream]
    #[test]
    fn crossed_owner_is_not_repaired_into_an_accepted_cut() {
        let temp = tempfile::tempdir().unwrap();
        let disk = Disk::open(&CapabilityRoot::open(temp.path()).unwrap()).unwrap();
        let store = RedbControllerStore::open(disk.clone(), initial()).unwrap();
        let old = store.load().unwrap();
        let mut owner: JournalOwner = disk.load(OWNER_SCHEMA).unwrap();
        owner.state_ref = Id([9; 32]);
        let mut tx = disk.db.begin_write().unwrap();
        tx.set_durability(Durability::Immediate).unwrap();
        {
            let mut table = tx.open_table(SNAPSHOTS).unwrap();
            write_owner(&mut table, &owner).unwrap();
        }
        tx.commit().unwrap();
        let next = apply(&old, &Command::Tick).unwrap();
        assert!(matches!(store.compare_commit(&old, &next), Err(Error::Invalid(_))));
        assert!(matches!(disk.progress_view(), Err(Error::Invalid(_))));
        assert!(matches!(disk.initialize_progress(), Err(Error::Invalid(_))));
        assert_eq!(store.load().unwrap(), old);
    }
}
