# Dependency Vulnerabilities

**Severity:** LOW (was HIGH)
**Category:** Security
**Date:** 2026-01-10
**Updated:** 2026-01-10

## Summary

Original audit identified 5 security vulnerabilities. **3 have been resolved.**

## Current Status

### FIXED (2026-01-10)

| Advisory | Package | Resolution |
|----------|---------|------------|
| RUSTSEC-2025-0140 | gix-date | Upgraded to 0.12.1 |
| RUSTSEC-2025-0021 | gix-features | Upgraded to 0.41.1+ |

### Feature-Gated (Not in Default Build)

These vulnerabilities only affect builds with `--features pijul`:

| Advisory | Package | Notes |
|----------|---------|-------|
| RUSTSEC-2024-0344 | curve25519-dalek 3.2.0 | Via libpijul (pijul feature only) |
| RUSTSEC-2022-0093 | ed25519-dalek 1.0.1 | Via libpijul (pijul feature only) |

The `pijul` feature is NOT in defaults. Default builds use:

- curve25519-dalek 4.1.3 (fixed)
- ed25519-dalek 2.2.0+ (fixed)

### No Fix Available

| Advisory | Package | Notes |
|----------|---------|-------|
| RUSTSEC-2023-0071 | rsa 0.9.10 | Via ssh-key -> aspen-forge. No upstream fix. |

The rsa vulnerability (Marvin Attack) has medium severity (5.9) and no fix is available upstream.

## Unmaintained Dependencies (Warnings)

These are informational warnings, not security vulnerabilities:

- `atomic-polyfill 1.0.3` - via postcard (iroh dependency)
- `atty 0.2.14` - via cargo-nextest (dev dependency)
- `bincode 1.3.3` - via madsim (test dependency)
- `fxhash 0.2.1` - via tui-logger (tui feature only)
- `instant 0.1.13` - via iroh
- `number_prefix 0.4.0` - via indicatif (cli dependency)
- `paste 1.0.15` - via ratatui (tui feature only)

## Recommendation

1. Monitor rsa crate for security patches
2. Consider removing `pijul` feature if not needed (eliminates libpijul vulns from Cargo.lock)
3. Upstream unmaintained deps are transitive - monitor for updates
