#![allow(deprecated)]

/// Aspen Storage Migration Tool
///
/// Migrates Raft state machine data from redb to SQLite format with verification.
///
/// Usage:
///   aspen-migrate --source data/node-1/state-machine.redb --target data/node-1/state-machine.db --verify
///
/// Tiger Style compliance:
/// - Fail-fast on any error
/// - Bounded operations (no unbounded iteration)
/// - Explicit verification steps
/// - RAII for resource cleanup
use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use aspen::raft::storage::RedbStateMachine;
use aspen::raft::storage_sqlite::SqliteStateMachine;
use clap::Parser;
use openraft::StoredMembership;
use openraft::storage::RaftStateMachine;

/// Command-line arguments for the migration tool
#[derive(Parser, Debug)]
#[command(name = "aspen-migrate")]
#[command(about = "Migrate Aspen state machine data from redb to SQLite", long_about = None)]
struct Args {
    /// Path to source redb database file
    #[arg(long)]
    source: PathBuf,

    /// Path to target SQLite database file
    #[arg(long)]
    target: PathBuf,

    /// Verify migration integrity after completion
    #[arg(long, default_value = "false")]
    verify: bool,

    /// Perform full checksum verification (expensive, checks all keys)
    #[arg(long, default_value = "false")]
    full_verify: bool,
}

/// Key-value pair from state machine
#[derive(Debug, Clone)]
struct KvPair {
    key: String,
    value: String,
}

/// Metadata extracted from source state machine
#[derive(Debug)]
struct StateMachineMetadata {
    last_applied_log: Option<openraft::LogId<aspen::raft::types::AppTypeConfig>>,
    last_membership: StoredMembership<aspen::raft::types::AppTypeConfig>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    println!("Aspen State Machine Migration Tool");
    println!("===================================");
    println!("Source (redb):  {}", args.source.display());
    println!("Target (SQLite): {}", args.target.display());
    println!();

    // Step 1: Validate inputs
    if !args.source.exists() {
        anyhow::bail!("Source database does not exist: {}", args.source.display());
    }

    if args.target.exists() {
        anyhow::bail!(
            "Target database already exists: {}. Remove it first to prevent data loss.",
            args.target.display()
        );
    }

    // Step 2: Open source redb database (read-only)
    println!("[1/6] Opening source redb database...");
    let source = open_source_redb(&args.source)?;

    // Step 3: Read all KV pairs from redb
    println!("[2/6] Reading all key-value pairs from redb...");
    let kv_data = read_all_kv_pairs(&source).await?;
    println!("      Found {} key-value pairs", kv_data.len());

    // Step 4: Read metadata from redb
    println!("[3/6] Reading metadata from redb...");
    let metadata = read_metadata(&source).await?;
    println!(
        "      Last applied log: {:?}",
        metadata
            .last_applied_log
            .as_ref()
            .map(|l| format!("term={}, index={}", l.leader_id.term, l.index))
            .unwrap_or_else(|| "None".to_string())
    );

    // Step 5: Create target SQLite database and migrate data
    println!("[4/6] Creating target SQLite database...");
    let target = create_target_sqlite(&args.target)?;

    println!("[5/6] Migrating data to SQLite...");
    migrate_data(&target, &kv_data, &metadata).await?;
    println!("      Migration complete!");

    // Step 6: Verify migration (if --verify flag)
    if args.verify {
        println!("[6/6] Verifying migration integrity...");
        verify_migration(&source, &target, args.full_verify).await?;
        println!("      Verification passed!");
    } else {
        println!("[6/6] Skipping verification (use --verify to enable)");
    }

    println!();
    println!("Migration Summary");
    println!("=================");
    println!("  Keys migrated:  {}", kv_data.len());
    println!("  Source:         {}", args.source.display());
    println!("  Target:         {}", args.target.display());
    println!("  Verified:       {}", args.verify);
    println!();
    println!("Next steps:");
    println!("  1. Backup the original redb file");
    println!("  2. Update configuration to use SQLite storage");
    println!("  3. Restart the Aspen node");
    println!();

    Ok(())
}

