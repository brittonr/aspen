//! Bounded pilot backend for the marble object-store spike.
//!
//! The backend is optional and configuration-selected: it compiles only under
//! the `marble-store` feature and mounts behind the local object storage seam
//! as an alternative physical layer for whole-object payloads. It never
//! becomes the default storage path and grants no runtime authority.
//!
//! BLAKE3 stays the only content identity. Marble ObjectIds are private
//! physical handles allocated in sharded ranges per object kind.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvError;
use std::sync::mpsc::Sender;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::TrySendError;
use std::sync::mpsc::channel;
use std::sync::mpsc::sync_channel;
use std::thread::JoinHandle;

use blake3::Hash as Blake3Hash;

use crate::error::MoltenError;
use crate::error::Result;

// r[impl aspen.marble_store.spike]
// r[impl aspen.marble_store.identity]
const DIGEST_BYTES: usize = 32;
const KIND_SHARD_CONTENT: u8 = 0;
const KIND_SHARD_INDEX: u8 = 1;
const MIN_MAINTENANCE_INTERVAL: u32 = 1;
const DEFAULT_MAINTENANCE_INTERVAL_BATCHES: u32 = 16;
const MAX_OBJECTS_PER_BATCH: usize = 4_096;
const MAX_BATCH_BYTES: u64 = 256 * 1024 * 1024;
const MAX_OBJECT_BYTES: usize = 64 * 1024 * 1024;
const MAX_EXECUTOR_QUEUE_DEPTH: usize = 1_024;
const MAX_READ_CACHE_ENTRIES: usize = 4_096;
const MAX_READ_CACHE_BYTES: usize = 64 * 1024 * 1024;
const MAX_RECOVERY_MAPPINGS: usize = 1_000_000;
const CONTENT_ID_BASE: u64 = 1 << 32;
const CONTENT_ID_CEILING: u64 = (1 << 48) - 1;
const INDEX_ID_BASE: u64 = 1 << 48;
const INDEX_ID_CEILING: u64 = (1 << 62) - 1;
const MARBLE_TARGET_FILE_SIZE: usize = 4 * 1024 * 1024;
const MARBLE_FILE_COMPACTION_PERCENT: u8 = 66;

const _: () = assert!(MAX_OBJECTS_PER_BATCH <= 16_384);
const _: () = assert!(MAX_BATCH_BYTES <= u32::MAX as u64);
const _: () = assert!(MAX_READ_CACHE_ENTRIES <= 65_536);
const _: () = assert!(MAX_RECOVERY_MAPPINGS <= 1_000_000);
const _: () = assert!(CONTENT_ID_BASE < CONTENT_ID_CEILING);
const _: () = assert!(INDEX_ID_BASE < INDEX_ID_CEILING);
const _: () = assert!(CONTENT_ID_CEILING < INDEX_ID_BASE);

/// Digest-to-physical-handle key over the fixed BLAKE3 byte length.
pub type DigestKey = [u8; DIGEST_BYTES];
/// Object kinds with sharded physical identifier ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    /// Caller-supplied payload object.
    Content,
    /// Backend-internal index snapshot object.
    Index,
}

impl ObjectKind {
    /// Stable kind label used by canonical receipts.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::Index => "index",
        }
    }
}

/// Configuration-selected bounds for one pilot backend instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreConfig {
    /// Directory that will hold the marble heap.
    pub heap_path: std::path::PathBuf,
    /// Compression level passed to marble; `None` disables compression.
    pub zstd_compression_level: Option<i32>,
    /// Whether marble fsyncs each committed batch.
    pub fsync_each_batch: bool,
    /// Largest accepted single object in bytes.
    pub max_object_bytes: usize,
    /// Largest accepted batch object count.
    pub max_batch_objects: usize,
    /// Executor queue depth for blocking operations.
    pub executor_queue_depth: usize,
    /// Read cache entry capacity in front of marble.
    pub read_cache_entries: usize,
    /// Read cache byte capacity in front of marble.
    pub read_cache_bytes: usize,
    /// Committed batches between backend-owned maintenance runs.
    pub maintenance_interval_batches: u32,
    /// Ceiling for content-kind physical identifiers.
    pub content_id_ceiling: u64,
    /// Largest single object marble itself accepts.
    pub marble_max_object_bytes: usize,
}

