## Context

The Jobs/CI extraction already has an archived baseline change, portable fixtures, negative boundary checks, and post-extraction cleanup commits. Current docs still keep `jobs-ci-core` at `workspace-internal` because owner/public API review and fresh evidence are required before any readiness raise.

## Goals / Non-Goals

**Goals:**

- Make the owner/public API review scope explicit and reviewable.
- Document canonical reusable surfaces, compatibility shells, and adapter exclusions.
- Keep readiness at `workspace-internal` until fresh evidence is captured and the readiness fields are intentionally changed.

**Non-Goals:**

- No Rust API movement in this start slice.
- No publishable/repo-split state; that remains blocked by license/publication policy.
- No broad executor/handler stabilization.

## Decisions

### 1. Review starts as documentation and evidence work

**Choice:** Start with manifest/spec/review artifacts rather than code changes.

**Rationale:** Recent implementation slices already moved drift-prone contracts to owning crates. The blocker is now ownership, stability labels, and evidence freshness.

**Alternative:** Immediately raise readiness to `extraction-ready-in-workspace`. Rejected for this start slice because fresh evidence and task coverage should be captured first.

### 2. Canonical reusable surfaces are narrow

**Choice:** Treat `aspen-jobs-core`, `aspen-ci-core`, and `aspen-jobs-protocol` as reusable Jobs/CI surfaces. Treat `aspen-client-api::CI_REQUEST_VARIANTS` as protocol-owned metadata consumed by CI handler registration.

**Rationale:** These crates own dependency-light contracts and deterministic helpers. Runtime shells, handlers, executors, workers, node/bootstrap, Iroh/blob/Forge wiring, and process/Nix/VM integration remain adapter surfaces.

## Risks / Trade-offs

**Stale evidence risk** → Fresh downstream, negative-boundary, compatibility, and readiness-checker evidence must be regenerated before any readiness raise.

**Over-stabilization risk** → The review should not bless broad `aspen-jobs`, `aspen-ci`, handler, worker, or executor APIs as reusable defaults.
