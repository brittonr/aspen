## Context

`docs/crate-extraction/transport-rpc.md` already defines the intended split: `aspen-transport` owns Iroh protocol helpers and adapter features; `aspen-rpc-core` owns dispatch traits and runtime context behind opt-in features. The current blocker is evidence, not a known API redesign.

## Goals / Non-Goals

**Goals:** create durable downstream, negative, compatibility, and checker evidence; fix only concrete feature leaks found by those checks; update readiness docs if evidence passes.

**Non-Goals:** publish crates, split repositories, remove runtime compatibility bundles, or redesign RPC protocol schemas.

## Decisions

### 1. Evidence-first implementation

**Choice:** Start with fixtures and graph checks before editing code.
**Rationale:** The manifest suggests the split may already be close; code changes should be driven by observed leaks.
**Alternative:** Refactor runtime context preemptively. Rejected because it risks churn without proof.

### 2. Runtime features stay explicit

**Choice:** Keep runtime compatibility behind named features and prove consumers enable them intentionally.
**Rationale:** Prevents reusable defaults from silently inheriting the app graph while preserving Aspen node behavior.

## Risks / Trade-offs

**Feature unification false positives** → Use downstream fixtures and direct `cargo tree -p` evidence separately so workspace feature unification does not hide default leaks.
