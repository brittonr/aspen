# Aspen consensus fast-path model checks

`verification/tla/ConsensusFastPath.tla` is the authoritative **finite design
model** of the **blocked** fast-path acceleration package (the `no_std`
[`molten-fastpath-core`](../crates/molten-fastpath-core/src/lib.rs) crate,
re-exported at `fabric_consistency::fastpath`), not a proof of Aspen's implementation
and not production Raft verification. The fast path is not wired into
`fabric_consistency::raft` or any live service; production Raft has its own
checked model, described in [production Raft model](raft-tla.md). The internal
Rust crate/CLI and canonical requirement identifiers retain the name `molten`.

There are two instances. The main one is exactly **three replicas, two
canonical command tokens, and three views** with the quorums
`derive_quorums(3)` derives (majority 2, superquorum 3); `QuorumsDerived =
TRUE` makes the model's `ASSUME` enforce the derived rules. Its superquorum is
every replica, so recovery-quorum selection is **not** load-bearing there. The
second instance is an explicitly **non-derived quorum-intersection exercise**:
**four replicas, a three-replica superquorum and recovery quorum, two commands
and two views** (one view change, no cascading recovery). `derive_quorums(4)`
gives superquorum 4, which would again make selection trivial; the derived
five-replica instance (superquorum 4, majority 3) has 3,710,344 distinct
safety-only states at two views, so with the temporal property it cannot meet
the 180-second per-checker limit. The four-replica exercise satisfies
`QuorumSize + 2 * SuperQuorumSize > 2 * N` (9 > 8), so recovery selects a
unique command, and its two-replica recovery-quorum mutant (8 = 8) must fail.
The larger Nickel fault-corpus profiles and the five-replica profile are not
these instances. No symmetry reduction, state constraint, depth cutoff, or
unchecked invariant is used to make parity fit the runtime budget.
`CHECK_DEADLOCK FALSE` permits quiescence; it does not discharge progress.

## Distributed state and quorum intersection

The 2026-11-26 rewrite replaced the earlier centralized model, whose guards
(`Pending(c) = {}`, a global commit order, `CONSTANT Quorum` never referenced)
restated its own invariants. The model now carries:

- **Per-replica state.** Every replica has its own acceptance log (`rlog`),
  view and normal/paused status. Client requests are broadcast and the network
  reorders them, so replicas accept commands in different orders.
- **Real superquorums.** `FastCommit` counts the replicas that acknowledged a
  command behind the same prefix in the current stable view and requires at
  least `SuperQuorumSize` of them, one of which must be the order-preserving
  active proposer (the compatible promise; see the source discrepancy below).
  A replica acknowledges only in its own current view, so acknowledgements from
  an earlier view never count.
- **Quorum-limited recovery.** `AgreeRecoverySet` sees only the logs of a
  recovery quorum of at least `QuorumSize` replicas that joined the new view.
  It keeps the durable original-engine order plus the longest extension held
  by `RecoveryThreshold == QuorumSize + SuperQuorumSize - N` reports at every
  position. Any recovery quorum and any superquorum share at least that many
  replicas by counting, so a fast-committed command always qualifies; the
  danger is ambiguity. The qualifying command at a position is unique only
  when `2 * RecoveryThreshold > QuorumSize`, that is, when
  `QuorumSize + 2 * SuperQuorumSize > 2 * N`; otherwise recovery may select a
  competing command. Recovered work is committed through the original engine
  before the marker and the new
  acceleration view.
- **Independent deciders.** The original engine commits the active proposer's
  proposals in its own order (abstract here; production Raft is checked
  separately). An order-preserving proposer proposes in its acceptance order; a
  reordering proposer (no compatible promise) may not. Fast and original
  decisions are recorded independently in `chosen`, so a missing guard shows
  up as two decisions disagreeing rather than as a certificate restating it.

### Source discrepancy: proposer acknowledgement

The model requires, as part of a compatible promise, that the order-preserving
active proposer has itself acknowledged the fast-committed command at the same
position. Until 2026-11-26 the Rust API did **not** enforce this:
`validate_promises` checked only that every active proposer's promise matched
the views and had `proposal_order_preserved`, and `conflict_free` is supplied
by the caller.

