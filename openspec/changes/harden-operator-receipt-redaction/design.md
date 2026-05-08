## Context

Aspen's evidence model depends on durable receipts that can be shared with operators. The evidence must be useful without exposing cluster tickets, cookies, private keys, connection strings, or other secret material.

## Goals / Non-Goals

**Goals:** add fail-closed redaction tests for receipt render/diagnose paths and document the evidence boundary.

**Non-Goals:** build a general-purpose secret scanner, modify cryptographic storage, or publish receipts externally in this change.

## Decisions

### 1. Pure helper coverage

**Choice:** Prefer pure render/diagnose helpers returning strings or structured summaries so tests can assert redaction without live clusters.

**Rationale:** It makes negative cases deterministic and cheap.

**Alternative:** End-to-end-only CLI tests were rejected as slower and less precise for redaction assertions.

### 2. Marker-based negative fixtures

**Choice:** Tests should inject recognizable secret markers and assert they do not appear in output.

**Rationale:** Marker tests catch accidental formatting regressions directly.

## Risks / Trade-offs

**Over-redaction** → Preserve non-secret identifiers, hashes, artifact names, and stage categories so evidence remains useful.

**Under-coverage** → Cover list/show/diagnose and any runtime-host evidence summaries touched by implementation.
