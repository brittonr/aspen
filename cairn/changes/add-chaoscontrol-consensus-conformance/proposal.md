## Why

Molten has canonical consensus commands, deterministic state-machine rules, Hegel properties, recovery tests, and multi-process fixtures. These checks do not provide a bounded external KVM fault campaign over the production-shaped consensus path.

ChaosControl is adding an implementation-neutral SMR chain workload. Molten needs a consumer-owned adapter and evidence boundary for that workload.

The integration must exercise Molten code and preserve Molten authority, policy, resource, durability, and release boundaries. A generic harness cannot own those claims.

## What Changes

- Add a lossless Molten SMR chain observer over committed control-plane state-machine transitions and canonical application-state refs.
- Preserve stable operation identity across acknowledgement, definite rejection, timeout, retry, and recovery paths.
- Package a Rust-only, production-shaped multi-node guest profile for the versioned ChaosControl workload contract.
- Add bounded no-fault, network, process, restart, quorum-loss, recovery, and snapshot catch-up campaigns.
- Import ChaosControl workload receipts as external supporting evidence through fail-closed canonical validation.
- Align invariant names and workload inputs with whole-system simulation while keeping simulation and VM evidence classes separate.
- Add explicit non-claims for consensus universality, Byzantine tolerance, production SLOs, authority, security, and release eligibility.

## Impact

- **Files**: consensus observation and harness code, cluster fixtures, guest packaging, Nix integration, evidence parsers, docs, and consensus/testing-harness Cairn specs.
- **Testing**: pure chain projection, operation identity, no-fault convergence, divergence fixture, partitions, quorum loss, crash/restart, snapshot catch-up, replay, malformed evidence, and claim-promotion denial.
- **Dependencies**: implementation depends on an archived versioned ChaosControl `add-smr-chain-workload` contract and immutable package identity.
- **Portability**: integration must use immutable source and artifact refs. Workspace-relative sibling paths remain development conveniences only.
- **Claims**: passing evidence covers only the exact Molten, ChaosControl, guest, profile, fault, and replay cohort.
