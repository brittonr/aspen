## Context

Delta specs are the source of truth for change semantics, but current checks miss several common consistency errors.

## Goals / Non-Goals

**Goals:** catch structural incoherence early.
**Non-Goals:** prove semantic correctness of all prose.

## Decisions

### 1. Parse IDs and section kinds deterministically

**Choice:** Validate `ADDED`, `MODIFIED`, and `REMOVED` sections with existing OpenSpec parsers where possible.
**Rationale:** Regex-only checks are brittle.
**Implementation:** Add a helper that maps requirement headings and IDs in main and delta specs.

### 2. Enforce modified-target existence

**Choice:** `MODIFIED` entries must match a main spec requirement by ID or heading.
**Rationale:** A modification to a missing base requirement is usually an accidental add.
**Implementation:** Fail unless the delta includes an explicit `Migration note:` marker.

## Risks / Trade-offs

**Legacy specs lacking IDs** → Gate only new active changes strictly; archived legacy specs may warn.

## Validation Plan

Fixture suite with valid add, valid modify, missing ID, and modify-missing-base cases.
