## Context

Aspen has two in-tree consensus paths. `fabric_consistency::raft` replicates a log across peers, takes a leader read index and backs the reliability and collaboration-scope services. `raft_control_plane` keeps one `ControlRegistryRuntime` in one process; `CoordinationRuntime.raft` is such a runtime, reached from `molten test coordination`, `molten raft run-fixture` and the job-schedule workflow. Neither calls Trellis; Trellis is used by choreography projection and the `job_dag` helpers. The registry's receipts nevertheless claimed Trellis evaluation, quorum commits and linearizable reads.

## Decisions

### Decision: Name registry predicates for what the process computes

**Choice:** Predicates are `registry-append-consistency` (prior-log binding), `registry-local-commit` (log-entry binding), `registry-commit-advancement` (monotonic index), `registry-committed-read` (committed-state binding) and `registry-snapshot-restore` (snapshot content ref). Every receipt carries `in-process-registry` as its evaluator check, which the parser requires in place of `trellis-predicate`. The commit predicate's `quorum-members` check, the commit receipt's `quorum-commit` check and its `quorum` member list are removed; the commit receipt check is `registry-commit`.

**Rationale:** No code evaluates a Trellis predicate or collects acknowledgements from the listed members, so those labels asserted properties that were never computed. Removing the member list rather than renaming it avoids implying the members took part.

### Decision: Version every family whose bytes change; keep no legacy decoder

**Choice:** `raft-predicate-receipt`, `raft-read-receipt` (field `read-index-predicate` becomes `committed-read-predicate`), `raft-commit-receipt`, `coordination-receipt`, `coordination-status-assertion` and `chain-checkpoint` move to v2 in both record label and schema string, and the ledger artifact-kind table follows. Parsers accept only v2.

**Rationale:** The same kind name with different meaning must not share an identity. No fixture, example, configuration or committed ledger reads these v1 records back, so a decode-only legacy path would have no caller.

### Decision: The in-process registry serves `local-committed` and denies `linearizable`

**Choice:** `ControlRegistryReadInput` borrows the `ControlRegistryRuntime` and drops the caller-supplied state, term, index and `read_index`; the receipt binds that runtime's committed state, term, index and group manifest. `local-committed` passes with a `registry-committed-read` predicate; `local-stale` stays non-authoritative with no predicate; `linearizable` is denied with `IN_PROCESS_REGISTRY_LINEARIZABLE_UNSUPPORTED`.

**Rationale:** With one process there is no replicated read index to take. Deriving the bound state from the runtime removes the vacuous input that made the stale-read check unreachable and prevents a caller from stamping a state the registry never committed.

### Decision: Coordination admits protected operations only with `local-committed`

**Choice:** Coordination admission denies `linearizable` for every operation and keeps denying `local-stale` for protected operations. Coordination, job-schedule and `raft run-fixture` defaults use `local-committed`. The read receipt's `read-index-bound` check becomes `committed-state-bound`, passing only for an admitted `local-committed` read; status assertions pass `committed-state-bound` for `local-committed`.

**Rationale:** Coordination runs on the in-process registry, so `local-committed` is the strongest truthful mode it can serve; denying `linearizable` up front gives one diagnostic for reads and mutations alike.

### Decision: Engine descriptors declare implemented modes, not the algorithm's ideal contract

**Choice:** The in-process registry descriptor supports `local-committed` and `local-stale`, advertises `local-committed-read` instead of `linearizable-read`, and names only `local-committed-state` currentness. Runtime construction and model switchover request that local capability. Descriptor validation requires a nonempty supported-mode set; the separate consensus algorithm-profile validator retains its linearizable-read requirement. Group manifest algorithm contracts remain unchanged.

**Rationale:** A model-only caveat cannot make a nonexistent read-index implementation a supported engine capability. Descriptor/admission identities change with these declared values; they keep their existing record shapes and versions. The existing registry-admission control now admits local-committed mode and denies linearizable mode/capability explicitly. The multinode fixture expects the current v2 commit-receipt family.

### Decision: Checkpoint control-plane mode is `local-checkpoint`

**Choice:** Chain checkpoints record `local-checkpoint`, the parser rejects any other mode, including `trellis-raft`.

**Rationale:** A checkpoint is accepted into the local ledger, not proposed through either consensus path.

## Risks / Trade-offs

- Stored v1 receipts or checkpoints in operator ledgers no longer parse; producing fresh v2 evidence is required.
- `molten.consensus.trellis_raft_core` stays in the inherited Tracey debt baseline: covering it would make that reviewed baseline under `evidence/` stale, and regenerating the baseline is left to integration.
- Chain-link `trellis` evidence fields and the chain predicate labels are not registry or Raft paths and are outside this change.