**Resolved (2026-11-26) by enforcing it in Rust admission.** The other
resolution, deriving the condition from the profile's base-ordering
assumptions, does not hold. `receive_order_preserved` constrains the order in
which a replica accepts requests. It does not make a superquorum contain the
proposer. Whenever the superquorum is smaller than the replica set,
which includes the derived five-replica profile (superquorum 4 of 5) and the
non-derived four-replica exercise, a superquorum can omit the proposer. Then
the proposer's original engine can decide a different command at that position.
`ConsensusFastPathMissingPromise.cfg` shows exactly this `CommittedOrderAgreement`
counterexample in all three checkers, so the condition stays in the model.

[`evaluate_stable_view`](../crates/molten-fastpath-core/src/quorate.rs) now
falls back with `MissingProposerAcknowledgement` unless every active proposer
is among the same-view acknowledgers. It falls back with
`OriginalPathUnavailable` when the original path is down. The acknowledgers
are the admitted replicas whose acknowledgements match the attempt's views and
operation. Under `conflict_free` that operation is the Rust abstraction of the
model's "same prefix". An active proposer outside the admitted replicas can
therefore never satisfy the guard. The check follows the promise checks, so a
missing, mixed-view or reordering promise keeps its own reason. Evidence:

- **Unit tests in `molten-fastpath-core`.** With five replicas, a 4-of-5
  superquorum without the proposer falls back, and the same size with the
  proposer fast-commits. An active proposer outside the admitted replicas also
  falls back. With two active proposers, a superquorum that omits one of them
  falls back and one that contains both fast-commits.
- **An exhaustive five-replica unit test.** It enumerates all 32 acknowledger
  subsets against all 31 non-empty active-proposer sets, which covers every one
  of the 160 single-proposer combinations and all two-proposer ones. Fast
  admission holds exactly when a superquorum acknowledged and contains every
  active proposer.
- **A bounded Hegel property.** It samples the same rule over seven replicas,
  with a superquorum of 6.
- **The Quint lane.** The replay driver uses five admitted replicas and the
  derived 4-of-5 superquorum. Its `unacknowledged` promise mode drops the
  admitted proposer `r0` from an otherwise complete superquorum, which is the
  TLA+ counterexample shape. The `proposerAcknowledgementTest` run replays it
  against the real API, followed by the same command with the proposer
  included.

## Hazard-to-property mapping

All names in `config/consensus-fastpath/profile.ncl` have executable properties:

| Profile invariant | TLA+ property | Quint composition property |
| --- | --- | --- |
| `recoverability` | `Recoverability` | `recoverability` |
| `no-conflicting-predecessor` | `ExecutionOrderAgreement`, `LinearizableConflictOrder` (derived) | `no_conflicting_predecessor` |
| `committed-order-agreement` | `CommittedOrderAgreement` | `committed_order_agreement` |
| `execution-order-agreement` | `ExecutionOrderAgreement` (derived) | `execution_order_agreement` |
| `linearizable-conflict-order` | `LinearizableConflictOrder` (derived) | `linearizable_conflict_order` |
| `at-most-once-application` | `AtMostOnceApplication` (derived) | `at_most_once_application` |

**Derived** properties are computed from each replica's log and the recorded
decisions; there is no independent execution or application state. Each is
implied by `CommittedOrderAgreement` (`AtMostOnceApplication` is literally its
second half), so they name the profile invariants but add no evidence beyond
it. The independent safety evidence is `CommittedOrderAgreement` and
`Recoverability`.

The hazard prose has additional boundaries:

