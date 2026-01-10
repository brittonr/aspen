# Missing FUSE Permission Checks

**Severity:** MEDIUM
**Category:** Security
**Date:** 2026-01-10

## Summary

The FUSE filesystem implementation does not enforce Unix permission checks.

## Affected Location

**File:** `crates/aspen-fuse/src/fs.rs` (Lines 906-911)

```rust
// For now, we allow all access (no permission enforcement)
// A real implementation would check ctx.uid/gid against file owner
```

## Issue

All FUSE operations allow any user, regardless of:

- File ownership (uid/gid)
- Permission bits (rwx)
- ACLs

## Risk

Any local user can read/write/execute all files in the mounted filesystem, bypassing intended access controls.

## Recommendation

Implement Unix permission checks:

```rust
fn check_access(&self, inode: u64, mask: i32, req: &Request<'_>) -> Result<(), libc::c_int> {
    let entry = self.get_entry(inode)?;
    let uid = req.uid();
    let gid = req.gid();

    // Root always has access
    if uid == 0 {
        return Ok(());
    }

    // Check owner permissions
    if entry.uid == uid {
        if (entry.mode >> 6) & mask as u32 == mask as u32 {
            return Ok(());
        }
    }

    // Check group permissions
    if entry.gid == gid {
        if (entry.mode >> 3) & mask as u32 == mask as u32 {
            return Ok(());
        }
    }

    // Check other permissions
    if entry.mode & mask as u32 == mask as u32 {
        return Ok(());
    }

    Err(libc::EACCES)
}
```
