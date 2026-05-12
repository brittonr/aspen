## Context

`docs/crate-extraction.md` marks four Redb Raft KV layers ready and leaves `aspen-raft-network` internal due to transitive app-concern paths through transport/sharding.

## Goals / Non-Goals

**Goals:** split minimal adapter checks from runtime integration, add evidence, and update readiness docs if the adapter passes.

**Non-Goals:** extract `aspen-raft` compatibility shell, replace Iroh, or change consensus semantics.

## Decisions

### 1. Feature topology first

**Choice:** Start by recording current cargo-tree leaks, then patch feature ownership.
**Rationale:** Avoid guessing which dependency path is still app-scoped.

### 2. Preserve runtime bundles

**Choice:** If defaults narrow, add compatibility feature bundles for existing runtime consumers.
**Rationale:** Extraction readiness should not force broad caller migrations in the same slice.

## Risks / Trade-offs

**Feature unification can mask leaks** → Verify both package-level minimal graphs and representative workspace consumers.