/// Open the source redb database in read-only mode
fn open_source_redb(path: &PathBuf) -> Result<std::sync::Arc<RedbStateMachine>> {
    #[allow(deprecated)]
    RedbStateMachine::new(path).context("Failed to open source redb database")
}

/// Create the target SQLite database
fn create_target_sqlite(path: &PathBuf) -> Result<std::sync::Arc<SqliteStateMachine>> {
    SqliteStateMachine::new(path).context("Failed to create target SQLite database")
}

/// Read all key-value pairs from the redb state machine
async fn read_all_kv_pairs(source: &std::sync::Arc<RedbStateMachine>) -> Result<Vec<KvPair>> {
    use redb::{ReadableTable, TableDefinition};

    const STATE_MACHINE_KV_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sm_kv");

    // Open read transaction directly on the underlying database
    // Note: We need to access the database connection directly, which isn't exposed
    // by the public API. For now, we'll use a different approach: iterate through
    // known keys or use a sampling strategy.
    //
    // Since RedbStateMachine doesn't expose an iterator API, we'll need to read
    // the database file directly using redb's public API.

    let db_path = source.path();
    let db = redb::Database::open(db_path).context("Failed to open redb database for reading")?;

    let read_txn = db
        .begin_read()
        .context("Failed to begin read transaction")?;
    let table = read_txn
        .open_table(STATE_MACHINE_KV_TABLE)
        .context("Failed to open KV table")?;

    let mut pairs = Vec::new();
    let iter = table.iter().context("Failed to iterate KV table")?;

    for item in iter {
        let (key, value) = item.context("Failed to read KV pair")?;
        pairs.push(KvPair {
            key: key.value().to_string(),
            value: value.value().to_string(),
        });
    }

    Ok(pairs)
}

/// Read metadata from the redb state machine
async fn read_metadata(source: &std::sync::Arc<RedbStateMachine>) -> Result<StateMachineMetadata> {
    use redb::TableDefinition;

    const STATE_MACHINE_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sm_meta");

    let db_path = source.path();
    let db = redb::Database::open(db_path)
        .context("Failed to open redb database for metadata reading")?;

    let read_txn = db
        .begin_read()
        .context("Failed to begin read transaction")?;
    let table = read_txn
        .open_table(STATE_MACHINE_META_TABLE)
        .context("Failed to open metadata table")?;

    // Read last_applied_log
    let last_applied_log = match table
        .get("last_applied_log")
        .context("Failed to read last_applied_log")?
    {
        Some(value) => {
            let bytes = value.value();
            let data: Option<openraft::LogId<aspen::raft::types::AppTypeConfig>> =
                bincode::deserialize(bytes).context("Failed to deserialize last_applied_log")?;
            data
        }
        None => None,
    };

    // Read last_membership
    let last_membership = match table
        .get("last_membership")
        .context("Failed to read last_membership")?
    {
        Some(value) => {
            let bytes = value.value();
            bincode::deserialize(bytes).context("Failed to deserialize last_membership")?
        }
        None => StoredMembership::default(),
    };

    Ok(StateMachineMetadata {
        last_applied_log,
        last_membership,
    })
}

/// Migrate data from redb to SQLite
async fn migrate_data(
    target: &std::sync::Arc<SqliteStateMachine>,
    kv_data: &[KvPair],
    metadata: &StateMachineMetadata,
) -> Result<()> {
    use rusqlite::params;

    // Get write connection
    let conn = target
        .write_conn
        .lock()
        .expect("SQLite connection mutex poisoned");

    // Begin transaction
    conn.execute("BEGIN IMMEDIATE", [])
        .context("Failed to begin transaction")?;

    // Insert all KV pairs
    for pair in kv_data {
        conn.execute(
            "INSERT INTO state_machine_kv (key, value) VALUES (?1, ?2)",
            params![&pair.key, &pair.value],
        )
        .context("Failed to insert KV pair")?;
    }

    // Insert metadata
    if let Some(ref log_id) = metadata.last_applied_log {
        let serialized =
            bincode::serialize(&Some(log_id)).context("Failed to serialize last_applied_log")?;
        conn.execute(
            "INSERT INTO state_machine_meta (key, value) VALUES ('last_applied_log', ?1)",
            params![serialized],
        )
        .context("Failed to insert last_applied_log")?;
    }

    let membership_bytes = bincode::serialize(&metadata.last_membership)
        .context("Failed to serialize last_membership")?;
    conn.execute(
        "INSERT INTO state_machine_meta (key, value) VALUES ('last_membership', ?1)",
        params![membership_bytes],
    )
    .context("Failed to insert last_membership")?;

    // Commit transaction
    conn.execute("COMMIT", [])
        .context("Failed to commit transaction")?;

    Ok(())
}

