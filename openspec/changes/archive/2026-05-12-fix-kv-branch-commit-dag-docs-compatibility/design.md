## Context

`docs/crate-extraction/kv-branch-commit-dag.md` records a pre-existing `aspen-docs --features commit-dag-federation` blocker attributed to iroh-blobs/docs and RNG API skew. The branch/DAG extraction itself is otherwise mostly proven.

## Goals / Non-Goals

**Goals:** make the docs representative consumer compile, keep reusable branch/DAG defaults clean, and update evidence.

**Non-Goals:** redesign iroh-docs synchronization, publish crates, or change commit hash semantics.

## Decisions

### 1. Reproduce before patching

**Choice:** Capture the exact failing `cargo check` transcript first.
**Rationale:** The blocker is described but may have shifted since the last evidence.

### 2. Localize the fix to compatibility edges

**Choice:** Prefer feature wiring or adapter shims in `aspen-docs` over branch/DAG API churn.
**Rationale:** Branch/DAG readiness depends on stable reusable defaults.

## Risks / Trade-offs

**Dependency skew may be upstream/vendor-bound** → If the fix requires broad dependency upgrades, defer with a narrower sub-change and preserve current readiness state.
