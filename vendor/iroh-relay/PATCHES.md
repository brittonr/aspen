# Aspen patches for iroh-relay 0.98.0

Aspen vendors `iroh-relay` temporarily to relax upstream prerelease Hickory pins while remediating RustSec DNS advisories.

## Local changes

- `Cargo.toml`: allow `hickory-proto = 0.26.1` and `hickory-resolver = 0.26.1` instead of upstream prerelease `=0.26.0-beta.4` pins.
- Removed registry packaging metadata and repository-only CI/config files from the vendored copy.

## Removal trigger

Drop this patch and return to crates.io once upstream `iroh-relay` publishes a release that depends on Hickory `0.26.1` or newer.
