# Aspen patches for iroh 0.98.2

Aspen vendors `iroh` temporarily to relax upstream prerelease Hickory pins while remediating RustSec DNS advisories.

## Local changes

- `Cargo.toml`: allow `hickory-resolver = 0.26.1` instead of the upstream prerelease `=0.26.0-beta.4`.
- Removed registry packaging metadata and repository-only CI/config files from the vendored copy.

## Removal trigger

Drop this patch and return to crates.io once upstream `iroh` publishes a release that depends on Hickory `0.26.1` or newer and still avoids the `postcard`/`atomic-polyfill` default-feature edge in Aspen's selected graph.
