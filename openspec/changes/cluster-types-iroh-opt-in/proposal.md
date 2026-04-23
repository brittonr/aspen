## Why

`aspen-cluster-types` sits on the alloc-only foundation path, but its manifest still defaulted to `iroh` helpers. That let bare or `workspace = true` consumers re-enable `iroh-base` implicitly, which hid no-std boundary regressions and contradicted the intended explicit opt-in model.

## What Changes

- Make `crates/aspen-cluster-types` alloc-safe by default with `default = []`.
- Keep runtime conversion helpers behind an explicit `iroh` feature only.
- Remove workspace-level implicit `iroh` enablement for `aspen-cluster-types` and save a reviewable proof artifact for the root `Cargo.toml` stanza.
- Audit every direct `aspen-cluster-types` dependency stanza so only crates that call `NodeAddress::new`, `ClusterNode::with_iroh_addr`, `.iroh_addr()`, or `try_into_iroh()` opt into `features = ["iroh"]`.
- Save reviewable dependency-tree and consumer-audit evidence for the bare/default graph, the explicit `iroh` graph, an alloc-safe workspace-consumer path, and the full direct-consumer inventory.

## Non-Goals

- No changes to `NodeAddress` serialization or transport semantics.
- No ticket or hook-ticket transport-neutral cleanup in this change.
- No broad rework of unrelated no-std seams beyond direct `aspen-cluster-types` consumers.

## Capabilities

### Modified Capabilities
- `architecture-modularity`: tighten the leaf-crate contract so `aspen-cluster-types` stays alloc-safe by default and runtime transport helpers require explicit per-crate opt-in.

## Impact

- **Files**: `crates/aspen-cluster-types/`, direct consumer `Cargo.toml` files, `Cargo.toml`, and OpenSpec artifacts/evidence for this seam.
- **APIs**: runtime `iroh` helper methods stay available, but only when the consuming crate opts into the `iroh` feature explicitly.
- **Dependencies**: bare/default `aspen-cluster-types` no longer pulls `iroh-base`.
- **Testing**: verify bare/default and explicit-`iroh` trees separately, prove at least one alloc-safe workspace-consumer path stays clean, and map every direct consumer in the audit to saved evidence or a deterministic validation rule.
