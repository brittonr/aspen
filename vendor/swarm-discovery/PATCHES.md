# Aspen patches for swarm-discovery 0.6.0-alpha.2

Aspen vendors `swarm-discovery` temporarily to relax upstream prerelease Hickory pins while remediating RustSec DNS advisories.

## Local changes

- `Cargo.toml`: allow `hickory-proto = 0.26.1` instead of upstream prerelease `=0.26.0-beta.4`.
- Removed registry packaging metadata and repository-only CI/config files from the vendored copy.

## Removal trigger

Drop this patch and return to crates.io once upstream `swarm-discovery` publishes a release that depends on Hickory `0.26.1` or newer.
