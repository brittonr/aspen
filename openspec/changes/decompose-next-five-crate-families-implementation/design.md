## Context

The parent `decompose-next-five-crate-families` change selected the next decomposition wave and completed roadmap/manifest setup. This implementation change carries the deferred first-blocker work forward while preserving the existing extraction discipline: no publication/repository split, no readiness promotion without evidence, and compatibility rails for existing Aspen consumers.

## Goals / Non-Goals

**Goals:**

- Make the readiness policy and checker understand the five selected families before any code movement.
- Land bounded first-blocker slices in dependency order: foundational seams first, then auth/tickets, jobs/CI, trust/secrets, and testing harness.
- Require downstream-style fixtures, negative boundary proofs, positive tests, compatibility checks, and verification index coverage before readiness changes.

**Non-Goals:**

- Do not publish crates, split repositories, or mark anything beyond `workspace-internal` / `extraction-ready-in-workspace`.
- Do not redo archived generic protocol/wire, transport/RPC, blob/castore/cache, KV branch/commit-DAG, coordination, or Raft KV extraction work.
- Do not make broad multi-family rewrites without completing the prior family evidence gate or recording an out-of-order guard.

## Decisions

### 1. Policy/checker first

**Choice:** Start by extending `docs/crate-extraction/policy.ncl` and `scripts/check-crate-extraction-readiness.rs` with family-aware candidate selection and selected-wave evidence requirements.

**Rationale:** The checker is the guardrail that prevents premature readiness promotion and gives each later first-blocker slice deterministic feedback.

**Alternative:** Move code first and retrofit policy later. Rejected because the parent requirements explicitly require policy/checker coverage before readiness changes.

### 2. Foundational seams remain the first implementation family

**Choice:** Work `foundational-types` before dependent families unless a task records the bypassed prerequisite and temporary compatibility guard.

**Rationale:** Jobs/CI, trust/secrets, and test harness slices consume storage/trait/type boundaries. Moving those first would hide dependency leaks behind compatibility paths.

### 3. Evidence gates are per-family

**Choice:** Each family must produce its own downstream metadata, forbidden-dependency proof, compatibility transcript, and positive/negative test evidence under this change directory before any checked completion claim.

**Rationale:** The selected wave is intentionally five families; per-family evidence prevents a single broad summary from masking missing proofs.

## Risks / Trade-offs

- **Checker strictness may fail current families** -> Keep readiness at `workspace-internal` and treat missing evidence as expected until the corresponding family task lands.
- **Large dependency graph churn** -> Keep compatibility re-exports/paths documented and tested before removing old imports.
- **Security-sensitive auth/trust changes** -> Require positive and malformed-input negative tests before compatibility claims.
