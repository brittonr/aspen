## Context

`docs/crate-extraction.md` now records the next decomposition wave. Protocol/wire remains listed as `workspace-internal` even though its manifest already points at completed downstream serialization fixture, negative runtime-boundary, and compatibility evidence. Policy is partially ahead of the manifest: `aspen-coordination-protocol` is already `extraction-ready-in-workspace`, while the sibling protocol crates still say `workspace-internal`.

## Goals / Non-Goals

**Goals:**

- Make the protocol/wire family readiness state match its verified boundary evidence.
- Re-run the evidence instead of relying only on archived outputs.
- Keep runtime handlers and concrete transport crates out of this readiness raise.

**Non-goals:**

- No new wire variants or serialization shape changes.
- No publication/repository split claim.
- No owner reassignment; owner remains Aspen protocol maintainers.

## Decisions

### 1. Promote only protocol/wire crates

**Choice:** Raise `aspen-client-api`, `aspen-forge-protocol`, `aspen-jobs-protocol`, and the family manifest/inventory to `extraction-ready-in-workspace`; leave runtime consumers as compatibility targets.

**Rationale:** The protocol crates are direct schema crates with no handler registry, node bootstrap, concrete transport endpoint, UI/web binary, or runtime auth shell dependency in their reusable default surfaces. `aspen-coordination-protocol` is already policy-marked ready and is included for family consistency.

**Alternative:** Leave the family `workspace-internal` until publication policy is decided. Rejected because the readiness contract explicitly allows `extraction-ready-in-workspace` before publication and blocks only publishable/repo-split states.

### 2. Use existing checker evidence names

**Choice:** Generate the fixed artifacts required by `scripts/check-crate-extraction-readiness.rs` for `protocol-wire`: downstream metadata, forbidden grep, and client compatibility evidence.

**Rationale:** The checker enforces family-specific evidence names. Reusing them avoids weakening policy and makes the new readiness report deterministic.

## Verification Strategy

This strategy covers `architecture-modularity.protocol-wire-readiness`, `architecture-modularity.protocol-wire-readiness.no-publication-claim`, and `architecture-modularity.protocol-wire-readiness.runtime-boundary`.

- Build/check protocol crates in default and no-default configurations where supported.
- Run the downstream fixture against canonical protocol crates with a fresh target directory.
- Capture cargo metadata for the fixture and negative boundary grep output proving forbidden runtime imports are absent.
- Run `cargo test -p aspen-client-api` for wire compatibility tests.
- Run the crate-extraction readiness checker with JSON and Markdown outputs under this change.
- Validate, archive, preflight, and drain-audit the OpenSpec package before commit.

## Risks / Trade-offs

**Stale archived fixture path** → Run the existing archived downstream fixture in place and save fresh outputs under this change.

**Overclaiming publication readiness** → Keep all docs at `extraction-ready-in-workspace` and retain publication blocks.

**Runtime dependency leaks through feature defaults** → Let the checker and cargo tree/grep evidence fail the change if app/runtime crates appear in reusable protocol surfaces.
