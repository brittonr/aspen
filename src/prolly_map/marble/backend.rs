// The store state the bounded executor's worker owns.
//
// The heap is a marble object store; the index is an adaptive radix trie over
// fixed 32-byte BLAKE3 digests. Every block is framed with its digest, so a
// reopen rebuilds the index from the batches marble atomically recovered.
// A failed write batch poisons the instance: marble may have installed part
// of the batch in its runtime location table, and only a reopen's recovery
// shows what is durable.

use molten_core::marble_store::*;
use molten_core::prolly_map::*;

use super::super::store::decode_head;
use super::super::store::encode_head;
use super::super::store::expected_matches;
use super::super::store::validate_map_id;
use super::super::*;

const TARGET_FILE_BYTES: usize = 64 * 1_024 * 1_024;
const FILE_COMPACTION_PERCENT: u8 = 66;
const SMALL_FILE_CLEANUP_THRESHOLD: usize = 64;
const MIN_COMPACTION_FILES: usize = 2;

pub(super) struct MarbleBackend {
    pub(super) heap: ::marble::Marble,
    pub(super) profile: MarbleStoreProfile,
    pub(super) index: ::art::Art<u64, 32>,
    pub(super) allocator: ObjectIdAllocator,
    pub(super) catalog: MarbleCatalog,
    pub(super) cache: ReadCache,
    pub(super) in_flight: Option<InFlightBatch>,
    pub(super) is_poisoned: bool,
    pub(super) batches_since_maintenance: u32,
    /// The failure that poisoned the instance during scheduled maintenance.
    pub(super) maintenance_failure: Option<ProllyPortError>,
}