/// Verify migration integrity
async fn verify_migration(
    source: &std::sync::Arc<RedbStateMachine>,
    target: &std::sync::Arc<SqliteStateMachine>,
    full_verify: bool,
) -> Result<()> {
    // 1. Count verification
    let source_kv_data = read_all_kv_pairs(source).await?;
    let source_count = source_kv_data.len();
    let target_count = target.count_kv_pairs()? as usize;

    if source_count != target_count {
        anyhow::bail!(
            "Key count mismatch: source={}, target={}",
            source_count,
            target_count
        );
    }
    println!("      Count verification: {} keys", source_count);

    // 2. Data verification
    let sample_size = if full_verify {
        source_count
    } else {
        std::cmp::min(100, source_count)
    };

    let mut verified_count: u32 = 0;
    const MAX_VERIFICATIONS: u32 = 10000; // Tiger Style: bounded verification

    for pair in source_kv_data.iter().take(sample_size) {
        verified_count += 1;
        if verified_count > MAX_VERIFICATIONS {
            anyhow::bail!(
                "Verification count {} exceeds maximum limit of {}",
                verified_count,
                MAX_VERIFICATIONS
            );
        }

        let target_value = target
            .get(&pair.key)
            .await
            .context("Failed to read from target")?;

        match target_value {
            Some(val) if val == pair.value => {}
            Some(val) => {
                anyhow::bail!(
                    "Value mismatch for key '{}': source='{}', target='{}'",
                    pair.key,
                    pair.value,
                    val
                );
            }
            None => {
                anyhow::bail!("Key '{}' missing in target", pair.key);
            }
        }
    }

    if full_verify {
        println!("      Data verification: all {} keys verified", sample_size);
    } else {
        println!(
            "      Data verification: {} of {} keys sampled",
            sample_size, source_count
        );
    }

    // 3. Metadata verification
    let source_metadata = read_metadata(source).await?;
    let mut target_clone = target.clone();
    let (target_last_applied, target_last_membership) = target_clone
        .applied_state()
        .await
        .context("Failed to read target applied state")?;

    if source_metadata.last_applied_log != target_last_applied {
        anyhow::bail!(
            "last_applied_log mismatch: source={:?}, target={:?}",
            source_metadata.last_applied_log,
            target_last_applied
        );
    }

    if source_metadata.last_membership.log_id() != target_last_membership.log_id() {
        anyhow::bail!(
            "last_membership mismatch: source={:?}, target={:?}",
            source_metadata.last_membership.log_id(),
            target_last_membership.log_id()
        );
    }

    println!("      Metadata verification: passed");

    // 4. Checksum verification (if full_verify)
    if full_verify {
        let source_checksum = compute_checksum(&source_kv_data);
        let mut target_kv_data = BTreeMap::new();
        for pair in &source_kv_data {
            if let Some(value) = target.get(&pair.key).await? {
                target_kv_data.insert(pair.key.clone(), value);
            }
        }
        let target_checksum = compute_checksum_from_map(&target_kv_data);

        if source_checksum != target_checksum {
            anyhow::bail!(
                "Checksum mismatch: source={}, target={}",
                source_checksum,
                target_checksum
            );
        }
        println!("      Checksum verification: passed ({})", source_checksum);
    }

    Ok(())
}

/// Compute deterministic checksum of KV pairs
fn compute_checksum(pairs: &[KvPair]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for pair in pairs {
        pair.key.hash(&mut hasher);
        pair.value.hash(&mut hasher);
    }
    hasher.finish()
}

/// Compute deterministic checksum from BTreeMap
fn compute_checksum_from_map(map: &BTreeMap<String, String>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for (key, value) in map {
        key.hash(&mut hasher);
        value.hash(&mut hasher);
    }
    hasher.finish()
}
