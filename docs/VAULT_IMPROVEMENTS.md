# Vault System Improvement Plan

## Overview

Enhance the existing hybrid storage architecture (redb for Raft log, SQLite for state) with targeted optimizations for vault operations while maintaining architectural simplicity.

## Phase 1: Database Schema Improvements

### 1.1 Add Indexes for Prefix Queries

```sql
-- Add index for efficient vault prefix scanning
CREATE INDEX IF NOT EXISTS idx_kv_prefix ON state_machine_kv(key);

-- Add covering index for common vault operations
CREATE INDEX IF NOT EXISTS idx_kv_prefix_value ON state_machine_kv(key, value);
```

### 1.2 Add Vault Metadata Table

```sql
-- Track vault-level metadata
CREATE TABLE IF NOT EXISTS vault_metadata (
    vault_name TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    key_count INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    description TEXT,
    owner TEXT,
    tags TEXT -- JSON array of tags
);

-- Trigger to auto-update vault stats on KV changes
CREATE TRIGGER IF NOT EXISTS update_vault_stats
AFTER INSERT OR UPDATE OR DELETE ON state_machine_kv
FOR EACH ROW
WHEN NEW.key LIKE 'vault:%' OR OLD.key LIKE 'vault:%'
BEGIN
    -- Update logic here
END;
```

### 1.3 Add Vault Access Log

```sql
-- Track vault access patterns for optimization
CREATE TABLE IF NOT EXISTS vault_access_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vault_name TEXT NOT NULL,
    operation TEXT NOT NULL, -- 'read', 'write', 'delete', 'scan'
    key TEXT,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch()),
    duration_us INTEGER -- microseconds
);

-- Index for time-based queries
CREATE INDEX IF NOT EXISTS idx_access_time ON vault_access_log(timestamp);
```

## Phase 2: Bounded Limits (Tiger Style)

### 2.1 Constants

```rust
// src/api/vault.rs additions
pub const MAX_VAULTS: usize = 1000;          // Max number of vaults
pub const MAX_KEYS_PER_VAULT: usize = 10000; // Max keys in a vault
pub const MAX_VALUE_SIZE: usize = 1_048_576;  // 1MB max value
pub const MAX_SCAN_RESULTS: usize = 1000;     // Max results from scan
```

### 2.2 Validation Functions

```rust
impl SqliteStateMachine {
    pub async fn validate_vault_operation(
        &self,
        vault: &str,
        operation: VaultOperation,
    ) -> Result<(), VaultError> {
        // Check vault name validity
        validate_vault_name(vault)?;

        // Check vault limits
        match operation {
            VaultOperation::Create => {
                let count = self.count_vaults()?;
                if count >= MAX_VAULTS {
                    return Err(VaultError::TooManyVaults { max: MAX_VAULTS });
                }
            }
            VaultOperation::AddKey => {
                let key_count = self.count_vault_keys(vault)?;
                if key_count >= MAX_KEYS_PER_VAULT {
                    return Err(VaultError::VaultFull {
                        vault: vault.to_string(),
                        max: MAX_KEYS_PER_VAULT,
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }
}
```

## Phase 3: Optimized Vault Operations

### 3.1 Prepared Statement Cache

```rust
pub struct PreparedStatements {
    scan_prefix: Statement,
    count_prefix: Statement,
    delete_prefix: Statement,
    vault_stats: Statement,
}

impl SqliteStateMachine {
    fn prepare_statements(conn: &Connection) -> Result<PreparedStatements> {
        Ok(PreparedStatements {
            scan_prefix: conn.prepare(
                "SELECT key, value FROM state_machine_kv
                 WHERE key >= ?1 AND key < ?2
                 ORDER BY key LIMIT ?3"
            )?,
            count_prefix: conn.prepare(
                "SELECT COUNT(*) FROM state_machine_kv
                 WHERE key >= ?1 AND key < ?2"
            )?,
            delete_prefix: conn.prepare(
                "DELETE FROM state_machine_kv
                 WHERE key >= ?1 AND key < ?2"
            )?,
            vault_stats: conn.prepare(
                "SELECT COUNT(*), SUM(LENGTH(value))
                 FROM state_machine_kv
                 WHERE key >= ?1 AND key < ?2"
            )?,
        })
    }
}
```

