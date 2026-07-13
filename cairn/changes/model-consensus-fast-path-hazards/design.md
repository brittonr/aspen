## Context

The accepted consensus architecture already provides canonical command envelopes, deterministic application state machines, client-session idempotency, snapshots, recovery receipts, pluggable engine profiles, and fail-closed production admission. The active `fabric-consistency-service-runtime` change correctly reclassifies the in-process Raft implementation as model-only and is blocked on a reusable admitted cross-process Iroh listener/session shell.

Jetpack is useful here as a pinned design and adversarial-test reference, not as code to import. Its central composition hazard is that a fast-path acknowledgement constrains the original path, but the proposer that made that promise may disappear after an election. Safety therefore depends on both recoverability of acknowledged commands and ordering recovered commands before conflicting work in the new view. These obligations are not discharged by proving base Raft alone.

Reference cohort:

- Paper: `Jetpack: Consensus Made Generally Fast`, OSDI 2026.
- Artifact: `stonysystems/jetpack` commit `c03e318ec355b11edd42aac56c68d0765f88d1d2`, MIT licensed.
- Artifact TLA+ results and benchmark reports are external evidence inputs only; they are not Molten proof or performance evidence.

## Decisions

### 1. Model the mechanism generically and pin Jetpack only as a reference

**Choice:** Add a generic crash-fault fast-path composition model with an explicit reference-source identity. Independently express Molten transitions and fixtures instead of importing the C++ runtime or treating external TLA+ specifications as authoritative.

**Rationale:** The useful result is a reusable safety contract for composed consensus profiles. Binding the source identity preserves provenance while keeping Molten's canonical models, claims, and implementation boundaries independent.

### 2. Keep model admission separate from live engine admission

**Choice:** Register the profile as pure-model or deterministic-simulation only. It may run over the existing in-process base model, but it cannot satisfy live or production engine selection and cannot unblock `fabric-consistency-service-runtime`.

**Rationale:** Same-process transitions can find logical counterexamples but cannot demonstrate process isolation, real transport, durable recovery, timing behavior, or operational readiness.

### 3. Make conflict classification a pure extension-owned safety contract

**Choice:** A versioned conflict-contract artifact binds command schemas and defines a deterministic, side-effect-free predicate over canonical commands. It must report conflict whenever order can affect application state or either command response. Unknown schemas, aliases, predicates, ranges, preconditions, analysis failures, or unsupported operations conservatively conflict and use the original path.

**Rationale:** False positives cost latency but preserve safety; a false negative can violate linearizability. Conflict semantics belong to the extension state machine, not generic node core or transport code.

### 4. Bind both paths to one canonical operation

**Choice:** The fast and original paths carry the same canonical command ref, client-session identity, sequence, policy/authority/resource cohort, group, generation, and engine epoch. A successful fast acknowledgement is a client-visible commit decision in the model, while the original path remains responsible for eventual canonical ordering and application. Convergence applies the operation at most once.

**Rationale:** Separate command identities or separate application effects would turn optimization into two operations and make fallback, replay, and receipt validation ambiguous.

### 5. Require same-view acknowledgements and every active proposer promise

**Choice:** Each fast acknowledgement binds both the acceleration view and the base-engine view. A fast commit requires one same-view acceleration superquorum plus promises from every active original-path proposer in that view. Acknowledgements from different views never combine. The base-model compatibility contract must also establish that conflicting commands proposed by one proposer preserve proposal order in log/execution order and that proposer receive order preserves proposal order. If buffering can reorder receipt and proposal, acknowledgement must wait for equivalent proposal-order evidence; if execution can reorder conflicting proposals, the transparent fast-path profile is incompatible.

**Rationale:** Quorum intersection is not sufficient when acknowledgements straddle views, a proposer outside the acknowledgement set can order conflicting work first, or the base model reorders commands after the acceleration layer has promised their order.

### 6. Recover before admitting new-view work

**Choice:** The acceleration layer tracks its own view. On a base-engine view change it pauses fast admission, agrees on the prior normal view's recovery set, commits that set—or an explicit no-op recovery marker—through the original path, and only then admits commands in the new normal view. Interrupted or cascading recoveries target the last normal view and carry accepted recovery state forward.

**Rationale:** Recoverability without priority still permits stale conflicting entries to precede a command already acknowledged to a client. The marker provides an explicit ordering and evidence boundary.

### 7. Model both three-replica and five-replica envelopes

**Choice:** The bounded corpus includes named three-replica and five-replica profiles with derived majority, superquorum, proposer, and failure bounds. The three-replica profile must make visible that its fast-path superquorum contains every replica and therefore becomes unavailable after any replica loss even while the original majority path may progress.

**Rationale:** Small control-plane deployments are likely to use three replicas; hiding that availability trade-off would make the model operationally misleading.

### 8. Export counterexamples, not transferred proofs

**Choice:** Emit canonical profile, run, trace, invariant, coverage, recovery, divergence, and minimized-counterexample artifacts. Readback names the pinned external reference and exact claim profile. External model-checking success may be linked as supporting provenance but never substitutes for Molten model execution.

**Rationale:** Reproducible negative traces are immediately useful to Molten simulation and ChaosControl while keeping proof and authority boundaries honest.

## Functional core / imperative shell split

- Pure core: profile validation, quorum derivation, conflict classification, stable-view transitions, dual-path convergence, recovery-set selection, marker ordering, invariant evaluation, replay comparison, counterexample reduction, and claim classification.
- Shell: load pinned reference metadata, materialize typed profiles, execute bounded exploration, persist artifacts, export repro bundles, and render operator summaries.

## Dependencies

- Existing canonical command, client-session, consensus profile, and in-process model boundaries.
- The active `fabric-whole-system-simulation` change may later consume this model and corpus, but this change does not depend on live adapters or its completion.
- The future `add-consensus-fast-path-acceleration` change depends on this package being completed and archived.

## Risks / Trade-offs

- An independently expressed model can reproduce the same misunderstanding as an implementation. Compare named scenarios and invariants against the pinned paper/artifact while preserving independent source and claim boundaries.
- State-space growth can hide important schedules. Keep explicit node, command, key, view, recovery, and step bounds; report unexplored alternatives rather than claiming completeness.
- A conflict predicate cannot establish arbitrary application semantics by inspection alone. Require extension-owned semantic fixtures and treat incomplete coverage as a production blocker.
- Model success may be overread as live readiness. Keep the profile denied for live and production use and include that denial in positive and negative validation.