impl StoreConfig {
    // r[impl aspen.marble_store.spike]
    pub fn pilot(heap_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            heap_path: heap_path.into(),
            zstd_compression_level: None,
            fsync_each_batch: true,
            max_object_bytes: MAX_OBJECT_BYTES,
            max_batch_objects: MAX_OBJECTS_PER_BATCH,
            executor_queue_depth: MAX_EXECUTOR_QUEUE_DEPTH,
            read_cache_entries: MAX_READ_CACHE_ENTRIES,
            read_cache_bytes: MAX_READ_CACHE_BYTES,
            maintenance_interval_batches: DEFAULT_MAINTENANCE_INTERVAL_BATCHES,
            content_id_ceiling: CONTENT_ID_CEILING,
            marble_max_object_bytes: MAX_OBJECT_BYTES,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        let mut issues = Vec::new();
        if self.max_object_bytes == 0 || self.max_object_bytes > MAX_OBJECT_BYTES {
            issues.push("max_object_bytes out of bounds");
        }
        if self.max_batch_objects == 0 || self.max_batch_objects > MAX_OBJECTS_PER_BATCH {
            issues.push("max_batch_objects out of bounds");
        }
        if self.executor_queue_depth == 0 || self.executor_queue_depth > MAX_EXECUTOR_QUEUE_DEPTH {
            issues.push("executor_queue_depth out of bounds");
        }
        if self.read_cache_entries == 0 || self.read_cache_entries > MAX_READ_CACHE_ENTRIES {
            issues.push("read_cache_entries out of bounds");
        }
        if self.read_cache_bytes == 0 || self.read_cache_bytes > MAX_READ_CACHE_BYTES {
            issues.push("read_cache_bytes out of bounds");
        }
        if self.maintenance_interval_batches < MIN_MAINTENANCE_INTERVAL {
            issues.push("maintenance_interval_batches below minimum");
        }
        if self.content_id_ceiling <= CONTENT_ID_BASE || self.content_id_ceiling > CONTENT_ID_CEILING {
            issues.push("content_id_ceiling out of bounds");
        }
        if self.marble_max_object_bytes == 0 {
            issues.push("marble_max_object_bytes must be positive");
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(MoltenError::invalid_harness(format!("marble store config denied: {issues:?}")))
        }
    }

    fn marble(&self) -> marble::Config {
        marble::Config {
            path: self.heap_path.clone(),
            zstd_compression_level: self.zstd_compression_level,
            fsync_each_batch: self.fsync_each_batch,
            target_file_size: MARBLE_TARGET_FILE_SIZE,
            file_compaction_percent: MARBLE_FILE_COMPACTION_PERCENT,
            max_object_size: self.marble_max_object_bytes,
            partition_function: shard_by_kind,
            ..marble::Config::default()
        }
    }
}

/// Marble partition function that colocates physical identifiers by object
/// kind range so maintenance groups objects with similar lifespans.
fn shard_by_kind(object_id: u64, _object_size: usize) -> u8 {
    if object_id >= INDEX_ID_BASE {
        KIND_SHARD_INDEX
    } else {
        KIND_SHARD_CONTENT
    }
}

/// One stored object as admitted into a batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredObject {
    /// Canonical BLAKE3 content reference.
    pub content_ref: String,
    /// Private physical handle; never content identity.
    pub object_id: u64,
    /// Object kind that owns the physical handle range.
    pub object_kind: ObjectKind,
    /// Stored byte length.
    pub byte_length: u64,
}

