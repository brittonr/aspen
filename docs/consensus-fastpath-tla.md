# Aspen consensus fast-path model checks

`verification/tla/ConsensusFastPath.tla` is the authoritative **finite design
model**, not a proof of Aspen's implementation. The internal Rust crate/CLI and
canonical requirement identifiers retain the name `molten`.

The instance is exactly **three replicas, two canonical command tokens, and
three views**. The larger Nickel fault-corpus profiles and the five-replica
profile are not this instance. No symmetry reduction, state constraint, depth
cutoff, or unchecked invariant is used to make parity fit the runtime budget.
`CHECK_DEADLOCK FALSE` permits quiescence; it does not discharge progress.

## Hazard-to-property mapping

All names in `config/consensus-fastpath/profile.ncl` have executable properties:

| Profile invariant | TLA+ property | Quint composition property |
| --- | --- | --- |
| `recoverability` | `Recoverability`, `RecoveryBeforeNewView` | `recoverability` |
| `no-conflicting-predecessor` | `NoConflictingPredecessor` | `no_conflicting_predecessor` |
| `committed-order-agreement` | `CommittedOrderAgreement` | `committed_order_agreement` |
| `execution-order-agreement` | `ExecutionOrderAgreement` | `execution_order_agreement` |
| `linearizable-conflict-order` | `LinearizableConflictOrder` | `linearizable_conflict_order` |
| `at-most-once-application` | `AtMostOnceApplication` | `at_most_once_application` |

The hazard prose has additional boundaries:

| Hazard / schedule | Named check and exact abstraction |
| --- | --- |
| Canonical dual-path identity; original-only operation | `TypeOK`, `LinearizableConflictOrder`: TLA+ carries one command token throughout. Quint's identity-drift input checks the real `IdentityMismatch` admission outcome. Canonical encoding, session aliasing, and policy admission are not modeled by TLA+. |
| Unknown schemas, aliases, ranges, predicates, preconditions, response dependencies, analysis failure | `NoConflictingPredecessor`: TLA+ conservatively treats every distinct command as conflicting. It does not implement a schema/conflict classifier. Quint supplies the classifier result as an input and checks real `Conflict` fallback. |
| False-negative conflict classification; stale conflicting predecessor | `NoConflictingPredecessor` checks the pending accepted predecessor set captured at fast admission. Base proposal/receive ordering is an explicit assumption; it is not an independently proved Rust engine. |
| Matching acceleration/base views; view-straddled acknowledgements | `NoMixedViewCommit` checks **retained admission certificates**, including every superquorum member's view. Old acknowledgements deliberately survive view changes, so the property is not merely a bound on current acknowledgements. Quint checks real mixed-view fallback. |
| Missing compatible proposer promise | `NoMissingPromise` checks each admission certificate. TLA+ conservatively requires promises from every replica; Quint uses the actual single active proposer fixture and exercises missing, mixed-view, and reordering promises. |
| Insufficient acknowledgements; partition/quorum loss | `NoMixedViewCommit` plus the superquorum admission guard; Quint checks insufficient acknowledgements. Message non-arrival is nondeterministic absence of acknowledgement. Neither model simulates a transport partition or durable membership service. |
| Leader failure after fast reply | `Recoverability`, `RepliesAfterApplication`: fast decisions/application/reply may precede original-path commitment; view change must carry accepted work into recovery. No OS process crash is represented. |
| Interrupted/cascading recovery, including an empty set | `Recoverability`, `RecoveryBeforeNewView`: separate pause, agreement, original commit, marker, resume actions; view changes can interrupt recovery. Quint replays all four real recovery actions and rejected drop/marker/view inputs. |
| Recovered work precedes conflicting new-view work | `CommittedOrderAgreement`, `ExecutionOrderAgreement`, `RecoveryBeforeNewView`: the original commit sequence must be a prefix of decision order before resumption. Quint checks the recovery API and ordered ledger composition, not original-engine commitment. |
| Replica restart/rejoin | `Recoverability`, `ExecutionOrderAgreement` describe retained abstract state only. Restart I/O, stable-storage survival and rejoin protocols are outside this TLA+ state machine; the Rust fault corpus remains a separate lane. |
| Duplicate convergence / authoritative replies | `AtMostOnceApplication`, `RepliesAfterApplication`; Quint invokes the actual `ApplicationLedger::converge` twice and compares outcomes/identities/counts after each step. |

`CommittedOrderAgreement` assumes one original-engine commit order;
`ExecutionOrderAgreement` checks each replica's lagging application prefix.
`LinearizableConflictOrder` checks the finite serial conflict order and reply
prefix, **not** real-time linearizability of concurrent network histories.
Fast admission records its actual promise/acknowledgement/predecessor witnesses;
later changes cannot make an invalid certificate appear valid.

