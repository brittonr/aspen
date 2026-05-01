## Why

The protocol/wire family already has direct manifests, downstream fixture evidence, postcard/wire compatibility tests, and deterministic dependency-boundary checks from the earlier extraction work. The top-level inventory still reports the family as `workspace-internal`, which makes the next extraction queue repeat a completed verification step.

## What Changes

- Promote the protocol/wire family from `workspace-internal` to `extraction-ready-in-workspace` inside the crate-extraction inventory and manifest.
- Align the policy entries for `aspen-client-api`, `aspen-forge-protocol`, and `aspen-jobs-protocol` with the already-ready `aspen-coordination-protocol` entry.
- Re-run the downstream fixture, compatibility, and dependency-boundary rails with fresh evidence under this change.

## Out of Scope

- Publication, repository split, or license policy changes.
- Runtime handler, transport, UI, CLI, or binary shell readiness changes.
- Wire schema changes; this is verification and readiness bookkeeping over existing protocol APIs.

## Verification Expectations

- Cover `architecture-modularity.protocol-wire-readiness`, `architecture-modularity.protocol-wire-readiness.no-publication-claim`, and `architecture-modularity.protocol-wire-readiness.runtime-boundary`.
- Save fresh downstream fixture metadata, negative boundary grep, client compatibility tests, checker reports, OpenSpec validation, preflight, and drain-audit evidence.
- Keep readiness capped at `extraction-ready-in-workspace`; publishable states remain blocked until human license/publication policy is decided.

## Impact

- **Files**: `docs/crate-extraction.md`, `docs/crate-extraction/protocol-wire.md`, `docs/crate-extraction/policy.ncl`, OpenSpec artifacts/evidence.
- **APIs**: none.
- **Dependencies**: none.
- **Testing**: protocol crate cargo checks/tests, downstream fixture, readiness checker, OpenSpec validate/preflight/audit.
