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

## Verification Strategy

- `openspec-governance.proposal-verification-traceability`: positive fixture covers proposals whose `## Verification Expectations` section maps every changed requirement/scenario ID.
- `openspec-governance.proposal-verification-traceability.cites-requirement-ids`: checker fixture covers complete ID citation and valid `defer to design` entries with requirement ID plus rationale.
- `openspec-governance.proposal-verification-traceability.missing-positive-verification-fails`: negative fixture omits an added requirement/scenario ID and must fail with that ID.
- `openspec-governance.proposal-verification-traceability.negative-behavior-needs-verification`: negative fixture includes rejection/failure behavior without a negative-path expectation and must fail.

## Validation Plan

Fixture proposals for complete mapping, missing ID mapping, missing negative path, valid deferral, and invalid deferral.
