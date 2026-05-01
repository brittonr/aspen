## Context

Full workspace checks may be too expensive for small changes, but skipping them silently creates review gaps.

## Goals / Non-Goals

**Goals:** make verification scope explicit and durable.
**Non-Goals:** force full workspace nextest for tiny doc-only changes when a justified scoped rail is enough.

## Decisions

### 1. Use a verification matrix

**Choice:** `verification.md` includes a `## Drain Verification Matrix` for implementation changes.
**Rationale:** Matrix makes full, scoped, skipped, and blocked states reviewable.
**Implementation:** Columns: rail, command, status, artifact, scope rationale.

### 2. Require explicit blocker for missing full cycle

**Choice:** Missing build/nextest/fmt entries fail unless marked doc-only or blocked with reason.
**Rationale:** Avoids silent omissions.
**Implementation:** Preflight checks matrix entries for checked implementation tasks.

## Risks / Trade-offs

**More process for tiny changes** → Permit doc-only bypass with no source code changes.

## Verification Strategy

- `openspec-governance.drain-verification-cycle.full-cycle-recorded`: full-cycle fixture where build, test, and format rows are present with commands, pass status, and artifacts.
- `openspec-governance.drain-verification-cycle.scoped-alternative-recorded`: scoped-alternative fixture where a rail records command, scoped status, artifact, scope rationale, and next best check.
- `openspec-governance.drain-verification-cycle.doc-only-bypass-explicit`: doc-only fixture where no source files changed and build/test/format rails are explicitly marked doc-only with rationale.
- `openspec-governance.drain-verification-cycle.missing-cycle-fails`: negative fixture with checked implementation work and no matrix.
- `openspec-governance.drain-verification-cycle.final-verification-requires-matrix-first`: negative fixture with final verification checked before a complete matrix.

## Validation Plan

Fixtures for full cycle, scoped replacement, and missing matrix.
