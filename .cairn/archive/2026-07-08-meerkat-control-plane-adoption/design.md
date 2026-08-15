# Design: Meerkat-inspired control-plane adoption

## Context

Molten already treats consensus as a narrow control-plane substrate for registries, policy pointers, locks, leases, queues, elections, barriers, and receipt indexes. The current accepted specs name Raft/read-index behavior because that is the implemented path. Meerkat's main lessons are not that Molten should replace Raft immediately, but that the runtime should make algorithm choice, read consistency, placement, and fault assumptions explicit enough to evaluate a leaderless quorum backend later.

External source reviewed: Cloudflare Blog, "Introducing Meerkat: an experiment in global consensus". Treat it as architecture inspiration, not authoritative proof for Molten.

## Decisions

### 1. Keep Raft as the default admitted profile

**Choice:** Add an algorithm-profile field to future group manifests, with the current Raft behavior represented as the default admitted profile.

**Rationale:** This preserves existing Trellis/Raft evidence, read-index requirements, membership admission, and coordination contracts while creating a stable slot for future alternatives. The manifest must prevent accidental semantic drift: an omitted or unknown algorithm profile cannot silently mean leaderless consensus.

### 2. Allow leaderless quorum only as an experimental profile

**Choice:** A Meerkat/QuePaxa-inspired profile may be modeled as an experimental leaderless quorum profile only when explicit policy, proof/model evidence, deterministic simulations, and operator receipts are present.

**Rationale:** Meerkat's attractive property is that any healthy replica can drive progress when a majority is reachable, reducing leader failure sensitivity. Molten should evaluate that property without claiming production readiness or weakening the current Raft-backed path.

### 3. Make read consistency observable in requests and receipts

**Choice:** Control-plane and coordination reads must name whether they require `linearizable` or accept `local-stale` behavior. Linearizable reads need read-index or algorithm-specific quorum evidence. Local-stale reads may support diagnostics and dashboards but cannot satisfy mutation guards, lock ownership, fencing checks, or release gates.

**Rationale:** Meerkat explicitly distinguishes current reads that enter consensus from stale-but-consistent local reads. Molten should adopt that operator-visible distinction because it prevents stale status from being mistaken for authority.

### 4. Add placement receipts before global-control-plane reliance

**Choice:** Consensus group setup should emit placement evidence that names selected members, admitted fault domains, membership evidence, policy refs, and latency/failure diagnostics. Unsafe or undeclared placements should deny before group installation or be marked diagnostic-only.

**Rationale:** Consensus latency is bounded by the reachable majority. Placement is therefore part of the safety/operability story, not merely deployment metadata.

### 5. Prove behavior through deterministic simulation first

**Choice:** The first implementation evidence should be deterministic simulation fixtures for failed or slow leaders, concurrent proposals, majority/minority partitions, stale read attempts, and placement misconfiguration before any live transport claim.

**Rationale:** Molten's evidence model values reproducible receipts. Simulation should catch protocol-state and receipt-shape regressions before VM/live soak tests add nondeterministic transport noise.

## Risks / Trade-offs

- A protocol-neutral manifest can look like support before an implementation exists. Mitigation: unknown or experimental profiles must deny unless every required policy and evidence ref is present.
- Local-stale reads can improve UX but are dangerous if reused as authority. Mitigation: receipts must classify them as non-linearizable and gates must reject them for mutation or release decisions.
- Leaderless quorum protocols are subtle. Mitigation: keep Raft default, require formal/model evidence before production profile admission, and make all current support explicitly experimental.
- Placement diagnostics can become stale. Mitigation: bind placement receipts to membership/config refs and require refresh on membership or policy changes.

## Validation strategy

For this Cairn package:

```sh
nix run path:/home/brittonr/git/OnixResearch/cairn#cairn -- validate --root . --policy cairn-policy/generated/cairn-policy.json
nix run path:/home/brittonr/git/OnixResearch/cairn#cairn -- gate proposal meerkat-control-plane-adoption --root .
nix run path:/home/brittonr/git/OnixResearch/cairn#cairn -- gate design meerkat-control-plane-adoption --root .
nix run path:/home/brittonr/git/OnixResearch/cairn#cairn -- gate tasks meerkat-control-plane-adoption --root .
```

For future implementation, run focused consensus, coordination, and deterministic simulation checks before broader pre-commit validation.