/// Result of one committed store batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchOutcome {
    /// Objects written and indexed by this batch, sorted by content ref.
    pub stored: Vec<StoredObject>,
    /// Digests already present and therefore reused without a new write.
    pub reused_refs: Vec<String>,
    /// Batch byte total including reused lookups.
    pub batch_bytes: u64,
    /// Maintenance run by the backend schedule with this batch, if any.
    pub maintenance: Option<MaintenanceOutcome>,
}

/// Result of one fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchOutcome {
    /// Canonical BLAKE3 content reference that was resolved.
    pub content_ref: String,
    /// Fetched bytes re-verified against the digest.
    pub bytes: Arc<Vec<u8>>,
    /// Private physical handle the digest resolved to.
    pub object_id: u64,
    /// Whether the bytes came from the bounded read cache.
    pub from_read_cache: bool,
}

/// Result of one backend-owned maintenance run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaintenanceOutcome {
    /// Files marble rewrote during this run.
    pub rewritten_files: usize,
}

/// Result of opening and recovering a backend over an existing heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryOutcome {
    /// Digest mappings replayed from atomically recovered batches.
    pub recovered_mappings: usize,
    /// Highest recovered content physical handle.
    pub max_content_object_id: u64,
    /// Heap file total in bytes after recovery.
    pub heap_bytes: u64,
}

/// Heap statistics snapshot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatsOutcome {
    /// Objects marble currently reports as live.
    pub live_objects: u64,
    /// Objects marble currently reports as stored.
    pub stored_objects: u64,
    /// Total heap file bytes.
    pub total_file_size: u64,
    /// Marble space amplification estimate.
    pub space_amplification: f32,
    /// Marble write amplification estimate.
    pub write_amplification: f32,
}

/// Executor command for blocking store operations.
enum Command {
    Store {
        batch: Vec<(ObjectKind, Vec<u8>)>,
        reply: Sender<Result<BatchOutcome>>,
    },
    Fetch {
        digest: DigestKey,
        reply: Sender<Result<FetchOutcome>>,
    },
    Maintain {
        reply: Sender<Result<MaintenanceOutcome>>,
    },
    Stats {
        reply: Sender<StatsOutcome>,
    },
    Shutdown,
}

/// Shared handle state for the bounded executor.
struct SharedExecutor {
    sender: SyncSender<Command>,
    worker: std::sync::Mutex<Option<JoinHandle<()>>>,
}

impl Drop for SharedExecutor {
    fn drop(&mut self) {
        let _ignored = self.sender.try_send(Command::Shutdown);
        if let Some(worker) = self.worker.lock().expect("marble store worker lock").take() {
            let _ignored = worker.join();
        }
    }
}

/// Configuration-selected marble-plus-art pilot backend handle.
///
/// Cloning shares one bounded executor. All blocking marble operations,
/// including recovery, run on the dedicated executor thread; callers only
/// queue bounded commands.
#[derive(Clone)]
pub struct Backend {
    executor: Arc<SharedExecutor>,
    recovery: RecoveryOutcome,
}

impl Backend {
    /// Opens the backend, recovers only atomically recovered batches into the
    /// digest index on the bounded executor, and replays no partial batch.
    // r[impl aspen.marble_store.spike]
    // r[impl aspen.marble_store.identity]
    pub fn open(config: &StoreConfig) -> Result<Self> {
        config.validate()?;
        std::fs::create_dir_all(&config.heap_path)?;
        let (sender, receiver) = sync_channel(config.executor_queue_depth);
        let (recovery_reply, recovery_result) = channel();
        let handle = spawn_executor(config.clone(), receiver, recovery_reply, &config.heap_path)?;
        let recovery = recovery_result.recv().map_err(|RecvError { .. }| {
            MoltenError::invalid_harness("marble store executor failed before recovery completed")
        })??;
        Ok(Self {
            executor: Arc::new(SharedExecutor {
                sender,
                worker: std::sync::Mutex::new(Some(handle)),
            }),
            recovery,
        })
    }

    /// Recovery summary observed while opening this backend.
    pub fn recovery(&self) -> RecoveryOutcome {
        self.recovery
    }

