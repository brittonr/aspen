# Path Traversal in FUSE Filesystem

**Severity:** MEDIUM
**Category:** Security
**Date:** 2026-01-10

## Summary

The FUSE filesystem implementation lacks path normalization, potentially allowing path traversal attacks.

## Affected Location

**File:** `crates/aspen-fuse/src/fs.rs` (Lines 86-88)

```rust
fn path_to_key(path: &str) -> String {
    path.trim_start_matches('/').to_string()
}
```

## Issues

1. **No path normalization**: Paths like `/dir/./secret` or `/dir/../parent` are not canonicalized
2. **No `..` rejection**: Parent directory traversal not blocked
3. **No symlink loop detection**: Symlinks stored with `.symlink` suffix but no cycle detection

## Risk

- Access parent directories via `/../` traversal
- Create symlink loops causing infinite traversal
- Store malicious symlink targets

## Recommendation

```rust
fn path_to_key(path: &str) -> Result<String, FsError> {
    // Reject paths with parent directory traversal
    if path.contains("..") {
        return Err(FsError::InvalidPath { reason: "parent directory traversal not allowed" });
    }

    // Normalize path (remove . and redundant separators)
    let normalized = PathBuf::from(path)
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .collect::<PathBuf>();

    Ok(normalized.to_string_lossy().into_owned())
}
```

Also add symlink loop detection with recursion limit during readlink operations.
