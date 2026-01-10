# Cargo Deny License Configuration Issues

**Severity:** LOW
**Category:** Build/CI
**Date:** 2026-01-10

## Summary

`cargo deny check` reports license rejection errors for aspen crates using GPL-2.0-or-later.

## Issue

```
error[rejected]: failed to satisfy license requirements
  - license = "GPL-2.0-or-later"
  - rejected: license is not explicitly allowed
```

## Affected Crates

All internal aspen-* crates:

- aspen
- aspen-api
- aspen-auth
- aspen-blob
- aspen-client
- aspen-client-api
- (and all other internal crates)

## Root Cause

The `deny.toml` license allowlist doesn't include GPL-2.0-or-later, which is the license used by all aspen crates.

## Recommendation

Add GPL-2.0-or-later to the allowed licenses in `deny.toml`:

```toml
[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "GPL-2.0-or-later",  # Add this
]
```

Or mark internal crates as private/unpublished to skip license checks.