    /// Stores one batch atomically and returns the committed outcome.
    // r[impl aspen.marble_store.identity]
    pub fn store_batch(&self, batch: Vec<(ObjectKind, Vec<u8>)>) -> Result<BatchOutcome> {
        let (reply, receiver) = channel();
        self.send(Command::Store { batch, reply })?;
        self.await_reply(receiver, "store batch")?
    }

    /// Resolves one canonical content reference and fetches verified bytes.
    // r[impl aspen.marble_store.identity]
    // r[impl aspen.marble_store.art_index]
    pub fn fetch(&self, content_ref: &str) -> Result<FetchOutcome> {
        let digest = digest_from_content_ref(content_ref)?;
        let (reply, receiver) = channel();
        self.send(Command::Fetch { digest, reply })?;
        self.await_reply(receiver, "fetch")?
    }

    /// Runs one explicit maintenance pass owned by the backend.
    // r[impl aspen.marble_store.spike]
    pub fn maintain(&self) -> Result<MaintenanceOutcome> {
        let (reply, receiver) = channel();
        self.send(Command::Maintain { reply })?;
        self.await_reply(receiver, "maintenance")?
    }

    /// Snapshots current heap statistics.
    pub fn stats(&self) -> Result<StatsOutcome> {
        let (reply, receiver) = channel();
        self.send(Command::Stats { reply })?;
        self.await_reply(receiver, "stats")
    }

    fn send(&self, command: Command) -> Result<()> {
        match self.executor.sender.try_send(command) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                Err(MoltenError::invalid_harness("marble store executor queue is saturated; retry after draining"))
            }
            Err(TrySendError::Disconnected(_)) => {
                Err(MoltenError::invalid_harness("marble store executor has stopped"))
            }
        }
    }

    fn await_reply<T>(&self, receiver: Receiver<T>, operation: &str) -> Result<T> {
        receiver.recv().map_err(|RecvError { .. }| {
            MoltenError::invalid_harness(format!("marble store executor dropped the {operation} reply"))
        })
    }
}

fn spawn_executor(
    config: StoreConfig,
    receiver: Receiver<Command>,
    recovery_reply: Sender<Result<RecoveryOutcome>>,
    heap_path: &Path,
) -> Result<JoinHandle<()>> {
    let heap_label = heap_path.display().to_string();
    std::thread::Builder::new()
        .name("molten-marble-store-executor".to_string())
        .spawn(move || match Worker::recover(&config) {
            Ok(worker) => {
                let _ignored = recovery_reply.send(Ok(worker.recovery_summary()));
                run_worker(worker, receiver);
            }
            Err(error) => {
                let _ignored = recovery_reply.send(Err(error));
            }
        })
        .map_err(|error| {
            MoltenError::invalid_harness(format!("marble store executor failed to start at {heap_label}: {error}"))
        })
}

/// Deterministic digest index plus caches owned by the executor thread.
struct Worker {
    config: StoreConfig,
    heap: marble::Marble,
    index: art::Art<u64, DIGEST_BYTES>,
    index_len: usize,
    max_content_id: u64,
    write_cache: HashMap<DigestKey, WriteEntry>,
    read_cache: ReadCache,
    next_content_id: u64,
    next_index_id: u64,
    batches_since_maintenance: u32,
}

/// One in-flight batch mapping served from the backend cache.
struct WriteEntry {
    object_id: u64,
    bytes: Arc<Vec<u8>>,
}

/// Bounded least-recently-used read cache in front of marble.
struct ReadCache {
    entries: HashMap<DigestKey, Arc<Vec<u8>>>,
    order: VecDeque<DigestKey>,
    bytes: usize,
    max_entries: usize,
    max_bytes: usize,
}

