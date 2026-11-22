## Context

Use the live service in `src/fabric_consistency/raft`, whose executor separates planned effects from applied observations. The original archived model pairs message deliveries and their acknowledgements atomically. Aspen #63 and Trace Lab #6 now target live Raft, not ConsensusFastPath; Trace Lab still owns the interchange mapping and validator execution.

## Decisions

### 1. Capture from an opt-in fixture, not a production switch

Run one explicitly selected, fixed three-voter scenario in the existing loopback live-cluster fixture. Capture only successfully executed inputs and actual applied state and correlated request/outcome facts at the fixture's serialized boundaries. Preserve rejected and retryable reads without manufacturing a `Current` outcome. A send is not a delivery and a planned `ProposalOutcome` is not an observed acknowledgement. Bind ordered facts to the group, requests, term and distinct replicas. Enforce the selected finite step and byte limits; no credentials, payloads, or host paths belong in the output.

### 2. Check an explicit post-election read slice

Begin at an observed checkpoint after a leader change, carrying the earlier independently observed write acknowledgement and new leader log/commit facts. Record the baseline's refused read before the new leader is ready and a later completed quorum read after the committed prefix covers the earlier acknowledgement. The model checks read-index and delivery facts only within this window. It must admit the real committed-log exception and must not claim to check the election or abstract network used by the archived model. Never reconstruct intermediate states, acknowledgements, or missing state assignments from a status hash.

[`RaftReadIndexWindow.tla`](../../../verification/tla/RaftReadIndexWindow.tla) takes five states from the following six ordered capture rows, with no invented intermediate transitions:

| Captured row | Modeled boundary |
| --- | --- |
| `earlier-write-ack` | Prelude; the typed `Committed` result supplies `priorAcknowledged` for all model states. |
| `leader-checkpoint` | `Init` with the observed new leader and retained log/commit/applied fields. |
| `read-retry` | `ReadRetry`; preserve the replica facts and record the outcome index. |
| `new-commit` | `ObserveCommit` from the real applied transition; the intervening append and durable effects are not separate model states. |
| `read-started` | `ReadStart`; `requiredIndex` and `readAcks` are the observed pending fields. |
| `quorum-read-current` | `ReadComplete`; join the matching delivered ACK with the prior acknowledgers and use the typed `Current` index. |

The model aliases only the three roster IDs fixed by the fixture to `A`, `B`, and `C`. Empty acknowledgements and the zero read index at `Init` represent no pending read, not unknown data. `ReadLinearizable` checks the required prefix at a completed read, not the application value. Changed term, snapshot/compaction, unobserved acknowledgements, missing state, and active rejoin are outside this projection.

### 3. Separate local observation from independent checking

Use the exact Dive source `0fc00914b7cd93d88e97e5d7d66aac805c06228b` (Trace Lab #6, report schema 4). Keep the baseline model and config constant across the positive scenario and any negative control. A control with deliberately stale facts may exercise the local observer but does not substitute for a reached independent negative verdict. If the genuine rows do not map completely, retain `unsupported`, with the first unmappable state/action and absent fact. Do not modify or make configurable the production read guard to manufacture a control.

For an implementation negative, use a disposable source snapshot with a test-only omission of the current-term read guard. Save the exact diff and select only the four-row guard-control arm: earlier acknowledged write, initial post-election checkpoint, early pending read, and the observed quorum result. The versioned control is separate from the clean run; the default and published Aspen code never select or provide the bypass. Do not claim detection if the control fails to reach a real stale `Current` result.

## Risks / Trade-offs

- This covers local exercised behavior, not full Raft, durable restart, liveness, arbitrary schedules, or production qualification.
- Trace Lab #6 and Octet #34 are still opened prerequisites for the standard driver and checked reusable binary. A local build or a model-only check is not an implementation trace verdict.
