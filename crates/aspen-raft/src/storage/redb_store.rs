//! Redb-backed persistent Raft log store.
//!
//! Provides ACID-compliant persistent storage for Raft log entries with chain hashing
//! for integrity verification. Each log entry has an associated chain hash computed as:
//!
//! ```text
//! hash = blake3(prev_hash || log_index || term || entry_data)
//! ```
//!
//! This creates an unbreakable chain where modifying any entry invalidates all
//! subsequent hashes, enabling detection of hardware corruption and tampering.
//!
//! # Tiger Style Compliance
//!
//! - Explicitly sized types (u64 for log indices)
//! - Fixed database size limit (configurable at creation)
//! - Fail-fast on corruption (redb panics on invalid state)
//! - Bounded operations (no unbounded iteration)
//! - Chain hashing for integrity verification (32-byte Blake3)

use std::fmt::Debug;
use std::io;
use std::ops::RangeBounds;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use aspen_core::hlc;
use aspen_hlc::SerializableTimestamp;
use openraft::LogState;
use openraft::OptionalSend;
use openraft::RaftLogReader;
use openraft::alias::LogIdOf;
use openraft::alias::VoteOf;
use openraft::entry::RaftEntry;
use openraft::storage::IOFlushed;
use openraft::storage::RaftLogStorage;
use redb::Database;
use redb::ReadableTable;
use serde::Deserialize;
use serde::Serialize;
use snafu::ResultExt;

use super::BeginReadSnafu;
use super::BeginWriteSnafu;
use super::CHAIN_HASH_TABLE;
use super::CommitSnafu;
use super::CreateDirectorySnafu;
use super::DeserializeSnafu;
use super::GetSnafu;
use super::INTEGRITY_META_TABLE;
use super::InsertSnafu;
use super::OpenDatabaseSnafu;
use super::OpenTableSnafu;
use super::RAFT_LOG_TABLE;
use super::RAFT_META_TABLE;
use super::RangeSnafu;
use super::RemoveSnafu;
use super::SNAPSHOT_TABLE;
use super::SerializeSnafu;
use super::StorageError;
use crate::constants::INTEGRITY_VERSION;
use crate::constants::MAX_BATCH_SIZE;
use crate::integrity::ChainHash;
use crate::integrity::ChainTipState;
use crate::integrity::GENESIS_HASH;
use crate::integrity::compute_entry_hash;
use crate::integrity::hash_to_hex;
use crate::integrity::verify_entry_hash;
use crate::types::AppTypeConfig;

/// Persistent Raft log backed by redb with chain hashing.
///
/// Stores log entries, vote state, committed index, and last purged log id on disk.
/// Provides ACID guarantees for all operations via redb transactions.
///
/// Each log entry has an associated chain hash computed as:
/// ```text
/// hash = blake3(prev_hash || log_index || term || entry_data)
/// ```
///
/// This creates an unbreakable chain where modifying any entry invalidates all
/// subsequent hashes, enabling detection of hardware corruption and tampering.
///
/// Tiger Style compliance:
/// - Explicitly sized types (u64 for log indices)
/// - Fixed database size limit (configurable at creation)
/// - Fail-fast on corruption (redb panics on invalid state)
/// - Bounded operations (no unbounded iteration)
/// - Chain hashing for integrity verification (32-byte Blake3)
///
/// # Cache Consistency
///
/// The `chain_tip` cache is an optimization to avoid database reads on every append.
/// It is safe because:
///
/// 1. **Raft serializes appends**: OpenRaft guarantees that log appends are serialized at the
///    consensus layer. Only the leader appends, and it does so sequentially. This eliminates
///    concurrent append races.
///
/// 2. **Database is source of truth**: On startup/recovery, `load_chain_tip()` reads from the
///    database, not the cache. A crash after database commit but before cache update is safe - the
///    next startup loads the correct state.
///
/// 3. **Truncate is also serialized**: Log truncation only happens during leader changes, which are
///    also serialized by Raft consensus.
#[derive(Clone, Debug)]
pub struct RedbLogStore {
    db: Arc<Database>,
    path: PathBuf,
    /// Cached chain tip state for efficient appends.
    /// Updated on each append, loaded on startup.
    ///
    /// Uses std::sync::RwLock because operations are fast and we need
    /// to access it from both sync (migration) and async (append) contexts.
    ///
    /// This cache is safe without versioning because Raft serializes all
    /// log operations. See struct-level documentation for details.
    chain_tip: Arc<StdRwLock<ChainTipState>>,
}