impl ReadCache {
    fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            max_entries,
            max_bytes,
        }
    }

    fn get(&mut self, digest: &DigestKey) -> Option<Arc<Vec<u8>>> {
        let bytes = self.entries.get(digest).cloned()?;
        let position = self.order.iter().position(|key| key == digest)?;
        self.order.remove(position);
        self.order.push_back(*digest);
        Some(bytes)
    }

    fn insert(&mut self, digest: DigestKey, bytes: Arc<Vec<u8>>) {
        let byte_length = bytes.len();
        if byte_length > self.max_bytes {
            return;
        }
        if self.entries.insert(digest, bytes).is_none() {
            self.order.push_back(digest);
            self.bytes += byte_length;
        }
        while self.entries.len() > self.max_entries || self.bytes > self.max_bytes {
            let Some(evicted) = self.order.pop_front() else {
                break;
            };
            if let Some(bytes) = self.entries.remove(&evicted) {
                self.bytes = self.bytes.saturating_sub(bytes.len());
            }
        }
    }
}

fn run_worker(mut worker: Worker, receiver: Receiver<Command>) {
    while let Ok(command) = receiver.recv() {
        match command {
            Command::Store { batch, reply } => {
                let _ignored = reply.send(worker.store(batch));
            }
            Command::Fetch { digest, reply } => {
                let _ignored = reply.send(worker.fetch(digest));
            }
            Command::Maintain { reply } => {
                let _ignored = reply.send(worker.maintain());
            }
            Command::Stats { reply } => {
                let _ignored = reply.send(worker.stats());
            }
            Command::Shutdown => break,
        }
    }
}

/// One admitted object with its computed digest.
struct AdmittedObject {
    kind: ObjectKind,
    digest: DigestKey,
    bytes: Vec<u8>,
}

/// Bounds-checked view of one requested batch.
struct AdmittedBatch {
    objects: Vec<AdmittedObject>,
    batch_bytes: u64,
}

impl Worker {
    /// Recovers the digest index from marble's atomically recovered heap.
    // r[impl aspen.marble_store.identity]
    fn recover(config: &StoreConfig) -> Result<Self> {
        let heap = config.marble().open()?;
        let mut index = art::Art::default();
        let mut index_len = 0_usize;
        let mut max_content_id = CONTENT_ID_BASE - 1;
        for object_id in heap.allocated_object_ids() {
            if object_id < CONTENT_ID_BASE || object_id >= config.content_id_ceiling {
                continue;
            }
            let Some(bytes) = heap.read(object_id)? else {
                return Err(MoltenError::invalid_harness(format!(
                    "recovered object {object_id} is absent from the marble heap"
                )));
            };
            let digest: DigestKey = blake3::hash(&bytes).into();
            index.insert(digest, object_id);
            index_len += 1;
            max_content_id = max_content_id.max(object_id);
            if index_len > MAX_RECOVERY_MAPPINGS {
                return Err(MoltenError::invalid_harness("marble store recovery exceeded its mapping bound"));
            }
        }
        Ok(Self {
            next_content_id: max_content_id + 1,
            next_index_id: INDEX_ID_BASE,
            config: config.clone(),
            heap,
            index,
            index_len,
            max_content_id,
            write_cache: HashMap::new(),
            read_cache: ReadCache::new(config.read_cache_entries, config.read_cache_bytes),
            batches_since_maintenance: 0,
        })
    }

    fn recovery_summary(&self) -> RecoveryOutcome {
        let stats = self.heap.stats();
        RecoveryOutcome {
            recovered_mappings: self.index_len,
            max_content_object_id: self.max_content_id,
            heap_bytes: stats.total_file_size,
        }
    }

