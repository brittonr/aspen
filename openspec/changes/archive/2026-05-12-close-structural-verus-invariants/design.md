## Context

The current remaining structural markers are concentrated in coordination queue/registry/worker/fencing/strategy specs, core directory/index specs, and commit-dag diff specs. Prior drain slices showed that successful proofs usually require local witnesses, explicit arithmetic branches, or stronger preconditions matching the post-state model.

## Goals / Non-Goals

**Goals:**
- Remove structural `external_body` markers with small helper lemmas.
- Preserve runtime behavior unless a spec is demonstrably inconsistent with existing executable saturation or update semantics.
- Keep each proof slice independently verifiable and commit-sized.

**Non-Goals:**
- Proving cryptographic collision resistance or encoding library correctness.
- Broad model rewrites unrelated to the listed markers.
- Changing public APIs.

## Decisions

### 1. Drain by model family

**Choice:** Work in narrow slices: queue FIFO, registry/directory/index Map/Set updates, worker invariants, strategies fairness, fencing arithmetic, and diff order/validity.

**Rationale:** Each family needs different helper lemmas; mixing them hides root causes and makes reverts expensive.

**Alternative:** One large proof pass. Rejected because previous direct sweeps showed the remaining markers require deliberate lemmas.

### 2. Require evidence per touched root

**Choice:** Every completed slice records the Verus root command and focused Rust tests if runtime helpers are touched.

**Rationale:** Structural proof changes can silently diverge from runtime semantics without focused evidence.

## Risks / Trade-offs

**Map/Set extensionality friction** → Add local helper lemmas and strengthen preconditions instead of using trusted wrappers.

**FIFO proof breadth** → Split FIFO and count/invariant facts; reuse queue insertion witness patterns from prior queue ack/dequeue/enqueue slices.

**Spec/runtime mismatch** → Prefer aligning the spec with existing executable behavior and add boundary tests when runtime helper semantics are clarified.
