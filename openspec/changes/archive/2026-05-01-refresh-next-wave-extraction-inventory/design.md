## Context

The detailed manifests for the next-five decomposition wave now contain implementation status sections and evidence references, while the broader inventory still has the pre-implementation placeholders for owners, manifests, and first actions.

## Goals / Non-Goals

**Goals:** make the top-level inventory match the implemented manifests and prevent duplicate work on completed blockers.

**Non-Goals:** raise any family to `publishable from monorepo`, perform a repository split, or claim external semver readiness.

## Decisions

### 1. Refresh inventory only to evidence-backed state

**Choice:** Update rows for foundational-types, auth-ticket, jobs-ci-core, trust-crypto-secrets, and testing-harness to point to existing manifests and current next actions.

**Rationale:** The family manifests are the detailed source; the inventory should summarize them without overstating publication readiness.

**Alternative:** Leave stale rows until a broader publication-policy change. Rejected because it causes future agents to reattempt completed blockers.

**Implementation:** Edit only `docs/crate-extraction.md` plus this OpenSpec change package.

## Verification Strategy

This docs/governance-only change covers `architecture-modularity.extraction-inventory-tracks-next-wave-evidence`, `architecture-modularity.extraction-inventory-tracks-next-wave-evidence.manifest-links`, and `architecture-modularity.extraction-inventory-tracks-next-wave-evidence.current-next-actions` with the smallest relevant rails:

- **Build rail:** `openspec validate refresh-next-wave-extraction-inventory --json` validates the delta spec and package structure.
- **Test rail:** `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify refresh-next-wave-extraction-inventory --json` verifies completed tasks and delta-spec mechanics.
- **Format rail:** `git diff --check` and `git diff --cached --check` verify markdown whitespace.
- **Evidence rail:** `scripts/openspec-preflight.sh refresh-next-wave-extraction-inventory` verifies checked tasks, durable evidence, and changed-file listings.

## Risks / Trade-offs

**Readiness overclaim** → Keep family readiness at `workspace-internal` unless existing manifests explicitly prove and allow `extraction-ready-in-workspace` for the whole family.

**Evidence path drift** → Validate and preflight the change before commit; archive after completion.