    // r[impl aspen.marble_store.identity]
    fn store(&mut self, batch: Vec<(ObjectKind, Vec<u8>)>) -> Result<BatchOutcome> {
        let admission = self.admit_batch(batch)?;
        let mut drafts = Vec::with_capacity(admission.objects.len());
        let mut write_pairs = Vec::with_capacity(admission.objects.len());
        let mut reused_digests = Vec::new();
        let mut handled = std::collections::HashSet::with_capacity(admission.objects.len());
        for object in &admission.objects {
            let is_first_occurrence = handled.insert(object.digest);
            let is_present = self.index.get(&object.digest).is_some() || self.write_cache.contains_key(&object.digest);
            if !is_first_occurrence || is_present {
                reused_digests.push(object.digest);
                continue;
            }
            let object_id = self.allocate(object.kind)?;
            self.write_cache.insert(object.digest, WriteEntry {
                object_id,
                bytes: Arc::new(object.bytes.clone()),
            });
            write_pairs.push((object_id, Some(object.bytes.clone())));
            drafts.push(StoredDraft {
                digest: object.digest,
                object_id,
                kind: object.kind,
                byte_length: object.bytes.len(),
            });
        }
        if write_pairs.is_empty() {
            return Ok(self.assembled_outcome(&admission, Vec::new(), reused_digests, None));
        }
        match self.heap.write_batch(write_pairs) {
            Ok(()) => self.commit_batch(&admission, drafts, reused_digests),
            Err(error) => {
                self.write_cache.clear();
                Err(MoltenError::Io(format!(
                    "marble write batch was interrupted before commit; no mappings were published: {error}"
                )))
            }
        }
    }

    // r[impl aspen.marble_store.identity]
    // r[impl aspen.marble_store.art_index]
    fn fetch(&mut self, digest: DigestKey) -> Result<FetchOutcome> {
        let content_ref = content_ref_of(&digest);
        if let Some(entry) = self.write_cache.get(&digest) {
            return Ok(FetchOutcome {
                content_ref,
                bytes: entry.bytes.clone(),
                object_id: entry.object_id,
                from_read_cache: false,
            });
        }
        let Some(&object_id) = self.index.get(&digest) else {
            return Err(MoltenError::invalid_harness(format!("digest lookup failed; {content_ref} was never stored")));
        };
        if let Some(bytes) = self.read_cache.get(&digest) {
            return Ok(FetchOutcome {
                content_ref,
                bytes,
                object_id,
                from_read_cache: true,
            });
        }
        let Some(bytes) = self.heap.read(object_id)? else {
            return Err(MoltenError::invalid_harness(format!(
                "physical object {object_id} for {content_ref} is absent from the marble heap"
            )));
        };
        let observed: DigestKey = blake3::hash(&bytes).into();
        if observed != digest {
            return Err(MoltenError::invalid_harness(format!(
                "fetched bytes for {content_ref} do not match the resolved digest"
            )));
        }
        let bytes = Arc::new(Vec::from(bytes));
        self.read_cache.insert(digest, bytes.clone());
        Ok(FetchOutcome {
            content_ref,
            bytes,
            object_id,
            from_read_cache: false,
        })
    }

    // r[impl aspen.marble_store.spike]
    fn maintain(&mut self) -> Result<MaintenanceOutcome> {
        let rewritten_files = self.heap.maintenance()?;
        Ok(MaintenanceOutcome { rewritten_files })
    }

    fn stats(&self) -> StatsOutcome {
        let stats = self.heap.stats();
        StatsOutcome {
            live_objects: stats.live_objects,
            stored_objects: stats.stored_objects,
            total_file_size: stats.total_file_size,
            space_amplification: stats.space_amplification,
            write_amplification: stats.write_amplification,
        }
    }

    fn admit_batch(&self, batch: Vec<(ObjectKind, Vec<u8>)>) -> Result<AdmittedBatch> {
        if batch.len() > self.config.max_batch_objects {
            return Err(MoltenError::invalid_harness(format!(
                "marble store batch denied: {} objects exceed the bound of {}",
                batch.len(),
                self.config.max_batch_objects
            )));
        }
        let mut objects = Vec::with_capacity(batch.len());
        let mut batch_bytes = 0_u64;
        for (kind, bytes) in batch {
            if bytes.len() > self.config.max_object_bytes {
                return Err(MoltenError::invalid_harness(format!(
                    "marble store batch denied: object of {} bytes exceeds the bound of {}",
                    bytes.len(),
                    self.config.max_object_bytes
                )));
            }
            batch_bytes = batch_bytes.saturating_add(bytes.len() as u64);
            if batch_bytes > MAX_BATCH_BYTES {
                return Err(MoltenError::invalid_harness(format!(
                    "marble store batch denied: {batch_bytes} batch bytes exceed the bound of {MAX_BATCH_BYTES}"
                )));
            }
            objects.push(AdmittedObject {
                kind,
                digest: blake3::hash(&bytes).into(),
                bytes,
            });
        }
        Ok(AdmittedBatch { objects, batch_bytes })
    }

