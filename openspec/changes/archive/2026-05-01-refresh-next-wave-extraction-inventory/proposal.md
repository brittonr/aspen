## Why

The next-five decomposition implementation has completed evidence for foundational-types, auth-ticket, jobs-ci-core, trust-crypto-secrets, and testing-harness, but the top-level crate-extraction inventory still describes those families as missing manifests, missing owners, or blocked on first actions that are now complete.

That stale inventory makes the next extraction step ambiguous and can send future work back to already-completed blockers.

## What Changes

- Refresh the broader crate-extraction inventory rows for the five implemented families.
- Point each family at its manifest and named owner group.
- Keep readiness at `workspace-internal` where policy still blocks raising the whole family, but update next actions to the current remaining blocker.
- Add OpenSpec governance coverage requiring next-wave inventory rows to stay synchronized with manifest evidence.

## Capabilities

### Modified Capabilities
- `architecture-modularity`: crate-extraction inventory consistency for completed next-wave family evidence.

## Verification Expectations

- Cover `architecture-modularity.extraction-inventory-tracks-next-wave-evidence`, `architecture-modularity.extraction-inventory-tracks-next-wave-evidence.manifest-links`, and `architecture-modularity.extraction-inventory-tracks-next-wave-evidence.current-next-actions` in the inventory refresh evidence.
- Validate the active OpenSpec change with `openspec validate refresh-next-wave-extraction-inventory --json`.
- Verify the package mechanics with the OpenSpec helper.
- Run `scripts/openspec-preflight.sh refresh-next-wave-extraction-inventory` after staging evidence so checked tasks remain evidence-backed.
- Negative path: stale inventory rows that cite missing manifests, `owner needed` despite manifest owners, or completed first blockers as next actions are rejected by review of `docs/crate-extraction.md` against the manifests.
- Run diff whitespace checks before commit.

## Impact

- **Files**: `docs/crate-extraction.md`, OpenSpec artifacts and evidence.
- **APIs**: none.
- **Dependencies**: none.
- **Testing**: `openspec validate`, helper verification, preflight, and diff whitespace checks.
