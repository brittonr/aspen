## Context

The repository already carries a TLC and TLA-ts rail for the consensus fast-path hazard model (`nix/consensus-fastpath-model.nix`). The audit-finding models follow its conventions: TLC 1.7.4 from nixpkgs with a version assertion, the unchanged TLA-ts publication, one configuration per scenario, and retained raw output.

## Decisions

### 1. Expectations are data, not code

`expectations.tsv` names each configuration, its module, the required outcome (`pass`, `invariant:<Name>` or `temporal`) and the issue. The runner compares actual outcomes against it. Current-code configurations are expected to fail, so the rail stays green while the issues are open and turns red if the modelled behaviour changes without the table changing.

### 2. Witness invariants prove passing configurations are not vacuous

`RaftReadIndex` adds `NoPostElectionRead`, which is expected to be violated. Its violation shows that the passing guard-only and fixed configurations reach a read that must observe a write acknowledged in an earlier term. A first fixed configuration at `MaxLogLen = 2` never reached that read and was replaced by `MaxLogLen = 3, MaxTerm = 2`.

### 3. Fix guidance for #44

`RaftReadIndex_NoopOnly` violates `ReadLinearizable`; `RaftReadIndex_GuardOnly` passes. The read guard (complete a ReadIndex read only after the leader has committed an entry in its current term) is the safety fix. A leader no-op is an availability addition so the guard can be satisfied without client writes.

### 4. Source fidelity

Each module header maps actions to source paths at `8ae93a5e5`. `WorldReleaseAttempt` follows the store transition table in `src/world_promotion/store/outbox.rs`, which has no edge from `Uncertain` back to `Claimed`, so retry is not modelled. `KeyFileCrash` lets background writeback persist any pending data or metadata independently, as on ext4 and xfs without data journaling. The counter models for #56, #57 and #59 restate the code; TLC makes their liveness claims precise.

### 5. Checker parity

TLC and TLA-ts run the same source and configuration. Verdicts and violated invariant names must match. Distinct-state counts are compared only for exhaustive passes; runs that stop at the first violation produce prefix counts that legitimately differ.

## Risks

- Bounds are small (three nodes, three terms, short logs). Larger bounds may expose behaviours these runs do not reach.
- The rail adds about seven minutes to `nix flake check`, dominated by `RaftReadIndex_GuardOnly` (4,319,902 distinct states).
