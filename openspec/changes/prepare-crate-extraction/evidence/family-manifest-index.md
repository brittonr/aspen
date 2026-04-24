# Broader extraction family manifest index

This artifact anchors checked tasks `I13`, `I14`, `I15`, and `I18` after the initial family manifest files were committed. It records the durable manifest paths and inventory synchronization target.

## Manifest stubs

- `docs/crate-extraction/foundational-types.md` covers foundational type/helper candidates, including `crates/aspen-storage-types`, `crates/aspen-cluster-types`, and `crates/aspen-traits`.
- `docs/crate-extraction/auth-ticket.md` covers auth/ticket candidates and their runtime feature contract.
- `docs/crate-extraction/protocol-wire.md` covers protocol/wire candidates, including `crates/aspen-client-api` and wire-compatibility rails.

## Inventory synchronization

- `docs/crate-extraction.md` remains the canonical inventory index.
- Rows affected by `I13` through `I17` carry owner, manifest link or explicit `manifest not yet created` status, readiness state, and next-action fields synchronized with these family manifests and `openspec/changes/prepare-crate-extraction/evidence/service-follow-up-selection.md`.