    fn allocate(&mut self, kind: ObjectKind) -> Result<u64> {
        let (next, ceiling, label) = match kind {
            ObjectKind::Content => (&mut self.next_content_id, self.config.content_id_ceiling, "content"),
            ObjectKind::Index => (&mut self.next_index_id, INDEX_ID_CEILING, "index"),
        };
        if *next >= ceiling {
            return Err(MoltenError::invalid_harness(format!(
                "marble store {label} object id range is exhausted; allocation fails closed"
            )));
        }
        let allocated = *next;
        *next += 1;
        Ok(allocated)
    }

    fn commit_batch(
        &mut self,
        admission: &AdmittedBatch,
        drafts: Vec<StoredDraft>,
        reused_digests: Vec<DigestKey>,
    ) -> Result<BatchOutcome> {
        for draft in &drafts {
            if let Some(entry) = self.write_cache.remove(&draft.digest) {
                self.read_cache.insert(draft.digest, entry.bytes);
            }
            if self.index.insert(draft.digest, draft.object_id).is_none() {
                self.index_len += 1;
            }
        }
        self.batches_since_maintenance += 1;
        let maintenance = if self.batches_since_maintenance >= self.config.maintenance_interval_batches {
            self.batches_since_maintenance = 0;
            Some(self.maintain()?)
        } else {
            None
        };
        Ok(self.assembled_outcome(admission, drafts, reused_digests, maintenance))
    }

    fn assembled_outcome(
        &self,
        admission: &AdmittedBatch,
        drafts: Vec<StoredDraft>,
        reused_digests: Vec<DigestKey>,
        maintenance: Option<MaintenanceOutcome>,
    ) -> BatchOutcome {
        let mut stored = Vec::with_capacity(drafts.len());
        for draft in drafts {
            stored.push(StoredObject {
                content_ref: content_ref_of(&draft.digest),
                object_id: draft.object_id,
                object_kind: draft.kind,
                byte_length: draft.byte_length as u64,
            });
        }
        stored.sort_by(|left, right| left.content_ref.cmp(&right.content_ref));
        let mut reused_refs = Vec::with_capacity(reused_digests.len());
        for digest in reused_digests {
            reused_refs.push(content_ref_of(&digest));
        }
        reused_refs.sort_unstable();
        reused_refs.dedup();
        BatchOutcome {
            stored,
            reused_refs,
            batch_bytes: admission.batch_bytes,
            maintenance,
        }
    }
}

struct StoredDraft {
    digest: DigestKey,
    object_id: u64,
    kind: ObjectKind,
    byte_length: usize,
}

/// Computes the fixed BLAKE3 digest key for object bytes.
pub fn digest_of(bytes: &[u8]) -> DigestKey {
    blake3::hash(bytes).into()
}

/// Renders the canonical content reference for a digest key.
pub fn content_ref_of(digest: &DigestKey) -> String {
    crate::preserves_rail::content_ref_from_blake3_hash(Blake3Hash::from(*digest))
}

/// Parses a canonical BLAKE3 content reference into a fixed digest key.
// r[impl aspen.marble_store.art_index]
pub fn digest_from_content_ref(content_ref: &str) -> Result<DigestKey> {
    let hex = crate::preserves_rail::content_ref_hex(content_ref)?;
    let digest = Blake3Hash::from_hex(hex).map_err(|error| {
        MoltenError::invalid_harness(format!("content ref {content_ref} does not decode as BLAKE3: {error}"))
    })?;
    Ok(digest.into())
}