impl RedbLogStore {
    /// Create or open a redb-backed log store at the given path.
    ///
    /// Creates the database file and all required tables if they don't exist.
    /// Also initializes chain hashing tables and migrates existing databases.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context(CreateDirectorySnafu { path: parent })?;
        }

        // Open existing database without truncating, create if missing.
        let db = if path.exists() {
            Database::open(&path).context(OpenDatabaseSnafu { path: &path })?
        } else {
            Database::create(&path).context(OpenDatabaseSnafu { path: &path })?
        };

        // Initialize tables (including chain hash tables)
        let write_txn = db.begin_write().context(BeginWriteSnafu)?;
        {
            write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(SNAPSHOT_TABLE).context(OpenTableSnafu)?;
            // Chain hashing tables
            write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        let db = Arc::new(db);

        // Load chain tip from database (or use defaults for empty/migrating database)
        let chain_tip = Self::load_chain_tip(&db)?;

        let store = Self {
            db,
            path,
            chain_tip: Arc::new(StdRwLock::new(chain_tip)),
        };

        // Migrate if needed (one-time operation for existing databases)
        store.migrate_if_needed()?;

        Ok(store)
    }

    /// Load chain tip state from database.
    ///
    /// Returns the cached chain tip if available, otherwise computes it
    /// by scanning the chain hash table (for migration scenarios).
    fn load_chain_tip(db: &Arc<Database>) -> Result<ChainTipState, StorageError> {
        let read_txn = db.begin_read().context(BeginReadSnafu)?;

        // Try to read cached chain tip
        let meta_table = read_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;

        let tip_hash = meta_table.get("chain_tip_hash").context(GetSnafu)?.and_then(|v| {
            let bytes = v.value();
            if bytes.len() == 32 {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(bytes);
                Some(hash)
            } else {
                None
            }
        });

        let tip_index: Option<u64> = meta_table
            .get("chain_tip_index")
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize(v.value()).ok());

        match (tip_hash, tip_index) {
            (Some(hash), Some(index)) => Ok(ChainTipState { hash, index }),
            _ => {
                // Chain tip not cached, check if we have any chain hashes
                let hash_table = read_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

                // Get the last entry's hash
                if let Some(last) = hash_table.iter().context(RangeSnafu)?.last() {
                    let (key, value) = last.context(GetSnafu)?;
                    let index = key.value();
                    let bytes = value.value();
                    if bytes.len() == 32 {
                        let mut hash = [0u8; 32];
                        hash.copy_from_slice(bytes);
                        return Ok(ChainTipState { hash, index });
                    }
                }

                // Empty database or no chain hashes yet
                Ok(ChainTipState::default())
            }
        }
    }

    /// Migrate existing database to chain hashing if needed.
    ///
    /// This is a one-time operation that computes chain hashes for all
    /// existing log entries.
    fn migrate_if_needed(&self) -> Result<(), StorageError> {
        let current_version = self.read_integrity_version()?;
        if current_version >= INTEGRITY_VERSION {
            return Ok(());
        }

        tracing::info!(current_version, target_version = INTEGRITY_VERSION, "migrating log storage to chain hashing");

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        let mut prev_hash = GENESIS_HASH;
        let mut entry_count: u64 = 0;
        let mut last_index: u64 = 0;

        {
            let log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

            // Iterate all existing entries and compute chain hashes
            for item in log_table.iter().context(RangeSnafu)? {
                let (key, value) = item.context(GetSnafu)?;
                let index = key.value();
                let entry_bytes = value.value();

                // Deserialize to get term
                let entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry =
                    bincode::deserialize(entry_bytes).context(DeserializeSnafu)?;
                let log_id = entry.log_id();
                let term = log_id.leader_id.term;

                // Compute chain hash
                let entry_hash = compute_entry_hash(&prev_hash, index, term, entry_bytes);

                hash_table.insert(index, entry_hash.as_slice()).context(InsertSnafu)?;

                prev_hash = entry_hash;
                last_index = index;
                entry_count += 1;

                if entry_count.is_multiple_of(10000) {
                    tracing::info!(entry_count, "migration progress");
                }
            }

            // Store migration version
            let mut meta_table = write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;
            let version_bytes = bincode::serialize(&INTEGRITY_VERSION).context(SerializeSnafu)?;
            meta_table.insert("integrity_version", version_bytes.as_slice()).context(InsertSnafu)?;

            // Store chain tip
            meta_table.insert("chain_tip_hash", prev_hash.as_slice()).context(InsertSnafu)?;
            let index_bytes = bincode::serialize(&last_index).context(SerializeSnafu)?;
            meta_table.insert("chain_tip_index", index_bytes.as_slice()).context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        // Update in-memory chain tip
        {
            let mut chain_tip = self.chain_tip.write().map_err(|_| StorageError::LockPoisoned {
                context: "writing chain_tip during migration".into(),
            })?;
            chain_tip.hash = prev_hash;
            chain_tip.index = last_index;
        }

        tracing::info!(
            entry_count,
            chain_tip_hash = %hash_to_hex(&prev_hash),
            "migration complete"
        );

        Ok(())
    }

    /// Read the current integrity schema version.
    fn read_integrity_version(&self) -> Result<u32, StorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = match read_txn.open_table(INTEGRITY_META_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(0), // Table doesn't exist = version 0
        };

        match table.get("integrity_version").context(GetSnafu)? {
            Some(value) => {
                let version: u32 = bincode::deserialize(value.value()).context(DeserializeSnafu)?;
                Ok(version)
            }
            None => Ok(0),
        }
    }

    /// Get the path to the log store database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Validates storage integrity (used by supervisor before restart).
    ///
    /// This method is called by the supervision system before allowing
    /// a RaftNode to restart after a crash. It ensures the storage is not
    /// corrupted and safe to use.
    ///
    /// # Returns
    /// - `Ok(ValidationReport)` if all validation checks pass
    /// - `Err(StorageValidationError)` if any corruption is detected
    ///
    /// # Note
    /// This validation is read-only and does not modify the database.
    /// The database is opened, validated, and closed within this function.
    pub fn validate(
        &self,
        node_id: u64,
    ) -> Result<crate::storage_validation::ValidationReport, crate::storage_validation::StorageValidationError> {
        crate::storage_validation::validate_raft_storage(node_id, &self.path)
    }

    // Internal helper: Read a value from the metadata table
    fn read_meta<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, StorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;

        match table.get(key).context(GetSnafu)? {
            Some(value) => {
                let bytes = value.value();
                let data: T = bincode::deserialize(bytes).context(DeserializeSnafu)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    // Internal helper: Write a value to the metadata table
    fn write_meta<T: Serialize>(&self, key: &str, value: &T) -> Result<(), StorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            let serialized = bincode::serialize(value).context(SerializeSnafu)?;
            table.insert(key, serialized.as_slice()).context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    // Internal helper: Delete a value from the metadata table
    fn delete_meta(&self, key: &str) -> Result<(), StorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            table.remove(key).context(RemoveSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }
}

// ============================================================================
// RaftLogReader Implementation
// ============================================================================

impl RaftLogReader<AppTypeConfig> for RedbLogStore {
    async fn try_get_log_entries<RB>(
        &mut self,
        range: RB,
    ) -> Result<Vec<<AppTypeConfig as openraft::RaftTypeConfig>::Entry>, io::Error>
    where
        RB: RangeBounds<u64> + Clone + Debug + OptionalSend,
    {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

        let mut entries = Vec::new();
        let iter = table.range(range).context(RangeSnafu)?;

        for item in iter {
            let (_key, value) = item.context(GetSnafu)?;
            let bytes = value.value();
            let entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry =
                bincode::deserialize(bytes).context(DeserializeSnafu)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    async fn read_vote(&mut self) -> Result<Option<VoteOf<AppTypeConfig>>, io::Error> {
        Ok(self.read_meta("vote")?)
    }
}

// ============================================================================
// RaftLogStorage Implementation
// ============================================================================

impl RaftLogStorage<AppTypeConfig> for RedbLogStore {
    type LogReader = Self;

    async fn get_log_state(&mut self) -> Result<LogState<AppTypeConfig>, io::Error> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

        // Get last log entry
        let last_log_id = table
            .iter()
            .context(RangeSnafu)?
            .last()
            .transpose()
            .context(GetSnafu)?
            .map(|(_key, value)| {
                let bytes = value.value();
                let entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry =
                    bincode::deserialize(bytes).context(DeserializeSnafu)?;
                Ok::<_, StorageError>(entry.log_id())
            })
            .transpose()?;

        let last_purged: Option<LogIdOf<AppTypeConfig>> = self.read_meta("last_purged_log_id")?;
        let last = last_log_id.or(last_purged);

        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }

    async fn save_committed(&mut self, committed: Option<LogIdOf<AppTypeConfig>>) -> Result<(), io::Error> {
        if let Some(ref c) = committed {
            self.write_meta("committed", c)?;
        } else {
            self.delete_meta("committed")?;
        }
        Ok(())
    }

    async fn read_committed(&mut self) -> Result<Option<LogIdOf<AppTypeConfig>>, io::Error> {
        Ok(self.read_meta("committed")?)
    }

    async fn save_vote(&mut self, vote: &VoteOf<AppTypeConfig>) -> Result<(), io::Error> {
        // Tiger Style: Check disk space before write to prevent corruption on full disk
        aspen_core::ensure_disk_space_available(&self.path)?;

        self.write_meta("vote", vote)?;
        Ok(())
    }

    async fn append<I>(&mut self, entries: I, callback: IOFlushed<AppTypeConfig>) -> Result<(), io::Error>
    where
        I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
        // Tiger Style: Check disk space before write to prevent corruption on full disk
        aspen_core::ensure_disk_space_available(&self.path)?;

        // Get current chain tip for hash computation
        let mut prev_hash = {
            let chain_tip = self.chain_tip.read().map_err(|_| StorageError::LockPoisoned {
                context: "reading chain_tip for append".into(),
            })?;
            chain_tip.hash
        };

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        let mut new_tip_hash = prev_hash;
        let mut new_tip_index: u64 = 0;
        let mut has_entries = false;
        {
            let mut log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

            // Performance optimization: Pre-serialize all entries before inserting
            // This reduces lock contention and allows redb to optimize bulk inserts
            // Tiger Style: Pre-allocate with MAX_BATCH_SIZE to avoid repeated reallocations
            let mut serialized_entries: Vec<(u64, u64, Vec<u8>, ChainHash)> =
                Vec::with_capacity(MAX_BATCH_SIZE as usize);

            for entry in entries {
                let log_id = entry.log_id();
                let index = log_id.index();
                let term = log_id.leader_id.term;
                let data = bincode::serialize(&entry).context(SerializeSnafu)?;

                // Compute chain hash
                let entry_hash = compute_entry_hash(&prev_hash, index, term, &data);

                serialized_entries.push((index, term, data, entry_hash));

                // Update prev_hash for next entry in batch
                prev_hash = entry_hash;
                has_entries = true;
            }

            // Bulk insert all serialized entries and their hashes
            for (index, _term, data, entry_hash) in &serialized_entries {
                log_table.insert(*index, data.as_slice()).context(InsertSnafu)?;
                hash_table.insert(*index, entry_hash.as_slice()).context(InsertSnafu)?;
            }

            // Update chain tip tracking
            if let Some((index, _, _, hash)) = serialized_entries.last() {
                new_tip_hash = *hash;
                new_tip_index = *index;

                // Persist chain tip to integrity metadata table for recovery across restarts
                let mut meta_table = write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;
                meta_table.insert("chain_tip_hash", new_tip_hash.as_slice()).context(InsertSnafu)?;
                let index_bytes = bincode::serialize(&new_tip_index).context(SerializeSnafu)?;
                meta_table.insert("chain_tip_index", index_bytes.as_slice()).context(InsertSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        // Update cached chain tip after successful commit
        if has_entries {
            let mut chain_tip = self.chain_tip.write().map_err(|_| StorageError::LockPoisoned {
                context: "writing chain_tip after append".into(),
            })?;
            chain_tip.hash = new_tip_hash;
            chain_tip.index = new_tip_index;
        }

        callback.io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let truncate_from = log_id.index();

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

            // Collect keys to remove (>= log_id.index())
            let keys: Vec<u64> = log_table
                .range(truncate_from..)
                .context(RangeSnafu)?
                .map(|item| {
                    let (key, _) = item.context(GetSnafu)?;
                    Ok::<_, StorageError>(key.value())
                })
                .collect::<Result<Vec<_>, _>>()?;

            for key in &keys {
                log_table.remove(*key).context(RemoveSnafu)?;
                hash_table.remove(*key).context(RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        // Repair chain tip: read hash from entry at (truncate_from - 1)
        let new_tip = if truncate_from > 0 {
            self.read_chain_hash_at(truncate_from - 1)?
                .map(|hash| ChainTipState {
                    hash,
                    index: truncate_from - 1,
                })
                .unwrap_or_default()
        } else {
            ChainTipState::default()
        };

        // Persist chain tip to integrity metadata table for recovery across restarts
        {
            let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
            {
                let mut meta_table = write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;
                meta_table.insert("chain_tip_hash", new_tip.hash.as_slice()).context(InsertSnafu)?;
                let index_bytes = bincode::serialize(&new_tip.index).context(SerializeSnafu)?;
                meta_table.insert("chain_tip_index", index_bytes.as_slice()).context(InsertSnafu)?;
            }
            write_txn.commit().context(CommitSnafu)?;
        }

        {
            let mut chain_tip = self.chain_tip.write().map_err(|_| StorageError::LockPoisoned {
                context: "writing chain_tip after truncate".into(),
            })?;
            *chain_tip = new_tip;
        }

        Ok(())
    }

    async fn purge(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        // Verify purge is monotonic (Tiger Style: fail fast on programmer error)
        if let Some(prev) = self.read_meta::<LogIdOf<AppTypeConfig>>("last_purged_log_id")?
            && prev > log_id
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("purge must be monotonic: prev={:?}, new={:?}", prev, log_id),
            ));
        }

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

            // Collect keys to remove (<= log_id.index())
            let keys: Vec<u64> = log_table
                .range(..=log_id.index())
                .context(RangeSnafu)?
                .map(|item| {
                    let (key, _) = item.context(GetSnafu)?;
                    Ok::<_, StorageError>(key.value())
                })
                .collect::<Result<Vec<_>, _>>()?;

            for key in &keys {
                log_table.remove(*key).context(RemoveSnafu)?;
                hash_table.remove(*key).context(RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        self.write_meta("last_purged_log_id", &log_id)?;
        Ok(())
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }
}

// ============================================================================
// Chain Integrity Methods
// ============================================================================

impl RedbLogStore {
    /// Reads the committed log index from metadata.
    ///
    /// Used for cross-storage validation to ensure state machine consistency.
    /// Returns the committed log index if it exists, None otherwise.
    pub async fn read_committed_sync(&self) -> Result<Option<u64>, io::Error> {
        let committed: Option<LogIdOf<AppTypeConfig>> = self.read_meta("committed").map_err(io::Error::other)?;
        Ok(committed.map(|log_id| log_id.index))
    }

    /// Read chain hash at a specific log index.
    ///
    /// Returns `None` if no hash exists at that index.
    fn read_chain_hash_at(&self, index: u64) -> Result<Option<ChainHash>, StorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

        match table.get(index).context(GetSnafu)? {
            Some(value) => {
                let bytes = value.value();
                if bytes.len() == 32 {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(bytes);
                    Ok(Some(hash))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Get the current chain tip for cross-replica verification.
    ///
    /// Returns (tip_index, tip_hash) which can be compared across replicas.
    /// If chain tips match, logs are identical up to that point.
    pub fn chain_tip_for_verification(&self) -> Result<(u64, ChainHash), StorageError> {
        let chain_tip = self.chain_tip.read().map_err(|_| StorageError::LockPoisoned {
            context: "reading chain_tip for verification".into(),
        })?;
        Ok((chain_tip.index, chain_tip.hash))
    }

    /// Get chain tip hash as hex string for logging/display.
    pub fn chain_tip_hash_hex(&self) -> Result<String, StorageError> {
        let chain_tip = self.chain_tip.read().map_err(|_| StorageError::LockPoisoned {
            context: "reading chain_tip for hex display".into(),
        })?;
        Ok(hash_to_hex(&chain_tip.hash))
    }

    /// Verify chain integrity for a batch of log entries.
    ///
    /// Returns Ok(true) if the batch is valid, Ok(false) if verification
    /// cannot proceed (e.g., entries don't exist), or Err with corruption details.
    ///
    /// # Arguments
    ///
    /// * `start_index` - First log index to verify
    /// * `batch_size` - Number of entries to verify (bounded by CHAIN_VERIFY_BATCH_SIZE)
    ///
    /// # Tiger Style
    ///
    /// - Bounded batch size prevents unbounded verification
    /// - Fail-fast on corruption detection
    pub fn verify_chain_batch(&self, start_index: u64, batch_size: u32) -> Result<u64, StorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let log_table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
        let hash_table = read_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

        // Get previous hash (for chain linkage)
        let mut prev_hash = if start_index == 0 || start_index == 1 {
            GENESIS_HASH
        } else {
            let prev_index = start_index - 1;
            match hash_table.get(prev_index).context(GetSnafu)? {
                Some(h) if h.value().len() == 32 => {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(h.value());
                    hash
                }
                _ => return Ok(0), // Previous hash not found, cannot verify
            }
        };

        let end_index = start_index.saturating_add(batch_size as u64);
        let mut verified_count: u64 = 0;

        for index in start_index..end_index {
            // Get entry
            let entry_bytes = match log_table.get(index).context(GetSnafu)? {
                Some(v) => v.value().to_vec(),
                None => break, // No more entries
            };

            // Get stored hash
            let stored_hash = match hash_table.get(index).context(GetSnafu)? {
                Some(h) if h.value().len() == 32 => {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(h.value());
                    hash
                }
                _ => {
                    return Err(StorageError::ChainHashMissing { index });
                }
            };

            // Deserialize to get term
            let entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry =
                bincode::deserialize(&entry_bytes).context(DeserializeSnafu)?;
            let log_id = entry.log_id();
            let term = log_id.leader_id.term;

            // Verify hash
            if !verify_entry_hash(&prev_hash, index, term, &entry_bytes, &stored_hash) {
                let computed = compute_entry_hash(&prev_hash, index, term, &entry_bytes);
                return Err(StorageError::ChainIntegrityViolation {
                    index,
                    expected: hash_to_hex(&stored_hash),
                    found: hash_to_hex(&computed),
                });
            }

            prev_hash = stored_hash;
            verified_count += 1;
        }

        Ok(verified_count)
    }

    /// Get the range of log indices available for verification.
    ///
    /// Returns (first_index, last_index) or None if no entries exist.
    pub fn verification_range(&self) -> Result<Option<(u64, u64)>, StorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let log_table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

        let first = log_table.iter().context(RangeSnafu)?.next();
        let last = log_table.iter().context(RangeSnafu)?.last();

        match (first, last) {
            (Some(first_result), Some(last_result)) => {
                let (first_key, _) = first_result.context(GetSnafu)?;
                let (last_key, _) = last_result.context(GetSnafu)?;
                Ok(Some((first_key.value(), last_key.value())))
            }
            _ => Ok(None),
        }
    }
}

// ============================================================================
// Historical Log Reader Implementation
// ============================================================================

#[async_trait::async_trait]
impl aspen_transport::log_subscriber::HistoricalLogReader for RedbLogStore {
    async fn read_entries(
        &self,
        start_index: u64,
        end_index: u64,
    ) -> Result<Vec<aspen_transport::log_subscriber::LogEntryPayload>, std::io::Error> {
        {
            use aspen_transport::log_subscriber::KvOperation;
            use aspen_transport::log_subscriber::LogEntryPayload;
            use aspen_transport::log_subscriber::MAX_HISTORICAL_BATCH_SIZE;
            use openraft::EntryPayload;
            use snafu::ResultExt;

            // Tiger Style: Bound the batch size
            let actual_end = std::cmp::min(end_index, start_index.saturating_add(MAX_HISTORICAL_BATCH_SIZE as u64));

            let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
            let table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

            let mut entries = Vec::new();
            let iter = table.range(start_index..=actual_end).context(RangeSnafu)?;

            // Create HLC for historical replay timestamps
            // Note: This is synthetic since we don't store the original HLC with entries
            let hlc = hlc::create_hlc("historical-reader");

            for item in iter {
                let (_key, value) = item.context(GetSnafu)?;
                let bytes = value.value();
                let entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry =
                    bincode::deserialize(bytes).context(DeserializeSnafu)?;

                let log_id = entry.log_id();
                let operation = match &entry.payload {
                    EntryPayload::Blank => KvOperation::Noop,
                    EntryPayload::Normal(app_request) => KvOperation::from(app_request.clone()),
                    EntryPayload::Membership(_) => KvOperation::MembershipChange {
                        description: "membership change".to_string(),
                    },
                };

                entries.push(LogEntryPayload {
                    index: log_id.index,
                    term: log_id.leader_id.term,
                    hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
                    operation,
                });
            }

            Ok(entries)
        }
    }

    async fn earliest_available_index(&self) -> Result<Option<u64>, std::io::Error> {
        use snafu::ResultExt;

        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

        let first = table.iter().context(RangeSnafu)?.next();
        match first {
            Some(result) => {
                let (key, _) = result.context(GetSnafu)?;
                Ok(Some(key.value()))
            }
            None => Ok(None),
        }
    }
}