| Hazard / schedule | Named check and exact abstraction |
| --- | --- |
| Canonical dual-path identity; original-only operation | Not modeled by TLA+: it carries one command token throughout. Quint's identity-drift input checks the real `IdentityMismatch` admission outcome. Canonical encoding, session aliasing, and policy admission are outside TLA+. |
| Unknown schemas, aliases, ranges, predicates, preconditions, response dependencies, analysis failure | TLA+ conservatively treats every pair of commands as conflicting, so a fast admission requires a superquorum to agree on the command's **whole** prefix (its position). It does not implement a schema/conflict classifier. Quint supplies the classifier result as an input and checks real `Conflict` fallback. |
| False-negative conflict classification; stale conflicting predecessor | Not exercised by a TLA+ mutant. Execution is derived: a replica executes its own log only through decided positions, so a predecessor decided differently blocks it; this is a consequence of `CommittedOrderAgreement`, not a separate check. The Rust fault corpus and Quint lane cover classifier fallback. |
| Matching acceleration/base views; view-straddled acknowledgements | Structural only: `FastCommit` requires `accelView = baseView` and counts only replicas currently normal in that view. No mixed-view mutant runs in TLA+; Quint checks the real mixed-view fallback against a patch-mutated Rust guard. |
| Missing compatible proposer promise | `ConsensusFastPathMissingPromise.cfg` admits fast commits without the order-preserving proposer's acknowledgement; TLC, TLA-ts and Dive all report a `CommittedOrderAgreement` counterexample in which the original engine decides a different command at a fast-committed position. Quint exercises missing, mixed-view, and reordering promises against the real API. |
| Insufficient acknowledgements; partition/quorum loss | `ConsensusFastPathInsufficientQuorum.cfg` (three replicas, two-replica superquorum: 2 + 2·2 = 2·3) and `ConsensusFastPathFourReplicaSmallQuorum.cfg` (non-derived four-replica exercise, two-replica recovery quorum: 2 + 2·3 = 2·4) both violate `QuorumSize + 2 * SuperQuorumSize > 2 * N`, so `RecoveryThreshold` (1) no longer selects a unique command and recovery can pick a competing one; all three checkers report `CommittedOrderAgreement` after view-change recovery. Message non-arrival is a replica that never accepts. Neither model simulates a transport partition or durable membership service. |
| Leader failure after fast reply | `Recoverability`: a fast decision may precede original-path commitment; after a view change the coordinator knows only the recovery quorum's logs, and the marker requires every earlier decision in the original engine's durable order. No OS process crash is represented. |
| Interrupted/cascading recovery, including an empty set | `Recoverability`, `RecoveryEventuallyResumes` in the three-replica instance (three views): separate pause, join, agreement, original commit, marker, resume actions; view changes can interrupt recovery. The four-replica instance has a single view change. Quint replays all four real recovery actions and rejected drop/marker/view inputs. |
| Recovered work precedes conflicting new-view work | `CommittedOrderAgreement`: replicas resume only by installing the recovered prefix, which the original engine has already committed. Quint checks the recovery API and ordered ledger composition. |
| Replica restart/rejoin | By construction lagging replicas may skip views and join later with stale logs, and a recovery extension must extend the durable order, so stale logs never support a disagreeing extension; any violation would surface as `CommittedOrderAgreement`. Restart I/O and stable-storage survival are outside this TLA+ state machine; the Rust fault corpus remains a separate lane. |
| Duplicate convergence / authoritative replies | `AtMostOnceApplication` (derived): no command is decided at two positions. Quint invokes the actual `ApplicationLedger::converge` twice and compares outcomes/identities/counts after each step. |

`LinearizableConflictOrder` checks the finite serial conflict order of
executions, **not** real-time linearizability of concurrent network histories.
The original engine is an abstract durable commit log driven by the active
proposer; its own safety is the subject of the separate production Raft model.

`RecoveryEventuallyResumes` is also in the config. `Spec` imposes weak fairness
on `RecoveryProgress` (joining, agreement, recovery commit, marker, resume).
This assumes replicas and the abstract original path eventually perform enabled
recovery work; it does not claim liveness through a permanent partition.
Cascading view increases are bounded by three, not silently excluded.

## Required TLC / TLA-ts / Dive semantic campaign

