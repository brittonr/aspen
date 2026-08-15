## Why

Molten is a network-facing distributed runtime that already links AGPL-licensed OnixResearch components, while its package metadata and current documentation still declare `MIT OR Apache-2.0`. That split is misleading for distributors and conflicts with the repository's active AGPL distribution-profile work.

## What Changes

- License Molten-owned source as `AGPL-3.0-or-later`.
- Ship the complete AGPL license text and state the boundary in current documentation.
- Preserve every vendored or third-party component under its upstream license and notices.
- Keep the broader dependency-pin and release-evidence work in `pin-canonical-evidence-dependencies` separate.

## Impact

- **Distribution**: New Molten releases use AGPL network copyleft; previously received permissive releases remain under their existing grants.
- **Dependencies**: No runtime dependency changes.
- **Claims**: The repository records its selected license; it does not claim universal legal compliance or relicense third-party code.
