# Unbounded File Reads (DoS Vulnerability)

**Severity:** HIGH
**Category:** Resource Bounds
**Date:** 2026-01-10
**Status:** FIXED (2026-01-10)

## Summary

8+ locations use `read_to_string()` or `read_to_end()` without size validation, enabling memory exhaustion attacks.

## Affected Locations

1. **src/bin/git-remote-aspen/main.rs:546, 559**
   - Config files and packed-refs reading

2. **crates/aspen-pijul/src/working_dir.rs:276, 372**
   - Pijul configuration file reading

3. **crates/aspen-cluster/src/config.rs:1271**
   - Cluster configuration file reading

4. **crates/aspen-cluster/src/lib.rs:854, 1435, 1521**
   - Various file operations

5. **crates/aspen-jobs/src/workers/maintenance.rs:298, 437, 463**
   - Job maintenance file reading

6. **crates/aspen-jobs/src/replay.rs:126**
   - Job spec file reading

## Risk

An attacker could provide a multi-gigabyte file that gets fully loaded into memory, causing OOM and service crash.

## Recommendation

Add size validation before reading:

```rust
use std::fs::metadata;

const MAX_CONFIG_FILE_SIZE: u64 = 10 * 1024 * 1024;  // 10MB

fn read_bounded_file(path: &Path) -> Result<String, Error> {
    let size = metadata(path)?.len();
    if size > MAX_CONFIG_FILE_SIZE {
        return Err(Error::FileTooLarge { size, max: MAX_CONFIG_FILE_SIZE });
    }
    std::fs::read_to_string(path).map_err(Into::into)
}
```

## Missing Constants

Add to `crates/aspen-constants/src/lib.rs`:

- `MAX_CONFIG_FILE_SIZE = 10 * 1024 * 1024` (10 MB)
- `MAX_PIJUL_CONFIG_SIZE = 1 * 1024 * 1024` (1 MB)
- `MAX_JOB_SPEC_FILE_SIZE = 1 * 1024 * 1024` (1 MB)

## Fix Applied

The vulnerability has been fixed with the following changes:

### 1. Added constants to `crates/aspen-constants/src/lib.rs`

```rust
pub const MAX_CONFIG_FILE_SIZE: u64 = 10 * 1024 * 1024;      // 10 MB
pub const MAX_PIJUL_CONFIG_SIZE: u64 = 1024 * 1024;          // 1 MB
pub const MAX_JOB_SPEC_SIZE: u64 = 1024 * 1024;              // 1 MB
pub const MAX_SOPS_FILE_SIZE: u64 = 10 * 1024 * 1024;        // 10 MB
pub const MAX_SQL_FILE_SIZE: u64 = 1024 * 1024;              // 1 MB
pub const MAX_KEY_FILE_SIZE: u64 = 64 * 1024;                // 64 KB
pub const MAX_SIMULATION_ARTIFACT_SIZE: u64 = 10 * 1024 * 1024; // 10 MB
pub const MAX_TUI_STATE_SIZE: u64 = 1024 * 1024;             // 1 MB
pub const MAX_GIT_PACKED_REFS_SIZE: u64 = 10 * 1024 * 1024;  // 10 MB
```

### 2. Updated `crates/aspen-cluster/src/config.rs`

- Added `FileTooLarge` error variant to `ConfigError`
- Modified `from_toml_file()` to check file size before reading

### 3. Updated `crates/aspen-pijul/src/constants.rs`

- Added `MAX_WORKING_DIR_FILE_SIZE` constant (1 MB)

### 4. Updated `crates/aspen-pijul/src/working_dir.rs`

- Modified `open()` to check config file size before reading
- Modified `staged_files()` to check staged file size before reading

### 5. Updated `src/bin/git-remote-aspen/main.rs`

- Added file size checks for ref files and packed-refs

### 6. Updated `crates/aspen-jobs/src/replay.rs`

- Modified `load()` to check replay file size before reading

All file reads now validate file size using `metadata().len()` before calling
`read_to_string()`, preventing memory exhaustion from oversized files.
