## Context

Proposal verification is currently free-form. Stronger structure will reduce later omissions without requiring implementation evidence up front.

## Goals / Non-Goals

**Goals:** make proposal verification expectations reviewable and requirement-aware.
**Non-Goals:** force exact command names before design chooses implementation details.

## Decisions

### 1. Use requirement IDs as the traceability anchor

**Choice:** Proposal verification bullets should cite requirement or capability IDs where available.
**Rationale:** IDs survive artifact stages and support deterministic checks.
**Implementation:** Gate extracts IDs from delta specs and checks proposal verification references or explicit deferrals.

### 2. Allow scoped deferrals

**Choice:** A proposal may defer exact rails to design if it names the requirement and reason.
**Rationale:** Early proposals sometimes know behavior but not exact test commands.
**Implementation:** Accept `defer to design` only with an ID and a reason.

## Risks / Trade-offs

**Proposal bloat** → Require concise mapping, not command transcripts.

## Validation Plan

Fixture proposals for complete mapping, missing ID mapping, and missing negative path.
