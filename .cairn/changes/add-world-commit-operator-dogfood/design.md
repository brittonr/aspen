## Context

Each world capability has a separate owner and receipt. Operators still need a stable product-facing path that composes those capabilities without hiding their decisions.

A monolithic command that performs implicit reads and mutations would violate the functional-core and imperative-shell boundary. The operator surface must remain plan-first and explicit.

## Decisions

### Decision: Extend the existing Molten CLI

**Choice:** Add a `molten world` command family. Do not create a separate Realm binary or daemon.

Commands cover checkpoint, inspect, branch, run, diff, conflicts, replay, simulate, verify, promote, export, import, and garbage-collection planning.

**Rationale:** Molten owns the first world-commit implementation and its runtime composition root.

### Decision: Plan every operation from explicit facts

**Choice:** Typed requests name exact world commits, branch IDs, expected generations, snapshot profiles, policies, authority-observation references, resource limits, and operation IDs.

The pure workflow core returns an ordered operation graph, blockers, required rechecks, and deterministic plan identity. It never reads files, environment, clocks, networks, stores, or credentials.

**Rationale:** Explicit inputs make previews reproducible and prevent hidden latest-state or ambient-authority dependencies.

### Decision: Make mutation preview-first

**Choice:** Mutating commands default to plan mode. Apply requires the exact plan identity and rechecks every mutable observation inside the owning mutation boundary.

A stale plan fails before effects. An uncertain effect or commit returns its existing reconciliation class.

**Rationale:** One workflow must not weaken the concurrency or authority rules of its components.

### Decision: Link component receipts without flattening them

**Choice:** `world-workflow-receipt-v1` contains ordered references to capture, branch, run, diff, replay, promotion, replication, import, and retention receipts.

The aggregate records completion and first blocker. It does not reinterpret component evidence or claim stronger authority.

**Rationale:** Operators need one review index, while each repository and subsystem retains its own claim.

### Decision: Dogfood one supported vertical slice

**Choice:** The required logical fixture performs this sequence:

1. Checkpoint a bounded world.
2. Create an attenuated simulation branch.
3. Run deterministic work and capture the successor.
4. Diff and inspect conflicts.
5. Replay exact transitions.
6. Verify closure and detached evidence.
7. Promote through effect-release reservations.
8. Export and import a complete capsule.
9. Plan retention and garbage collection.

An opaque fixture restores and replays one exact ChaosControl cohort without semantic merge.

**Rationale:** A vertical slice catches contract gaps that isolated tests cannot expose.

### Decision: Keep optional profiles explicit

**Choice:** Witnessed heads and executable extents appear as profile capabilities with admitted, blocked, unsupported, or unavailable status.

The workflow cannot silently replace them with weaker local-head or ordinary-artifact behavior when policy requires the stronger profile.

**Rationale:** Optional integration must fail closed instead of creating false parity.

## Rollout

1. Define workflow DTOs and pure planning over prerecorded observations.
2. Add read-only inspect, diff, verify, and replay-plan commands.
3. Add preview modes for every mutation.
4. Add logical apply workflow with deterministic local adapters.
5. Add the exact opaque fixture.
6. Enable witness and executable-extent profiles only after their producer contracts pass.

## Risks / Trade-offs

- One workflow can become a hidden monolith. Keep operation logic in existing cores and restrict this change to composition.
- Many receipts increase operator output. Provide bounded summaries and stable links.
- Partial success is normal. The workflow must preserve the first blocker and completed component receipts.
- A dogfood fixture is bounded evidence. It does not prove all applications, profiles, or hosts.
