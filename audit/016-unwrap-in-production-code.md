# Unwrap/Expect in Production Code

**Severity:** MEDIUM
**Category:** Error Handling
**Date:** 2026-01-10

## Summary

1,578 `.unwrap()` and 503 `.expect()` calls found in crates, with some in production code paths.

## Statistics

- `.unwrap()` calls: 1,578 across 142 files
- `.expect()` calls: 503 across 61 files
- `panic!()` calls: 83 across 21 files
- `todo!()`: 0 (good)
- `unimplemented!()`: 0 (good)

## Notable Production Code Issues

### generate_fuzz_corpus.rs - 53 violations

**File:** `src/bin/generate_fuzz_corpus.rs`
Contains many `.unwrap()` on file operations.

### aspen-fuse/src/inode.rs - 8 violations

**File:** `crates/aspen-fuse/src/inode.rs`
Lock poisoning handled with `.expect()` - documented fail-fast.

### aspen-fuse/src/main.rs - 2 violations

**File:** `crates/aspen-fuse/src/main.rs:216, 299`
Redundant `.expect()` calls after validation already confirmed values exist.

## Recommendation

1. Replace `.unwrap()` with `?` operator where possible
2. Replace `.expect()` with `.context()` from snafu/anyhow
3. Use `.unwrap_or_default()` for non-critical cases
4. Document and keep fail-fast `.expect()` only for truly unrecoverable states

Example fix:

```rust
// Before
let file = std::fs::File::open(path).unwrap();

// After
let file = std::fs::File::open(path)
    .context(OpenFileSnafu { path })?;
```
