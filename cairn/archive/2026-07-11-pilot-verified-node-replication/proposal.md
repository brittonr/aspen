# Proposal: Pilot verified node replication

## Summary

Run a bounded compatibility and claim-boundary pilot for `verus-lang/verified-node-replication` before considering it for Molten's local multicore data structures. Pin the upstream source and its Verus submodule, probe the reviewed Octet verifier, audit trusted proof boundaries, and emit a deterministic pilot decision.

## Motivation

Verified node replication could fit read-heavy local NUMA structures, but it is not distributed replication or consensus. Its repository pins a 2024 Verus snapshot and exposes trusted top-level refinement theorems and traits. Direct dependency adoption without a compatibility probe would conflate upstream proof claims with Molten runtime guarantees.

## Scope

- Pin the verified-node-replication source revision, source hash, Verus submodule revision, and license.
- Add a typed Nickel pilot profile with local-only scope, resource bounds, promotion criteria, and non-claims.
- Add positive and negative profile fixtures.
- Run the current Octet production Verus against the pinned source with required feature flags.
- Record pass, fail, or blocked compatibility plus bounded diagnostics.
- Audit trusted proof markers and prevent runtime dependency promotion while blockers remain.

## Non-goals

- Do not add node replication to Molten's runtime dependency graph in this change.
- Do not call it network replication, distributed consistency, or consensus.
- Do not claim upstream trusted theorems are independently discharged by Molten.
- Do not benchmark production throughput until verifier compatibility and API admission pass.

## Validation

Profile positive/negative fixtures, source sentinel and license checks, exact verifier compatibility probe, trusted-boundary audit, Cairn validation, and documentation review.