`RecoveryEventuallyResumes` is also in the config. `Spec` imposes weak fairness
on `RecoveryProgress`. This assumes the abstract original path can eventually
perform enabled recovery work; it does not claim liveness through a permanent
partition. Cascading view increases are bounded by three, not silently excluded.

## Required TLC / TLA-ts semantic campaign

Pins: TLA+ **1.7.4 / TLC 2.19**, and the unchanged npm **TLA-ts 0.0.10**
publication (see `verification/tla-ts/README.md`). `check.mjs` invokes the
published asynchronous `check({source, config})`, including temporal checking,
and exports real verdicts, generated/distinct/queued counts and formatted
counterexample states. A parse is never accepted as a semantic verdict.

The agreed engineering bound is **180 seconds per checker/config, 720 seconds
for the four sequential runs**, excluding Nix/tool builds. Both individual and
total durations are retained. Timeouts, missing tools, unsupported operators,
exceptions, incomplete searches, unexpected exits, wrong versions and baseline
state-count mismatches fail closed. Bounds/properties must not be weakened to
fit this limit.

The four runs are baseline TLC, baseline TLA-ts, missing-promise mutant TLC,
and missing-promise mutant TLA-ts. The baseline must exhaust the graph, agree
on success, and report identical generated **and distinct** state counts.
`ConsensusFastPathMissingPromise.cfg` disables only the promise guard; both
checkers must report `NoMissingPromise` and a nonempty causal trace. The mutant
stops at its first violation, so its visited-prefix counts are retained and
compared but are not misrepresented as exhaustive graph counts; different
successor orders can give different prefixes. Investigate any mismatch in the
retained raw traces rather than changing the model.

After root flake integration:

```sh
nix build --keep-failed .#checks.x86_64-linux.consensus-fastpath-model-parity
```

The output directory contains `campaign.json`, the exact source/config copies,
per-run `.stdout`, `.stderr`, `.json` summaries, and TLC metadata. On failure,
`--keep-failed` preserves the failed build directory. For an explicit local
output directory that survives failure, inside a shell containing the same
pinned TLC and Node:

```sh
scratch=$(mktemp -d)
tar -xzf verification/tla-ts/tla-ts-0.0.10.tgz -C "$scratch"
node --test verification/tla-ts/campaign.test.mjs
node verification/tla-ts/campaign.mjs "$scratch/evidence" "$scratch"
```

The test inputs in `campaign.test.mjs` are labeled synthetic reader tests, not
observed checker logs. They cover count divergence, incomplete/parse-only
output, tool drift, wrong mutant properties and missing traces.

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
5, 64 traces and 48 steps, plus deterministic recovery/duplicate/empty-recovery
and mixed-view fixtures. The six Quint property names above are checked by
`quint run --invariant inv`; replay separately checks Rust state equality.
This is trace-based conformance, not proof of TLA+/Quint equivalence.

The real Rust guard mutant is
`verification/tla/mutants/mixed-view-guard.patch`. Apply it **only to an
isolated source copy**, using the Nix module's `mutateSource` function. It
removes the actual acceleration-view comparison in `same_view_acknowledgers`;
there is no alternate test implementation. Build normal and mutated
`fastpathquint` test executables from the same pinned dependencies, then run:

```sh
node verification/tla/quint-campaign.mjs \
  /absolute/new/quint-evidence \
  /absolute/normal/fastpathquint-test-binary \
  /absolute/mutated/fastpathquint-test-binary
```

The campaign retains generated ITF traces, normal and mutant replay logs and
durations. It requires all four normal replay tests and a mutant failure
containing `State invariant failed`. The authored
`fixtures/mixed-view-guard.json` describes the causal minimum: initialization
plus one mixed-view admission input. It is **not** a fabricated observed ITF;
the campaign requires the generated two-state trace before claiming evidence.

### Can Quint replace the parity source?

Not established. In the pinned Quint source,
[`compileToTlaplus`](https://github.com/quint-co/quint/blob/3eaa85fc01820ea25bd7cb7baee0bb8d5c02c8b0/quint/src/compileToTlaplus.ts)
uses an **Apalache server** to translate analyzed Quint IR. Merely having Quint
or generating simulation traces does not establish TLC/TLA-ts verdict or state
count parity. An additional pinned Apalache/compiler cohort, emitted-module
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

Root `flake.nix` imports `nix/consensus-fastpath-model.nix` for the TLC/TLA-ts
checks and `nix/consensus-fastpath-quint.nix` for the pinned Quint evaluator and
normal/guard-mutated Rust conformance check. The Cargo dev dependency, lock,
Git-source hashes and both unit2nix plans carry the exact cohort above.

Both the hermetic `nextest` check and the developer `nextest-ci` app include
the pinned Quint evaluator directly; they do not depend on an ambient dev
shell. Source copying and normal/mutant builds remain outside the 720-second
TLC/TLA-ts model budget.

The parent completed both campaigns on 2026-11-13. With three replicas, two
commands and three views, TLC and TLA-ts each generated 6,312,147 states and
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
