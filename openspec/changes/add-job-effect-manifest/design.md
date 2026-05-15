## Context

Aspen already treats authorization and credential output as security boundaries, but effects are not consistently represented as one typed manifest. A manifest can become the shared contract between closure admission, UCAN authorization, worker sandboxing, and receipts.

## Goals / Non-Goals

**Goals:**
- Make effect requirements explicit, versioned, and reviewable.
- Enforce deny-by-default before runtime start.
- Feed sandbox policy and redaction from the same declarations.

**Non-Goals:**
- Build a new programming language effect system.
- Prove kernel-level sandbox enforcement for all runtimes in one change.
- Remove existing capability checks before compatibility is proven.

## Decisions

### 1. Manifest declares requested effects, admission grants capabilities

**Choice:** The manifest records requested effects; admission maps those requests to granted UCAN/capability handles and records a redacted proof summary.

**Rationale:** Requested effects and granted authority are different facts and should both be auditable.

### 2. Deny undeclared or ungranted effects

**Choice:** The first enforced executor slice must reject operations outside the manifest or without granted capability.

**Rationale:** This prevents manifests from becoming documentation-only.

### 3. Receipts use effect taxonomy for redaction

**Choice:** Receipt rendering consults the effect taxonomy so secret/config/network/capability handles are summarized safely.

**Rationale:** The same effect boundary that admits execution should govern output safety.

## Risks / Trade-offs

**Taxonomy too broad** → Start with common first-party effects and allow versioned extension.

**Executor mismatch** → Land one executor slice with tests before claiming workspace-wide enforcement.

**Policy duplication** → Keep UCAN/capability mapping in one reusable helper where practical.
