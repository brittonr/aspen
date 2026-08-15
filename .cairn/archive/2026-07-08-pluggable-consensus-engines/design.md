# Design: Pluggable consensus engines

## Context

Molten control-plane manifests now declare an algorithm profile, read consistency support, quorum rule, placement ref, and required evidence refs. That gives operators explicit semantics, but runtime construction still treats Raft as the only executable production path and hard-codes profile validation in Raft-oriented modules.

This change makes the profile field operational without weakening the current safety boundary. "Pluggable" means a manifest can resolve to a registered engine implementation through a stable interface. "Swappable" means an existing group can move between admitted engines only through an explicit, receipt-backed transition plan. It does not mean unchecked dynamic loading, silent fallback, or live replacement without fencing.

## Decisions

### Engine registry is policy-backed and fail-closed

**Choice:** Add a consensus engine registry that maps `(algorithm_profile, profile_version)` to an implementation descriptor, capability set, production-admission status, evidence requirements, and conformance receipt refs.

**Rationale:** Profiles should be data-driven enough to add future engines without editing every caller, but admission must remain explicit. Unknown, disabled, experimental, or evidence-incomplete entries deny before runtime construction.

### Engine interface exposes common receipt semantics

**Choice:** Define a `ControlPlaneConsensusEngine` boundary for control-plane proposals, linearizable reads, local-stale reads, snapshots, recovery, membership/config transitions, and readback summaries. The core of each engine should be pure over canonical inputs and return deterministic decisions/receipts. The shell should own storage, network, timers, host effects, and CLI/operator I/O.

**Rationale:** Coordination and control-plane applications need common evidence classes, not Raft-specific internals. A pure core makes conformance tests possible without standing up transports or clocks.

### Raft remains the only admitted production engine initially

**Choice:** The initial implementation should adapt the current Raft-backed path behind the registry/interface. The leaderless profile remains experimental and diagnostic until it has separate accepted implementation, proof/model evidence, deterministic simulation coverage, placement evidence, membership evidence, and policy admission.

**Rationale:** This creates a real extension point while preserving all existing production claims. Pluggability should not accidentally promote an experimental algorithm.

### Swaps are explicit transitions, not implicit config edits

**Choice:** Changing a group from one engine/profile to another must require a canonical switchover plan and receipt. The plan should bind source profile, target profile, source state ref, target bootstrap state ref, membership/config refs, fencing epoch, replay/conformance evidence, currentness evidence, rollback posture, operator approval refs, and diagnostics.

**Rationale:** Consensus algorithm changes are semantic migrations. A manifest edit alone is not enough to prove that already-committed control-plane state remains safe, current, and replayable.

### Coordination consumes abstract currentness evidence

**Choice:** Coordination primitives should depend on common commit/read/fencing/currentness receipt fields and should not inspect Raft-specific leader, term, index, or read-index internals except through the normalized engine receipt.

**Rationale:** Locks, queues, elections, barriers, rate limits, and registry pointers should continue to work across admitted engines when they have equivalent currentness and fencing evidence.

### Conformance fixtures gate every engine

**Choice:** Every registered engine should have deterministic conformance fixtures for admitted proposal, duplicate operation denial, linearizable read freshness, local-stale classification, snapshot/recovery, membership/config denial, and canonical state-machine replay. Switchover fixtures should cover both successful Raft-to-target migration in simulation and negative paths.

**Rationale:** A common interface is only useful if implementations prove the same application-level semantics and fail safely for unsupported or unsafe transitions.

## Risks / Trade-offs

- A registry can look like runtime plugin support before multiple engines are actually production-admitted. Mitigation: readback must show engine status, and only Raft starts as admitted production.
- Engine traits can hide important algorithm-specific evidence. Mitigation: normalized receipts carry common fields while preserving engine-specific evidence refs as opaque, reviewable attachments.
- Switchover receipts may be verbose. Mitigation: this is appropriate because swaps are rare, high-risk control-plane operations.
- Dynamic plugin loading could expand the attack surface. Mitigation: this change scopes pluggability to registered in-process engines and policy-admitted descriptors; dynamic loading would require a separate Cairn change.

## Validation strategy

For this Cairn package:

```sh
nix run path:${ONIX_RESEARCH_ROOT:-$HOME/git/OnixResearch}/cairn#cairn -- validate --root . --policy ${ONIX_RESEARCH_ROOT:-$HOME/git/OnixResearch}/cairn/cairn-policy/generated/cairn-policy.json
nix run path:${ONIX_RESEARCH_ROOT:-$HOME/git/OnixResearch}/cairn#cairn -- gate proposal pluggable-consensus-engines --root . --policy ${ONIX_RESEARCH_ROOT:-$HOME/git/OnixResearch}/cairn/cairn-policy/generated/cairn-policy.json
nix run path:${ONIX_RESEARCH_ROOT:-$HOME/git/OnixResearch}/cairn#cairn -- gate design pluggable-consensus-engines --root . --policy ${ONIX_RESEARCH_ROOT:-$HOME/git/OnixResearch}/cairn/cairn-policy/generated/cairn-policy.json
nix run path:${ONIX_RESEARCH_ROOT:-$HOME/git/OnixResearch}/cairn#cairn -- gate tasks pluggable-consensus-engines --root . --policy ${ONIX_RESEARCH_ROOT:-$HOME/git/OnixResearch}/cairn/cairn-policy/generated/cairn-policy.json
```

For future implementation, run focused consensus/coordination tests, deterministic conformance and switchover fixture tests, `cargo test --lib`, `cargo fmt --check`, `pre-commit run --all-files`, Cairn validation/gates, sync, and archive validation before commit.
