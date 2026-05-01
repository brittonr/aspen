## Context

The current design template allows validation plans but does not make requirement coverage mandatory. Recent JJ and extraction reviews show this gap.

## Goals / Non-Goals

**Goals:** make verification planning first-class at design time.
**Non-Goals:** require full command transcripts before implementation.

## Decisions

### 1. Verification Strategy is mandatory when specs exist

**Choice:** Designs touching delta specs must include `## Verification Strategy`.
**Rationale:** The design stage is the last point before tasks where requirement coverage can be shaped cheaply.
**Implementation:** Update template guidance and gate logic to require the heading and at least one requirement/scenario ID reference.

### 2. Require positive and negative path mapping

**Choice:** Strategy entries must identify happy-path and failure-path checks when the spec includes negative scenarios.
**Rationale:** Negative path omissions recur in review.
**Implementation:** Gate scans delta specs for scenario names or failure keywords and verifies corresponding strategy text references.

## Risks / Trade-offs

**Overly rigid matching** → Fail missing strategies, missing ID references, and negative-path entries that lack either a named check or explicit defer-with-rationale once fixtures stabilize.

## Verification Strategy

- `openspec-governance.design-verification-strategy.mapped-strategy-present`: fixture with `## Verification Strategy` citing requirement/scenario IDs plus a real sample-change design gate run.
- `openspec-governance.design-verification-strategy.missing-strategy-fails`: negative fixture with delta specs but no strategy section.
- `openspec-governance.design-verification-strategy.negative-paths-planned`: negative fixture where changed failure behavior lacks a negative check or defer-with-rationale entry.

## Validation Plan

Fixture tests for missing strategy, missing requirement ID, missing negative-path mapping, and valid mapped strategy; plus one real sample-change gate run.
