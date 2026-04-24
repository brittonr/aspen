# Binary Shell Audit

This audit covers task `I19` for the work completed so far in `prepare-crate-extraction`.

## Scope reviewed

Task scope names these binary and app-shell paths:

- `src/bin/aspen_node`
- `crates/aspen-cli`
- dogfood (`crates/aspen-dogfood`)
- handlers
- bridges
- gateways
- web
- TUI

## Current change impact

The checked tasks in this slice changed OpenSpec artifacts, extraction documentation, policy, baseline evidence, and the readiness checker script. No Rust runtime source under the binary/app-shell paths above was modified in this slice.

Changed implementation/script path:

- `scripts/check-crate-extraction-readiness.rs`

Changed documentation/policy/evidence paths:

- `docs/crate-extraction.md`
- `docs/crate-extraction/*.md`
- `docs/crate-extraction/policy.ncl`
- `openspec/changes/prepare-crate-extraction/**`

## Result

No reusable behavior was newly added to binary/app-shell paths in the checked slice, so no migration from binaries into library crates is required yet.

Future code-movement tasks (`I7` through `I12`) must refresh this audit if they touch `src/bin/aspen_node`, `crates/aspen-cli`, dogfood, handlers, bridges, gateways, web, or TUI code.