TLC **2.19 / TLA+ 1.7.4** is authoritative. TLA-ts uses the unchanged npm
**0.0.10** publication and its asynchronous `check({source, config})` API,
including final liveness checking. The third checker is the MIT-licensed
[Dive **0.1.0** x86_64 Linux release](https://github.com/Chelis-Lang/dive/releases/tag/v0.1.0),
tag commit `94cbbed9ae02ad3d30bbe2191bd2619a839b8795`, archive SHA-256
`cf0a0e2ac8fdccc2c7c73ca563ee567703a64b9ebd7f418eca6a4102263ab65d`.
The check invokes `dive check SPEC --config CFG --workers 1 --compat=tlc --output json`
and reads only [schema 3](https://github.com/Chelis-Lang/dive/blob/94cbbed9ae02ad3d30bbe2191bd2619a839b8795/docs/user/reports-and-errors.md).
Dive's TLC profile performs a final temporal check; it does not reproduce
TLC's lasso choice. A parse is never accepted as a semantic verdict.

The agreed engineering bound is **180 seconds per checker/config and 720
seconds for all sequential checker runs** (two baselines and three mutants,
three checkers each: fifteen runs), excluding Nix/tool builds. Individual
and total durations are retained. Wrong pins or versions, timeouts, missing
tools, refusals (Dive exit 3), budget stops (Dive exit 5), malformed reports,
exceptions, incomplete searches, unexpected exits and baseline count mismatches
fail closed. Bounds and properties must not be weakened to fit this limit.

The campaign runs TLC, TLA-ts and Dive on each baseline and each mutant
(`MODEL_RUNS` in `verification/tla-ts/campaign.mjs`). Each baseline must
exhaust the graph and report success, an empty
queue, and generated **and distinct** counts identical to TLC. In particular,
Dive must report a completed and exhausted temporal check; safety-only success
is not enough. `ConsensusFastPathMissingPromise.cfg` disables only the promise
guard, `ConsensusFastPathInsufficientQuorum.cfg` changes only
`SuperQuorumSize`, and `ConsensusFastPathFourReplicaSmallQuorum.cfg` changes
only `QuorumSize` of the four-replica baseline; for each, all three checkers
must report `CommittedOrderAgreement` with a causal trace of
more than one state. A mutant stops at its first violation, so its visited
prefix counts are observations, not parity assertions: different successor
orders can give different prefixes. Investigate any mismatch in the retained
raw traces rather than changing the model.

Run the existing check on x86_64 Linux:

```sh
nix build --keep-failed .#checks.x86_64-linux.consensus-fastpath-model-parity
```

The output contains `campaign.json` with the Dive pin, exact commands,
observed version, durations and BLAKE3 digests for the source/config copies,
raw `.stdout` and `.stderr`, and `.json` summaries. Dive's `.report.json` files
retain the byte-exact native JSON. TLC state-store scratch is not evidence.
On failure, `--keep-failed` preserves the failed build directory. For an
explicit local output directory that survives failure, use a shell with the
same pinned TLC and Node, `b3sum`, and a verified Dive release runnable on the
host. See [`verification/tla-ts/README.md`](../verification/tla-ts/README.md)
for the archive layout.

```sh
scratch=$(mktemp -d)
release=dive-v0.1.0-x86_64-unknown-linux-gnu
# Set this to the absolute path of the exact pinned release archive.
dive_archive=/absolute/path/to/$release.tar.gz
printf '%s  %s\n' cf0a0e2ac8fdccc2c7c73ca563ee567703a64b9ebd7f418eca6a4102263ab65d "$dive_archive" | sha256sum --check || exit 1
tar -xzf "$dive_archive" -C "$scratch" "$release/dive"
tar -xzf verification/tla-ts/tla-ts-0.0.10.tgz -C "$scratch"
node --test verification/tla-ts/campaign.test.mjs
node verification/tla-ts/campaign.mjs "$scratch/evidence" "$scratch" "$dive_archive" "$scratch/$release/dive"
```

The test inputs in `campaign.test.mjs` are labeled synthetic reader tests, not
observed checker logs. They cover count divergence, refusal, budget stops,
incomplete or parse-only output, timeout, pin/version drift, wrong mutant
properties and missing traces.

## Required Quint-to-Rust lane (separate)

`verification/tla/ConsensusFastPath.qnt` mirrors the existing pure API:
`BeginViewChange`, `AgreeRecoverySet`, `CommitRecoveryMarker`,
`ResumeNormalView`, fast admission and convergence. `tests/fastpathquint.rs`
calls **the real** `transition_recovery`, `evaluate_stable_view` and
`ApplicationLedger::converge`. The `State` projection compares phases, all
views, accepted/recovery sets, marker presence, admission decisions and
application/reply identities/outcomes after **every** trace step. Rejected
transitions must preserve the rest of the state. Order is a caller/composition
precondition, not functionality invented inside the ledger.

The dev-only Nix cohort in `nix/consensus-fastpath-quint.nix` pins:

- Quint **0.33.1**, revision `3eaa85fc01820ea25bd7cb7baee0bb8d5c02c8b0`,
  SHA-256-pinned upstream source archive, unchanged upstream npm lock and
  Apache-2.0 notices.
- `quint-connect` **0.1.2**, revision
  `4f018f54fc7dd4cef341d10111427bab59d3b307` (Apache-2.0), as a Cargo
  **dev dependency only**.
- The wrapper selects Quint's bundled **TypeScript evaluator**. Upstream
  quint-connect does not pass `--backend`, and Quint otherwise defaults to a
  separately downloaded Rust evaluator; that unpinned download is not allowed.

Missing/drifting Quint fails rather than skipping. The test campaign uses seed
5, 64 traces and 48 steps, plus deterministic recovery/duplicate/empty-recovery,
mixed-view and proposer-acknowledgement fixtures. The six Quint property names above are checked by
`quint run --invariant inv`; replay separately checks Rust state equality.
This is trace-based conformance, not proof of TLA+/Quint equivalence.

The real Rust guard mutant is
`verification/tla/mutants/mixed-view-guard.patch`. Apply it **only to an
isolated source copy**, using the Nix module's `mutateSource` function. It
removes the actual acceleration-view comparison in `same_view_acknowledgers`
(`crates/molten-fastpath-core/src/quorate.rs`, which the mutated workspace
rebuilds as `molten`'s path dependency);
there is no alternate test implementation. Build normal and mutated
`fastpathquint` test executables from the same pinned dependencies, then run:

```sh
node verification/tla/quint-campaign.mjs \
  /absolute/new/quint-evidence \
  /absolute/normal/fastpathquint-test-binary \
  /absolute/mutated/fastpathquint-test-binary
```

The campaign retains generated ITF traces, normal and mutant replay logs and
durations. It requires all five normal replay tests and a mutant failure
containing `State invariant failed`. The authored
`fixtures/mixed-view-guard.json` describes the causal minimum: initialization
plus one mixed-view admission input. It is **not** a fabricated observed ITF;
the campaign requires the generated two-state trace before claiming evidence.

### Can Quint replace the parity source?

Not established. In the pinned Quint source,
[`compileToTlaplus`](https://github.com/quint-co/quint/blob/3eaa85fc01820ea25bd7cb7baee0bb8d5c02c8b0/quint/src/compileToTlaplus.ts)
uses an **Apalache server** to translate analyzed Quint IR. Merely having Quint
or generating simulation traces does not establish verdict or state-count
parity with TLC. An additional pinned Apalache/compiler cohort, emitted-module
review and aligned state mapping would be needed to qualify that route. This
change keeps the hand-written `.tla` as parity authority; the Quint API model
is intentionally a separate abstraction.

## Optional Veil lane

Veil is not selected as a replacement or a required dependency. The issue's
candidate revision is `57447722e7f25f06c75cff8735d10d41697afc14`; its
[`lean-toolchain`](https://github.com/verse-lab/veil/blob/57447722e7f25f06c75cff8735d10d41697afc14/lean-toolchain)
requires **Lean 4.35.0**. Its shipped `tla-to-veil` guidance calls for matching
actions, fairness, known violations and distinct-state counts, not just parsing
or proving an unrelated invariant. No Veil translation, `#model_check`,
`#temporal_check`, timing or speedup is claimed here. That optional temporal
check is out of scope because this change does not select/build a reviewed
Lean/Veil cohort or translated model; the required TLA+ temporal property is
not dropped. Do not substitute UPDR output for implementation proofs.

## Integration and evidence status

Root `flake.nix` imports `nix/consensus-fastpath-model.nix` for the x86_64 Linux
three-checker parity rail and `nix/consensus-fastpath-quint.nix` for the pinned
Quint evaluator and normal/guard-mutated Rust conformance check. The Cargo dev
dependency, lock, Git-source hashes and both unit2nix plans carry the exact
Quint cohort above.

Both the hermetic `nextest` check and the developer `nextest-ci` app include
the pinned Quint evaluator directly; they do not depend on an ambient dev
shell. Source copying and normal/mutant builds remain outside the 720-second
model campaign budget.

### Distributed rewrite (2026-11-26)

The rewritten per-replica model passed the integrated three-checker rail
(`nix build .#checks.x86_64-linux.consensus-fastpath-model-parity`, built on
x86_64 Linux) in 169 seconds of checker time, inside the 720-second bound.
TLC, TLA-ts and Dive agreed, with an empty queue and the temporal property
checked, on:

| Run | Generated | Distinct | Verdict (all three checkers) | TLC / TLA-ts / Dive |
| --- | --- | --- | --- | --- |
| three replicas, derived quorums | 483,262 | 151,928 | pass | 26 s / 12 s / 6 s |
| four replicas, non-derived exercise | 908,720 | 228,838 | pass | 52 s / 25 s / 15 s |
| missing promise | 7,660 | 3,212 | `CommittedOrderAgreement`, 7-state trace | |
| three replicas, two-replica superquorum | 43,827 | 18,178 | `CommittedOrderAgreement`, 10-state trace through recovery | |
| four-replica exercise, two-replica recovery quorum | 196,833 | 58,253 | `CommittedOrderAgreement`, 11-state trace through recovery | |

Mutant counts are identical across the checkers here but remain observations.
The Quint API model, `tests/fastpathquint.rs` and the Quint campaign mirror the
Rust API rather than this TLA+ state shape and were unchanged by that rewrite;
the later proposer-acknowledgement resolution (above) added one promise mode
and one replay test to them.

The evidence below describes the **earlier centralized model** and is retained
as history; its `NoMissingPromise` certificate invariant no longer exists.

The original two-checker and Quint campaigns completed on 2026-11-13. With
three replicas, two commands and three views, TLC and TLA-ts each generated 6,312,147 states and
visited 966,147 distinct states with an empty queue. Both rejected the actual
missing-promise mutation with a six-state `NoMissingPromise` trace. The pinned
Quint TypeScript campaign passed 64 traces of at most 48 steps, all four real
Rust replay tests, and the actual guard-mutated Rust executable failed state
comparison as required. Its generated two-state causal witness is retained in
[`mixed-view-guard.itf.json`](../verification/tla/fixtures/mixed-view-guard.itf.json).
The [retained TLC/TLA-ts files](../.cairn/archive/2026-11-15-check-consensus-fastpath-parity/evidence/tlc-tla-ts-raw-2026-11-13/)
contain the actual stdout, stderr, verdicts and source/config copies, with
byte-identical BLAKE3 retention bindings; internal TLC state-store scratch is
not claimed. The [Quint campaign](../.cairn/archive/2026-11-15-check-consensus-fastpath-parity/evidence/quint-2026-11-13/campaign.json)
retains its traces and normal/mutant replay observations. These are bounded
observations; final source/lifecycle acceptance remains distinct from
model-checker or replay success.

[Issue #62's separate Dive trial](https://github.com/brittonr/aspen/issues/62)
records the same baseline counts and the six-state `NoMissingPromise` trace
on unchanged inputs. That trial is not execution evidence from the integrated
three-checker rail and is not a speed comparison. Dive agreeing with TLC is
not a proof of Aspen's Rust implementation.

The [integrated three-checker run](../.cairn/archive/2026-11-21-add-dive-consensus-fastpath-checker/evidence/verification.md)
then passed all five synthetic controls and six checker invocations. Dive,
TLC and TLA-ts agreed on 6,312,147 generated and 966,147 distinct baseline
states with no queued states; all three retained a six-state
`NoMissingPromise` trace for the mutant. The archived bundle binds the exact
model, configurations, version, options, streams and per-checker durations.

An additional isolated `nextest-ci` app run on 2026-11-14 passed all four real
Rust replay tests with ambient Quint absent. It used genuine locked
`cargo metadata --no-deps` data and the previously built, BLAKE3-bound test
binary. The [invocation and result](../.cairn/archive/2026-11-15-check-consensus-fastpath-parity/evidence/nextest-ci-2026-11-14/invocation.json)
establish evaluator availability through the actual app, not a cold public
bootstrap or a repair of the separate default Cargo metadata panic.

All outputs are finite-model or bounded API-conformance evidence only. They do
not prove the external Jetpack artifact, live Aspen transport, storage
survival, timing, production linearizability, arbitrary predicates, Byzantine
tolerance, throughput, latency, transactions or release readiness.
