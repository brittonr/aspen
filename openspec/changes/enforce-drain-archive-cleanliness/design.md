## Context

Drain mode is autonomous and must not rely on chat-only claims. Archive postconditions are easy to check deterministically.

## Goals / Non-Goals

**Goals:** produce durable queue-empty evidence and prevent stale active paths.
**Non-Goals:** alter OpenSpec archive merge semantics.

## Decisions

### 1. Add explicit post-archive drain audit

**Choice:** After archive, run a deterministic audit that lists active directories and checks `.drain-state.md`.
**Rationale:** Reviewers need a saved transcript, not a summary.
**Implementation:** Add `scripts/openspec-drain-audit.sh` or extend preflight with `--drain-complete`.

### 2. Rewrite archive evidence paths before final preflight

**Choice:** Archived `verification.md` and `tasks.md` evidence paths must point to archive paths.
**Rationale:** Existing preflight already depends on current paths.
**Implementation:** Audit for stale `openspec/changes/<name>` paths outside diff artifacts.

## Risks / Trade-offs

**Diff artifacts contain old paths** → Allow old paths inside saved diff files, but require current verification/task coverage paths to be archive-relative.

## Validation Plan

Positive archived fixture and negative leftover-active-path fixture.