impl MarbleBackend {
    /// Open the heap and rebuild the index from its recovered objects.
    // r[impl aspen.marble_store.identity]
    // r[impl aspen.marble_store.art_index]
    pub(super) fn open(directory: std::path::PathBuf, profile: MarbleStoreProfile) -> ProllyPortResult<Self> {
        if let Some(issue) = validate_store_profile(&profile).into_iter().next() {
            return Err(issue_error("prolly-marble-profile", &issue));
        }
        let max_object_size = usize::try_from(profile.max_object_bytes)
            .map_err(|_| issue_error("prolly-marble-profile", &MarbleStoreIssue::ObjectBoundOutOfRange))?;
        let heap = ::marble::Config {
            path: directory,
            zstd_compression_level: None,
            fsync_each_batch: true,
            target_file_size: TARGET_FILE_BYTES,
            file_compaction_percent: FILE_COMPACTION_PERCENT,
            max_object_size,
            small_file_cleanup_threshold: SMALL_FILE_CLEANUP_THRESHOLD,
            partition_function: ::marble::default_partition_function,
            min_compaction_files: MIN_COMPACTION_FILES,
        }
        .open()
        .map_err(|error| heap_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?;
        let (high_water, _) = heap.free_object_ids();
        let mut recovery = IndexRecovery::new(profile.layout, profile.max_object_bytes);
        for object_id in heap.allocated_object_ids() {
            let object = heap
                .read(object_id)
                .map_err(|error| heap_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?;
            if let Some(object) = object {
                recovery.observe(object_id, &object);
            }
        }
        let recovered = recovery
            .finish()
            .map_err(|issues| ProllyPortError::new("prolly-marble-recovery", format!("{issues:?}"), false))?;
        let mut index = ::art::Art::new();
        for (digest, object_id) in recovered.entries {
            index.insert(digest, object_id);
        }
        let allocator = ObjectIdAllocator::seeded(profile.layout, high_water)
            .map_err(|issue| issue_error("prolly-marble-recovery", &issue))?;
        Ok(Self {
            heap,
            profile,
            index,
            allocator,
            catalog: recovered.catalog,
            cache: ReadCache::new(profile.read_cache_entries),
            in_flight: None,
            is_poisoned: false,
            batches_since_maintenance: 0,
            maintenance_failure: None,
        })
    }

    pub(super) fn ensure_serving(&self) -> ProllyPortResult<()> {
        if self.is_poisoned {
            let detail = self.maintenance_failure.as_ref().map_or_else(
                || "a write batch failed; reopen the store to recover".to_owned(),
                |failure| format!("scheduled maintenance failed: {}; reopen the store to recover", failure.detail),
            );
            return Err(ProllyPortError::new("prolly-marble-poisoned", detail, false));
        }
        Ok(())
    }

    /// Read one block: the in-flight batch, then the cache, then the heap.
    pub(super) fn read_block(&mut self, node_ref: &NodeRef) -> ProllyPortResult<Option<Vec<u8>>> {
        self.ensure_serving()?;
        let Ok(digest) = digest_of_node_ref(node_ref) else {
            return Ok(None);
        };
        let indexed = self.index.get(&digest).copied();
        match resolve(&digest, self.in_flight.as_ref(), indexed) {
            Resolution::InFlight(payload) => Ok(Some(payload.to_vec())),
            Resolution::Absent => Ok(None),
            Resolution::Indexed(object_id) => {
                if let Some(payload) = self.cache.get(&digest) {
                    return Ok(Some(payload.to_vec()));
                }
                let payload = self.read_committed(&digest, object_id)?;
                self.cache.insert(digest, &payload);
                Ok(Some(payload))
            }
        }
    }

    pub(super) fn read_committed(&self, digest: &Digest, object_id: u64) -> ProllyPortResult<Vec<u8>> {
        let object = self
            .heap
            .read(object_id)
            .map_err(|error| heap_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?
            .ok_or_else(|| ProllyPortError::new("prolly-marble-corrupt", "an indexed object is missing", false))?;
        let frame = decode_block_frame(&object, self.profile.max_object_bytes)
            .map_err(|issue| issue_error("prolly-marble-corrupt", &issue))?;
        if frame.digest != *digest {
            return Err(ProllyPortError::new(
                "prolly-marble-corrupt",
                "an object's digest differs from its index entry",
                false,
            ));
        }
        Ok(frame.payload.to_vec())
    }

    /// Stage blocks as one crash-atomic marble batch.
    // r[impl aspen.marble_store.identity]
    pub(super) fn stage_blocks(&mut self, blocks: &[EncodedBlock]) -> ProllyPortResult<()> {
        self.ensure_serving()?;
        let mut requests = Vec::with_capacity(blocks.len());
        for block in blocks {
            let digest =
                digest_of_node_ref(&block.node_ref).map_err(|issue| issue_error("prolly-marble-node-ref", &issue))?;
            requests.push(StageRequest {
                digest,
                payload: &block.bytes,
            });
        }
        let plan = plan_block_batch(&requests, |digest| self.index.get(digest).copied(), &self.allocator).map_err(
            |issue| {
                let code = match issue {
                    MarbleStoreIssue::DigestCollision => "prolly-block-collision",
                    MarbleStoreIssue::ObjectIdExhausted => "prolly-marble-object-id-exhausted",
                    _ => "prolly-marble-plan",
                };
                issue_error(code, &issue)
            },
        )?;
        for indexed in &plan.indexed {
            if self.read_committed(&indexed.digest, indexed.object_id)? != indexed.payload {
                return Err(ProllyPortError::new(
                    "prolly-block-collision",
                    "existing block bytes do not match identity",
                    false,
                ));
            }
        }
        let Some(reservation) = plan.reservation.clone() else {
            return Ok(());
        };
        let mut frames = Vec::with_capacity(plan.writes.len());
        for write in &plan.writes {
            let frame = encode_block_frame(&write.digest, write.payload, self.profile.max_object_bytes)
                .map_err(|issue| issue_error("prolly-marble-frame", &issue))?;
            frames.push((write.object_id, frame));
        }
        self.in_flight = Some(InFlightBatch::begin(&plan));
        let written = self.write_objects(
            frames.iter().map(|(object_id, frame)| (*object_id, Some(frame.as_slice()))),
            ProllyPortPhase::Staging,
        );
        let batch = self.in_flight.take().unwrap_or_default();
        let observation = match written {
            Ok(()) => WriteBatchObservation::Committed,
            Err(_) => WriteBatchObservation::Failed,
        };
        if let BatchSettlement::Install(mappings) = settle_batch(batch, observation) {
            for (digest, object_id) in mappings {
                self.index.insert(digest, object_id);
            }
            self.allocator.commit(&reservation);
            self.count_committed_batch();
        }
        written
    }

    pub(super) fn read_root(&self, map_id: &str) -> ProllyPortResult<Option<PublishedProllyRoot>> {
        self.ensure_serving()?;
        validate_map_id(map_id)?;
        self.catalog.heads.get(map_id).map(|head| decode_head(head)).transpose()
    }

    pub(super) fn compare_and_advance(
        &mut self,
        map_id: String,
        expected: &ExpectedProllyRoot,
        next: &PublishedProllyRoot,
    ) -> ProllyPortResult<ProllyPublicationObservation> {
        let observed = self.read_root(&map_id)?;
        if observed.as_ref() == Some(next) {
            return Ok(ProllyPublicationObservation::AlreadyApplied);
        }
        if !expected_matches(expected, observed.as_ref()) {
            return Ok(ProllyPublicationObservation::Stale);
        }
        let mut catalog = self.catalog.clone();
        catalog.heads.insert(map_id, encode_head(next)?);
        catalog.retention_generation = catalog.retention_generation.saturating_add(1);
        self.write_catalog(catalog, ProllyPortPhase::Publication)?;
        Ok(ProllyPublicationObservation::Applied)
    }

    /// Rewrite the catalog object as one batch, then adopt it.
    pub(super) fn write_catalog(&mut self, next: MarbleCatalog, phase: ProllyPortPhase) -> ProllyPortResult<()> {
        let bytes = encode_catalog(&next).map_err(|issue| issue_error("prolly-marble-catalog", &issue))?;
        let catalog_id = self.profile.layout.catalog_id();
        self.write_objects([(catalog_id, Some(bytes.as_slice()))], phase)?;
        self.catalog = next;
        self.count_committed_batch();
        Ok(())
    }

    /// Write one batch; a failure poisons the instance.
    pub(super) fn write_objects<'a>(
        &mut self,
        objects: impl IntoIterator<Item = (u64, Option<&'a [u8]>)>,
        phase: ProllyPortPhase,
    ) -> ProllyPortResult<()> {
        let written = self.heap.write_batch(objects);
        written.map_err(|error| {
            self.is_poisoned = true;
            heap_error(phase, ProllyPortFailureKind::Commit, error)
        })
    }

    /// Count a committed batch and run maintenance when the schedule says so.
    /// A maintenance failure poisons the instance; the committed batch stands.
    pub(super) fn count_committed_batch(&mut self) {
        self.batches_since_maintenance = self.batches_since_maintenance.saturating_add(1);
        if self.profile.maintenance.is_due(self.batches_since_maintenance)
            && let Err(failure) = self.maintain()
        {
            self.maintenance_failure = Some(failure);
        }
    }
}

pub(super) fn heap_error(
    phase: ProllyPortPhase,
    kind: ProllyPortFailureKind,
    error: impl std::fmt::Display,
) -> ProllyPortError {
    let outcome = classify_port_failure(phase, kind);
    ProllyPortError::new("prolly-marble", error.to_string(), outcome.is_unknown())
}

pub(super) fn issue_error(code: &'static str, issue: &MarbleStoreIssue) -> ProllyPortError {
    ProllyPortError::new(code, format!("{issue:?}"), false)
}
