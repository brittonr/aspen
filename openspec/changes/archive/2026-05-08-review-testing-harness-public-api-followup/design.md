## Context

`testing-harness-extraction` already requires reusable defaults and explicit adapters, but its canonical purpose still reflects an archived placeholder. Runtime-host readiness now depends on suite inventory APIs and validation behavior, making a focused follow-up worthwhile.

## Goals / Non-Goals

**Goals:** review public API boundaries, preserve reusable inventory/check helpers, and prevent adapter dependency leakage.

**Non-Goals:** publish a crate, split the repository, or rewrite all harness layers in one slice.

## Decisions

### 1. API inventory before changes

**Choice:** Start by inventorying exported types/functions and dependency graphs.

**Rationale:** The current API may already be good enough in places; evidence should drive changes.

### 2. Negative adapter checks

**Choice:** Require negative checks proving VM/patchbay/madsim/network dependencies do not leak into defaults.

**Rationale:** Reusable harness value depends on keeping the core boundary small.

## Risks / Trade-offs

**Breaking tests** → Preserve compatibility re-exports or migrate call sites in focused commits.

**Scope creep** → Keep publication/repo split out of scope unless a future OpenSpec accepts it.