### 3.2 Efficient Range Queries

```rust
pub fn scan_vault_efficient(&self, vault: &str) -> Result<Vec<(String, Vec<u8>)>> {
    // Use range query instead of LIKE
    let start = format!("vault:{}:", vault);
    let end = format!("vault:{}:", vault + "\x00");

    let conn = self.read_pool.get()?;
    let mut stmt = conn.prepare_cached(
        "SELECT key, value FROM state_machine_kv
         WHERE key >= ?1 AND key < ?2
         ORDER BY key LIMIT ?3"
    )?;

    stmt.query_map(params![start, end, MAX_SCAN_RESULTS], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?.collect()
}
```

## Phase 4: Vault-Level Operations

### 4.1 Atomic Vault Operations

```rust
impl SqliteStateMachine {
    /// Delete entire vault atomically
    pub async fn delete_vault(&self, vault: &str) -> Result<u64> {
        let start = format!("vault:{}:", vault);
        let end = format!("vault:{}:", vault + "\x00");

        let conn = self.write_conn.lock().unwrap();
        let tx = conn.transaction()?;

        let count = tx.execute(
            "DELETE FROM state_machine_kv WHERE key >= ?1 AND key < ?2",
            params![start, end],
        )?;

        tx.execute(
            "DELETE FROM vault_metadata WHERE vault_name = ?1",
            params![vault],
        )?;

        tx.commit()?;
        Ok(count as u64)
    }

    /// Copy vault to new name atomically
    pub async fn copy_vault(&self, from: &str, to: &str) -> Result<u64> {
        validate_vault_name(from)?;
        validate_vault_name(to)?;

        let from_prefix = format!("vault:{}:", from);
        let to_prefix = format!("vault:{}:", to);

        let conn = self.write_conn.lock().unwrap();
        let tx = conn.transaction()?;

        // Copy all keys
        let count = tx.execute(
            "INSERT INTO state_machine_kv (key, value)
             SELECT
                 ?1 || substr(key, length(?2) + 1) as new_key,
                 value
             FROM state_machine_kv
             WHERE key >= ?2 AND key < ?3",
            params![to_prefix, from_prefix, from_prefix + "\x00"],
        )?;

        tx.commit()?;
        Ok(count as u64)
    }
}
```

### 4.2 Vault Export/Import

```rust
#[derive(Serialize, Deserialize)]
pub struct VaultExport {
    pub vault_name: String,
    pub exported_at: u64,
    pub metadata: VaultMetadata,
    pub entries: Vec<(String, String)>, // key without prefix, value
}

impl SqliteStateMachine {
    pub async fn export_vault(&self, vault: &str) -> Result<VaultExport> {
        let entries = self.scan_vault_efficient(vault)?;
        let metadata = self.get_vault_metadata(vault)?;

        Ok(VaultExport {
            vault_name: vault.to_string(),
            exported_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            metadata,
            entries: entries
                .into_iter()
                .map(|(k, v)| {
                    let key = k.strip_prefix(&format!("vault:{}:", vault))
                        .unwrap_or(&k)
                        .to_string();
                    (key, String::from_utf8_lossy(&v).to_string())
                })
                .collect(),
        })
    }

    pub async fn import_vault(&self, export: VaultExport) -> Result<()> {
        // Validate limits
        if export.entries.len() > MAX_KEYS_PER_VAULT {
            return Err(VaultError::TooManyKeys);
        }

        let conn = self.write_conn.lock().unwrap();
        let tx = conn.transaction()?;

        // Import all entries
        for (key, value) in export.entries {
            let full_key = make_vault_key(&export.vault_name, &key);
            tx.execute(
                "INSERT OR REPLACE INTO state_machine_kv (key, value) VALUES (?1, ?2)",
                params![full_key, value],
            )?;
        }

        // Import metadata
        tx.execute(
            "INSERT OR REPLACE INTO vault_metadata
             (vault_name, description, owner, tags) VALUES (?1, ?2, ?3, ?4)",
            params![
                export.vault_name,
                export.metadata.description,
                export.metadata.owner,
                serde_json::to_string(&export.metadata.tags)?
            ],
        )?;

        tx.commit()?;
        Ok(())
    }
}
```

