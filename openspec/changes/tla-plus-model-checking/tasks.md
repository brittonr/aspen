## 1. Infrastructure

- [ ] 1.1 Add TLA+ tooling to Nix flake: package `tla2tools.jar` (TLC model checker) as a Nix derivation with JRE
- [ ] 1.2 Create `nix run .#check-tla` app that runs TLC on all `.tla` specs in `tla/`
- [ ] 1.3 Add TLA+ model checking to `nix flake check` outputs
- [ ] 1.4 Create `tla/` directory at repo root with `.gitignore` for TLC output files (states/, *.dump)
- [ ] 1.5 Add a `tla/README.md` documenting conventions: variable naming, module structure, how to run locally

## 2. Trust Init Spec

- [ ] 2.1 Write `tla/trust_init.tla`: model cluster secret creation with N nodes and threshold K
- [ ] 2.2 Define state variables: `nodes` (per-node state), `leader` (coordinator), `msgs` (global message set), `shares` (per-node shares)
- [ ] 2.3 Define actions: `LeaderCreateSecret`, `LeaderDistributeShares`, `NodeReceiveShare`, `NodeAckShare`
- [ ] 2.4 Define safety invariant `TypeOK` (all variables within expected domains)
- [ ] 2.5 Define safety invariant `AllNodesHaveSharesAfterInit`: when init completes, every member has exactly one share
- [ ] 2.6 Write `tla/trust_init.cfg` for TLC: 3 nodes, K=2
- [ ] 2.7 Verify spec passes TLC with zero errors

## 3. Reconfiguration Spec

- [ ] 3.1 Write `tla/trust_reconfig.tla`: model reconfiguration with share collection, secret rotation, and encrypted chain
- [ ] 3.2 Define coordinator states: `Idle`, `CollectingOldShares`, `Preparing`, `WaitingForAcks`
- [ ] 3.3 Define node states: `Idle`, `Prepared`, `Committed`, `Expunged`
- [ ] 3.4 Model crash/restart: `CrashNode` action removes node from active set; `RestartNode` action restores from persistent state
- [ ] 3.5 Define safety invariant `CommittedNodesHaveShares`: committed node always has a share for its committed epoch
- [ ] 3.6 Define safety invariant `EpochMonotonicity`: no node commits an epoch less than its previously committed epoch
- [ ] 3.7 Define safety invariant `ConfigurationConsistency`: same epoch implies same config across nodes
- [ ] 3.8 Define safety invariant `ExpungementPermanence`: once expunged, always expunged
- [ ] 3.9 Define liveness property `ReconfigurationCompletes`: under fairness, all new members eventually commit
- [ ] 3.10 Write `tla/trust_reconfig.cfg`: 3 nodes, 2 epochs, max 2 crashes per node
- [ ] 3.11 Verify spec passes TLC

## 4. Expungement Spec

- [ ] 4.1 Write `tla/trust_expunge.tla` (or extend trust_reconfig): model Expunged message delivery and peer enforcement
- [ ] 4.2 Define actions: `SendExpunged`, `RecvExpunged`, `PeerEnforceExpunge` (respond with Expunged to non-member)
- [ ] 4.3 Define invariant: expunged nodes never respond to GetShare
- [ ] 4.4 Define liveness: removed nodes are eventually notified (directly or via peer enforcement)
- [ ] 4.5 Verify spec passes TLC

## 5. Combined Spec and State-Space Reduction

- [ ] 5.1 Write `tla/trust_combined.tla` composing init, reconfig, and expungement
- [ ] 5.2 Apply state-space reduction: use CHOOSE for message receipt (not \E), fixed configuration sequences, crash limits
- [ ] 5.3 Write `tla/trust_combined.cfg`: 3 nodes, 2 reconfigurations, max 1 crash per node
- [ ] 5.4 Verify combined spec passes TLC within CI time budget (< 5 minutes)
- [ ] 5.5 Create a "deep check" config with 5 nodes and 3 epochs for periodic runs

## 6. Documentation and Spec-Code Mapping

- [ ] 6.1 Write `docs/tla-specs.md`: overview of specs, how to read them, how invariants map to Rust code
- [ ] 6.2 Add comments in Rust trust protocol code referencing specific TLA+ invariants (e.g., `// TLA+ invariant: CommittedNodesHaveShares`)
- [ ] 6.3 Add a spec-to-code mapping table in `tla/README.md`: TLA+ variable → Rust struct field, TLA+ action → Rust method
- [ ] 6.4 Add TLA+ section to `AGENTS.md` documenting how to run, modify, and extend specs
