## Context

The last no-std split removed larger runtime leaks from `aspen-core`, `aspen-storage-types`, `aspen-traits`, and wire crates, but `aspen-cluster-types` still defaulted to `iroh` conversion helpers. That meant a bare dependency could still resolve `iroh-base`, and a workspace dependency entry with `features = ["iroh"]` could silently re-enable runtime helpers for every `workspace = true` consumer.

## Goals / Non-Goals

**Goals**
- Make `aspen-cluster-types` alloc-safe by default.
- Keep `NodeAddress` / `ClusterNode` runtime conversion helpers behind explicit feature opt-in.
- Audit direct consumers so only crates that actually call runtime helper APIs enable `iroh`.
- Save deterministic evidence for the bare/default graph, explicit `iroh` graph, and representative consumers.

**Non-Goals**
- Changing `NodeAddress` data layout or transport semantics.
- Reworking unrelated no-std seams such as tickets or hook tickets in this change.
- Changing transitive runtime crates that do not depend on `aspen-cluster-types` directly.

## Decisions

### 1. `aspen-cluster-types` defaults to alloc-safe mode

**Choice:** keep crate root `#![cfg_attr(not(test), no_std)]`, set `default = []`, and leave `iroh-base` behind the existing `iroh` feature.

**Rationale:** the types crate already compiles alloc-only; the bug is feature topology, not the core type model.

### 2. Runtime crates opt into `iroh` in their own dependency stanzas

**Choice:** remove workspace-level implicit `features = ["iroh"]`, then add explicit `features = ["iroh"]` only in crates that use runtime helper APIs.

**Implementation:** enumerate every direct `aspen-cluster-types` dependency stanza from repo manifests with `rg 'aspen-cluster-types\\s*=\\s*\\{' . -g 'Cargo.toml'`, classify each consumer as `iroh-opt-in` or `alloc-safe`, and tie the classification to concrete helper usage or non-usage using `rg 'NodeAddress::new|ClusterNode::with_iroh_addr|\\.iroh_addr\\(|try_into_iroh\\(' . -g '*.rs'`. Representative compile rails must include every direct consumer changed by the audit, unchanged consumers must still be traced to saved manifest/helper-usage evidence in `direct-consumer-audit.md`, and every `iroh-opt-in` classification must correspond to an explicit consumer manifest stanza after workspace-level feature removal.

**Rationale:** the dependency edge that needs runtime transport helpers should declare that requirement locally. That keeps consumer audits reviewable and prevents `workspace = true` inheritance from masking leaks.

### 3. Verification separates bare/default and explicit-`iroh` proofs

**Choice:** save distinct artifacts for:
- bare/default `aspen-cluster-types` trees and checks
- explicit `--features iroh` trees and checks
- a root-workspace dependency proof showing `Cargo.toml` no longer re-enables `iroh`
- a complete direct-consumer manifest audit
- representative runtime/test consumer compile rails
- `verification.md` mapping each claim to saved artifacts

**Implementation:** save the exact results for `cargo check -p aspen-cluster-types --no-default-features`, `cargo check -p aspen-cluster-types --no-default-features --target wasm32-unknown-unknown`, `cargo test -p aspen-cluster-types`, `cargo test -p aspen-cluster-types --features iroh`, `cargo tree -p aspen-cluster-types -e normal --depth 2`, `cargo tree -p aspen-cluster-types --features iroh -e normal --depth 2`, and `cargo tree -p aspen-traits -e normal --depth 2` under `openspec/changes/cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`. Save the root-workspace stanza proof under `openspec/changes/cluster-types-iroh-opt-in/evidence/workspace-dependency-proof.txt`, and make that proof fail review if the root workspace dependency still enables `features = ["iroh"]`. Save the full direct-consumer inventory, opt-in classification, helper-usage justification, and per-consumer evidence references under `openspec/changes/cluster-types-iroh-opt-in/evidence/direct-consumer-audit.md`. Save representative runtime/test consumer compile rails under `openspec/changes/cluster-types-iroh-opt-in/evidence/runtime-consumers.md`. Keep `openspec/changes/cluster-types-iroh-opt-in/verification.md` synchronized with those artifacts.

**Rationale:** the review failure came from claiming the bare graph was clean while only showing an explicit-`iroh` graph. The fixed evidence must show both modes separately, prove the workspace stanza is alloc-safe, and prove the consumer audit is complete.

## Risks / Trade-offs

- **Missed consumer opt-in** → mitigate with a direct dependency audit plus representative compile rails.
- **Workspace inheritance confusion** → mitigate by making root workspace dependency alloc-safe and requiring local feature stanzas where needed.