## Phase 5: Observability

### 5.1 Metrics Collection

```rust
#[derive(Default)]
pub struct VaultMetrics {
    pub total_vaults: AtomicU64,
    pub total_keys: AtomicU64,
    pub total_bytes: AtomicU64,
    pub read_latency_us: Histogram,
    pub write_latency_us: Histogram,
    pub scan_latency_us: Histogram,
}

impl SqliteStateMachine {
    pub fn record_operation(&self, op: &str, duration: Duration) {
        match op {
            "read" => self.metrics.read_latency_us.record(duration.as_micros() as u64),
            "write" => self.metrics.write_latency_us.record(duration.as_micros() as u64),
            "scan" => self.metrics.scan_latency_us.record(duration.as_micros() as u64),
            _ => {}
        }
    }
}
```

### 5.2 Vault Statistics API

```rust
#[derive(Serialize, Deserialize)]
pub struct VaultStats {
    pub vault_name: String,
    pub key_count: u64,
    pub total_bytes: u64,
    pub avg_key_size: f64,
    pub avg_value_size: f64,
    pub last_accessed: Option<u64>,
    pub access_count_24h: u64,
    pub top_keys: Vec<(String, u64)>, // key, access_count
}

impl SqliteStateMachine {
    pub async fn get_vault_stats(&self, vault: &str) -> Result<VaultStats> {
        let conn = self.read_pool.get()?;

        // Get basic stats
        let (key_count, total_bytes): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), SUM(LENGTH(value))
             FROM state_machine_kv
             WHERE key >= ?1 AND key < ?2",
            params![format!("vault:{}:", vault), format!("vault:{}:", vault + "\x00")],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // Get access patterns
        let access_stats = conn.query_row(
            "SELECT MAX(timestamp), COUNT(*)
             FROM vault_access_log
             WHERE vault_name = ?1
             AND timestamp > unixepoch() - 86400",
            params![vault],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        Ok(VaultStats {
            vault_name: vault.to_string(),
            key_count: key_count as u64,
            total_bytes: total_bytes as u64,
            avg_key_size: /* calculate */,
            avg_value_size: /* calculate */,
            last_accessed: access_stats.0,
            access_count_24h: access_stats.1,
            top_keys: /* query top accessed keys */,
        })
    }
}
```

## Implementation Priority

1. **Quick Wins** (1-2 hours):
   - Add indexes to existing tables
   - Implement prepared statement caching
   - Add basic vault limits

2. **Medium Effort** (4-8 hours):
   - Vault metadata table with triggers
   - Atomic vault operations
   - Efficient range queries

3. **Larger Features** (1-2 days):
   - Export/import functionality
   - Full metrics and observability
   - Access pattern tracking

## Performance Expectations

With these improvements:

- Prefix queries: 10-100x faster with indexes
- Vault listing: O(1) with metadata table vs O(n) scan
- Range queries: 2-5x faster than LIKE queries
- Atomic operations: Ensure consistency
- Metrics: Enable optimization based on actual usage

## Migration Path

1. Add new tables/indexes (backward compatible)
2. Deploy new code with feature flags
3. Backfill vault_metadata from existing data
4. Enable new features gradually
5. Monitor performance improvements
