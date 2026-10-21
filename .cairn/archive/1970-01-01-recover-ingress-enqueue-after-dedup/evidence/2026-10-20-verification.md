# F03 executed verification — 2026-10-20

## Subject, outcome, and owner

The implementation is in the dedicated `aspen-f03-ingress-20261020` worktree, based on `60ee887549335f50ab4322a2e6e3fe554a822303`. The historical audit examined `fa1ced3e808861d8ce59f02a6fd6b13b655f5147` and recorded F03 as a source finding, not an executed crash reproduction. The edited tree is not identified here as a committed or released candidate.

Immediate outcome: dedup acceptance cannot silently acknowledge an unpublished request. Durable capability: pure, exact-identity reconciliation plus atomic dedup/publication intent, capability-rooted no-replace publication, durable quarantine, and repeatable reopen-boundary regressions. Current consumers are node-control ingress retries, direct submission, and normal control dispatch. Node-runtime maintainers own decisions and ordering; delivery-idempotency maintainers own reservation and intent persistence; node-host maintainers own filesystem mechanisms.

The implementation decision in [design.md](../design.md#implementation-decision--2026-10-20) consumes the existing Transactional Reconciliation Core quarantine vocabulary. General storage repair, consensus recovery, and voting admission remain outside F03.

## Baseline evidence and chronology

The retained original control run in the predecessor worktree passed three tests: fresh enqueue/provenance, missing-authority denial, and tampered-envelope denial. A later immutable checkout of `60ee887549335f50ab4322a2e6e3fe554a822303` repeated the same three controls successfully. The immutable replay is retrospective pre-fix evidence; it is not claimed as an additional chronological pre-edit capture.

A public-API example in that immutable checkout committed legacy dedup, stopped before inbox publication, reopened through normal delivery, and printed:

```text
F03 pre-fix decision=pass phase=duplicate-or-deny enqueued=false queue=None diagnostics=0 receipt=blake3:1f9aa76d24b23662161ed539a1597cde84db8798898bb1cfa673844d96bbb286
```

This executes the original lost-publication counterexample using actual local storage. It does not retroactively change the original audit's evidence class or claim physical crash injection. The task-owned baseline example and detached worktree were removed after execution.

## Post-fix public-API exercise

Executed:

```text
nix develop -c cargo run --example f03_recovery_smoke -- /tmp/molten-f03-smoke-ffvjh7pg
```

The temporary example initialized isolated roots and exercised actual public ingress, delivery-idempotency, Redb, and node-state APIs. Reservation, interrupted `publishing`, and quarantine states were established through those APIs and reopened before delivery. It did not start a node or use invented startup admission evidence.

Observed output:

```text
fresh duplicate: phase=duplicate-publication enqueued=false queue_preserved=true receipt=blake3:4e7428f29f3870bc52d7417719cbec287ebc3be613f37eb479020bde18bda532
fresh: decision=pass phase=deliver enqueued=true queue=Some("blake3:27236445c7a3513fae1a7d69e03792fa87691f06986e09bfcbdcd9d95aae5e5a") receipt=blake3:395384078c1104ce2a978562af7922fc240d68d68207515adca0f4a439f0bea8
recovery duplicate: phase=duplicate-publication enqueued=false queue_preserved=true receipt=blake3:4e7428f29f3870bc52d7417719cbec287ebc3be613f37eb479020bde18bda532
recovery: decision=pass phase=recovered-publication enqueued=true queue=Some("blake3:27236445c7a3513fae1a7d69e03792fa87691f06986e09bfcbdcd9d95aae5e5a") receipt=blake3:b620bc19ac4137f35c6f4d8df65f0b592d0a449711b2a266f4db4b53147c354a
legacy: decision=deny phase=outcome-unknown enqueued=false queue=None receipt=blake3:2b1869b693a586949268f58f7a21505c6eab2618b8d7c0105903552a15bad082
unknown: decision=deny phase=outcome-unknown enqueued=false queue=None receipt=blake3:2b1869b693a586949268f58f7a21505c6eab2618b8d7c0105903552a15bad082
quarantine: decision=deny phase=outcome-unknown enqueued=false queue=None receipt=blake3:83ca5863965b94922a594d099ed17616ecb2baa83fb748eb9cc68749b4cb1d8b
```

The example asserted canonical decisions/phases, actual enqueue, queue-reference preservation on exact retries, and denial without enqueue for unresolved legacy, interrupted, and quarantined cases. The temporary example and its owned state root were removed after execution.

## Permanent coverage and compatibility

- `crates/molten-core/src/node_ingress/tests/`: pure decision precedence, exact identity, healthy recovery, legacy uncertainty, conflicts, unavailable observations, and quarantine.
- `src/node/ingress/tests/positive.rs`: fresh publication, exact duplicate suppression, reopen after atomic reservation, inbox-only observation repair, and immutable completed-dispatch observations.
- `src/node/ingress/tests/interruptions.rs`: executed status before result publication, result before dispatch receipt, ambiguous publication, malformed intent phase, persisted quarantine, unavailable observations, and an existing empty store.
- `src/node/ingress/tests/negative.rs`: conflicting queue request/operation, inbox payload, and historical control-result request bindings; prior dedup/inbox/dispatch facts remain unchanged.
- `src/node/ingress/tests/compatibility.rs`: legacy dedup without fabricated intent, exact surviving inbox/result observations, and queue-only uncertainty.

Queue/ingress v1 receipt shapes and delivery identity/sequence framing are unchanged. New phases distinguish recovery, observation repair, duplicate publication, completed dispatch, conflict, and unknown outcome. Observation repair produces new evidence; it does not invent an original enqueue receipt or overwrite a historical result. Actual status-dispatch unit cases use the existing `cfg(test)` startup fixture and do not constitute F02 startup-policy evidence.

## Pre-review source and focused Nix gates

- `nix develop -c cargo fmt --check`: passed after the source-policy refactor.
- Configured workspace and library Octet producers: both **clean**, zero findings/warnings/errors, pinned `cargo-octet 0.1.0` and `nightly-2026-03-21-x86_64-unknown-linux-gnu`. Workspace config/profile hashes: `b3:f6d90d147080ec9c5ff594bf443aed65b8f60517ec0eecdda92502b1f6964268` / `b3:bdaab55d417efab9becf009dbf8e655a084da641a854331fc72bf76fab13e482`. Library config/profile hashes: `b3:7e5ddc24bd10c119cbb858ca382e7f0f4cbd629a2fe1075ec8402936da19d71a` / `b3:2aad1f1f1a78bc9277fdb6abe27b39a261cacbd0e2d280f548667b61f852dbbd`.
- Object corpus: generated successfully from all **1,928** sorted `src/**/*.rs` members using 95 bounded enumeration queries with no truncated leaf scope.
- Artifact import: **pass**, six artifacts, receipt `blake3:68ececc77f0e10df25e9f8139e26d6e6d5500b8c3cfd5280a0664f01dbabbef8`.
- Unchanged `strict-ci` source gate: **pass**, receipt `blake3:e731694e88caa82d12a3532b8971b05126b2d97bc7c53ef14335339eafc34df4`.
- Remediation plan: completed, receipt `blake3:67d442e52ea136f91422ee5f65291a4ac146c7cc7302c6c412b86f872e95c254`.
- `nix build --no-link .#checks.x86_64-linux.node-ingress-octet-deny-all .#checks.x86_64-linux.node-state-authority .#checks.x86_64-linux.fabric-port-boundaries .#checks.x86_64-linux.inherited-tracey-debt .#checks.x86_64-linux.nextest-config -L`: **passed**. The focused real-core compile had warnings denied and zero Octet findings; output `/nix/store/jlb27xj6752va04fs3nqlh3qxj9gb0zr-octet-consumer-check`.
- Inherited traceability: **pass**, 2,857 accepted requirements, 924 referenced, 1,933 uncovered matching the unchanged 1,933-entry baseline, 10 pending references, zero dangling references. Pending references establish identity resolution only, not accepted coverage or completion.
- Separate proposal/design/tasks gates: **pass**, zero issues, explicit owner policy `/home/brittonr/git/OnixResearch/cairn/cairn-policy/generated/cairn-policy.json`, BLAKE3 `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`. These are structural planning observations, not semantic acceptance authority.

## Retained failed attempts

- An early normal focused run had 11 passing and 16 failing tests because the repair-abort callback also rejected initialization of newly created empty Redb files. The adapter now distinguishes atomically acquired new files from existing files; existing empty/repair-requiring stores still reject rather than silently reset history.
- Initial Octet producers reported excessive file lengths, redundant path naming, and six-argument helpers. The implementation was split into focused files and explicit input structs; no lint allowance, warning baseline, or policy weakening was introduced.
- The first focused Nix invocation could not see the new check manifest through Git-backed source capture. The first full flake invocation likewise could not read new `src/delivery/parts/idempotency/p007/body.rs`. Task-owned new source files were admitted to the Git-backed source set; no product fallback or weaker check was substituted.
- A pre-review workspace Clippy run rejected two collapsible nested guards in `src/node/ingress/state.rs` and `src/node/ingress/mod.rs`. The supported let-chain code action and the later error-boundary correction removed them; warnings remain denied. Fresh post-review verification follows the final source-gate sequence.
- Read-only reviews found lease-contention terminal denial, pre-reservation global quarantine, and dropped dedup rejection diagnostics. These were corrected rather than accepted as limitations; post-review execution is recorded below.
- A workspace producer overlapped the temporary lease/diagnostic example and reported eight warnings, all in `examples/f03_lease_smoke.rs`; the library producer was clean. The unchanged `strict-ci` gate rejected that exit-zero `warning-only` result with receipt `blake3:e14e0c4e6bcd46302f29ee500cdbb0898aaf0ef0f7f78e505e2e9d2e10c1433d`; later test/lint steps in that chain did not run. The example was already removed after execution; both producers and the strict sequence were restarted without that scaffold.
- The sibling Cairn provider's raw `path:` fetch failed with `file '/socket' has an unsupported type`. Git-backed source capture then failed to build with `E0583`, missing module `source_review`. No sibling source was changed. The already installed immutable producer `/nix/store/j3ni3lj66ylskpcg78ip531s9lz4z4av-cairn-0.1.0/bin/cairn` successfully previewed synchronization under the same explicit owner policy, whose BLAKE3 remains `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`; no policy or strictness was reduced.

## Claim boundary

Controlled persisted-boundary observations are not hardware power-loss or actual Redb commit/synchronization fault injection. They do not establish exactly-once external effects, arbitrary storage durability, consensus recovery, production startup readiness, or release eligibility. Full-workspace, full-flake, final lifecycle, and publication observations are recorded separately as they complete; no success for those gates is inferred from focused checks.

## Normal focused execution

- `nix develop -c cargo test -p molten-core node_ingress`: **16 passed**, zero failed or ignored. This executes the real workspace core, not only the hermetic deny-all compile.
- `nix develop -c cargo test --lib ingress -- --nocapture`: **28 passed**, zero failed or ignored, including the three original controls, normal supervised ingress delivery, and every F03 adapter recovery/denial boundary.
- Controlled persisted-boundary cases include actual status execution before result publication and immutable result observation before a dispatch receipt. Their synthetic startup fixture is explicitly test-only; neither result substitutes for independent normal-startup gate evidence.

## Review corrections and post-review execution

Read-only shell/store reviews found two availability regressions: non-blocking lease
contention could create a terminal ingress receipt, and errors before reservation
could durably quarantine unrelated requests. The adapter now waits for the lease,
isolates pre-reservation rejection/read failures, and validates direct-submission
conflicts before publication. Post-reservation uncertainty still fails closed.

Permanent regressions now cover concurrent submissions, a rejected over-bound input
followed by valid delivery, unreadable per-request observations followed by an
independent submission, and conflicting direct submission followed by an independent
submission. Prior dedup/inbox facts remain preserved.

The temporary public-API lease exercise held the actual publication lease in another
thread for 300 ms and then delivered through the normal API. The post-fix observation
was:

```text
lease after: decision=pass phase=deliver enqueued=true queue=Some("blake3:27236445c7a3513fae1a7d69e03792fa87691f06986e09bfcbdcd9d95aae5e5a") receipt=blake3:395384078c1104ce2a978562af7922fc240d68d68207515adca0f4a439f0bea8 total_ms=310
```

This is one contention/publication observation, not a latency bound. After those
corrections, `nix develop -c cargo test --lib ingress -- --nocapture` passed **31
tests**, zero failed or ignored. The earlier full-flake success preceded these review
corrections; final source, workspace, and flake results are recorded below rather than
inferred from that earlier run.

A final public-API exercise repeated contention, then supplied sequence 3 after
accepted sequence 1 for the same peer/topic. The canonical ingress receipt denied
the gap without enqueue and retained the delivery diagnostics:

```text
gap: decision=deny phase=duplicate-or-deny diagnostics=["publication reconciliation unresolved: Rejected", "delivery sequence 3 leaves gap before expected 2", "node control ingress idempotency decision gap"]
```

The exercised temporary example and its guarded state root were removed. This uses
actual receipt readback, not a permanent test that pins incidental diagnostic wording.

## Post-review full flake execution

`nix flake check --keep-going -L` **passed all 77 selected x86_64-linux checks** after
the availability corrections. Incompatible systems were omitted by Nix; this is not
an all-systems result. Nix can reuse realized checks, so this does not assert that
every VM profile executed afresh.

The hermetic nextest check ran **1,794 tests: 1,794 passed, zero skipped**, with
`ci-test-run` receipt
`blake3:3c2359b74a68591fc85d32511f8f2234abd3f60b9a933001b3267ab5b519b9af`.
Its output is `/nix/store/q8ilygnqnb4c5wvrx75zws0rhx5wc6m9-molten-nextest`.
The local-node dogfood check also passed at
`/nix/store/j4j31s35h9hdcyngxa1qj5jvvw7kpwm4-molten-dogfood-local-node`.
Those dogfood release-shaped receipts remain evidence-only; no release was authorized.

## Hook-compatible lifecycle producer

The installed producer's derivation identifies immutable source
`/nix/store/6bpdhd8hszl5jgggf0k16yys89wgxhi5-source`. Its exported owner-policy JSON
has the same BLAKE3 `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`
as the live sibling export. Running `nix run path:SOURCE#cairn -- validate --root .
--policy SOURCE/cairn-policy/generated/cairn-policy.json --strict` against that
immutable source passed: 39 active changes, zero issues/findings, receipt
`93b159de79c43155ccc048314cd5363de18d60fbf22461db9c0c6f01a692bf83`.

Publication can set `CAIRN_ROOT` to this actual immutable provider source. The existing
pre-push hook then runs unmodified through its documented Nix launcher, rather than
ingesting unrelated current-provider work or bypassing a hook.

## Final scaffold-free source and workspace sequence

After removing the smoke example, the complete strict sequence ran in order: both
configured Octet producers, corpus receipt, artifact import, unchanged `strict-ci`
gate, remediation plan, default workspace tests, compatible-feature all-targets
tests, warnings-denied Clippy, formatting, and strict owner-policy Cairn validation.

- Both Octet producers: **clean**, zero findings/warnings/errors, with the unchanged
  pinned toolchain and config/profile identities recorded above.
- Corpus: all **1,928** sorted Rust source inputs, enumerated in 94 bounded queries
  with no truncated leaf scopes; **14,360** objects, set identity
  `b3:5e7152bc9b58f4c7fe1adf9eb95e1606a8b71a433bc62534bf35eff93972702d`.
- Six-artifact import: **pass**, receipt
  `blake3:fb151003765106e2d1e8bdff03e3128c78bf4a47b95eb185a2c261c71b41990a`.
- Unchanged `strict-ci` gate: **pass**, receipt
  `blake3:499e4e1c06c9b854a605f0b7eb13c2c81d38fb5eadc8d31c2a41d9e460a147dc`.
- Remediation plan: completed, receipt
  `blake3:a89681c4deb65c54eb60fe8f658cc9f1e698c4447597f5b449b1c5f8020381e6`.
- `nix develop -c cargo test --workspace --quiet`: **2,257 passed**, zero failed or
  ignored, across 21 suite summaries including doctests.
- `nix develop -c cargo test --workspace --all-targets --features
  doltlite-oracle,executable-extents,world-snapshot-vm-cohort --quiet`: **2,269 passed**,
  zero failed, one ignored external DoltLite oracle. This does not claim that oracle
  or profiling backends executed.
- `nix develop -c cargo clippy --workspace --all-targets --features
  doltlite-oracle,executable-extents,world-snapshot-vm-cohort -- -D warnings`: **passed**.
- `nix develop -c cargo fmt --check`: **passed**.
- Installed immutable Cairn with the explicit live sibling owner policy and
  `--strict`: **passed**, 39 active changes, zero issues/findings, receipt
  `7a72c353d152f4032982835ed58fd1751c0e81ecb22753aa896a51a5b884717f`.

Separate proposal/design/tasks gates also passed with zero issues under that owner
policy. These planning gates and checked task boxes do not themselves grant semantic
acceptance, release authority, or publication authority.

## Completion review

After all implementation boxes were checked, the tasks gate passed with all 15
tasks complete, zero issues, receipt
`89847c98a572b7697861b5e1bbc96c1d3e3ed43ddc2830c76e0d067258bf4f09`.
Fresh strict owner-policy validation passed with 39 active changes and zero
issues/findings, receipt
`acb6056838fcc776d0c72179b306c717cfac94ede274b721eac63e27569995c3`.
Native completeness review passed without findings, receipt
`c2e84be578b265b3a219af59b2d91dc51fcb631c75953c2ddf814d3ce97f3f4a`.

The read-only implementation reviews were static. Their availability/diagnostic
findings were corrected and exercised as recorded above. Native completeness is
structural linkage evidence, not independent human approval or a correctness proof.

## Native sync and archive

Under the same explicit owner policy, the final sync preview was unblocked and
execution merged exactly the five F03 requirements into the accepted node-runtime
specification; receipt
`eb358dca97cf93936dc17206791e4146dd4fade303f05430bd112d6eaabfa19e`.
The archive preview was separately unblocked; execution moved the completed package
to `.cairn/archive/1970-01-01-recover-ingress-enqueue-after-dedup`, receipt
`05a807099e1e6fc51c36ac3a5f3063686e63cbdb75a2545f18b3e5bcf73f7897`.
The archive date is the native tool's selected destination, not a claimed execution
date. The audit's current proposal link now targets that archive; its original
source-only evidence classification remains unchanged.

After sync/archive, the inherited-traceability Nix check passed with all 13 guard
tests passing, 1,933 unchanged inherited-debt entries, and zero dangling references.
Strict owner-policy validation then passed with 38 active changes, zero
issues/findings, receipt
`2fbc92b5fee9ec6fea5c370c1058c13603be5994633f0598f20ba1991fd5396b`.
