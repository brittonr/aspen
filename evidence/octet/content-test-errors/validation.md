# Content-replication test-error validation

## Result

The private integration helpers return explicit errors. The final focused suite passes 22 tests. Both standard nextest profiles pass 1,670 tests with zero skipped tests. Clippy and formatting pass.

This is a test-boundary improvement. The concrete multiprocess adapter only succeeds with `Received`. This work does not establish a reachable public runtime panic.

Strict Octet still denies acceptance: 6,789 warnings and 309 critical findings. Full Nix still rejects six lifecycle references. Molten completion, lifecycle acceptance, integration, deployment, and production readiness remain unestablished.

## Source and owner

Parent commit: `2465a5e8480e718a0169221a50a097143ea0c802`.
Final checked index: `c4c6612e5a6dbc64d66f221c0952e756743d21fc`.
Branch: `completion/content-test-errors-20260914`.
Worktree: `/home/brittonr/.local/share/molten-completion/implementation-content-errors`.

Only these files differ before evidence packaging:

- `README.md`
- `tests/content_replication.rs`
- `tests/content_replication/outcomes.rs`

Molten's content-replication owner maintains the helper and its tests. The existing integration suite is the consumer. Repeatability comes from the exact outcome cases, real adapter assertions, standard test commands, and source bindings.

No production source, core, public API, port, protocol, dependency, toolchain pin, runtime profile, accepted specification, or lifecycle task changed. The primary checkout and published Raft worktree remain separate.

## Behavior and evidence limits

`received` returns the complete `Received` envelope unchanged. It exhaustively rejects `Cancelled`, `Uncertain`, `Unavailable`, and `TimedOut`. Errors retain the variant and debug detail in `MoltenError::InvalidHarness`. Cases cover ordinary, empty, escaped, and Unicode detail.

`run_action` propagates workspace, adapter-open, and fetch errors. `verify_envelope` propagates the existing offline verifier's error. The successful integration tests still use the real two-process adapter.

All original success assertions remain: action kind, operation identity, content identity, encoded length, repair target, protected form, verification decision, parent receipt, verification receipt, and one completed call. The process timeout remains `DEFAULT_DISTINCT_PROCESS_TIMEOUT_MS`.

The setup negative asserts the exact `Io` translation for an empty workspace label. Open and fetch negatives assert exact errors and unchanged manifest/action inputs. The invalid-operation adapter case asserts zero completed calls and no run directory. Source order places that rejection before transport execution. The counter alone does not count process starts.

The offline negative uses an absent directory in an owned workspace. A direct `read_dir` observation supplies the exact native I/O error. The verifier helper must return that same error. The test also checks the unchanged envelope and absent directory. It does not exercise every verifier failure or prove freedom from filesystem races.

The unit envelope's receipt references are test data. They do not represent a live transport receipt or grant authority. Existing live tests separately assert real receipt bindings.

Fallible fixtures and happy-path tests return `Result`. Rust's test runner treats `Err` as failure. Negative tests require exact errors, and a panic also fails the test. No assertion or rejection became a fallback or successful result.

Fixture indexing, existing assertions, and the post-fetch operation-prefix invariant remain. String construction allocates. This work does not establish complete helper panic freedom or allocation-failure safety.

## Executed sequence

| Task | Source or check | Direct result |
|---|---|---|
| 312 | Original baseline attempt | 124, before tests during Cargo prerequisite build |
| 323 | Separate cold prerequisite | 0, reviewed Cargo package rebuilt with 20 schema tests and one doctest |
| 326 | Unchanged integration baseline | 0, 12 passed |
| 382 | Old panic behavior behind extracted fallible helpers | 101, 14 passed and seven failed |
| 385 | First green | 0, 21 passed |
| 390 | First-green package-scoped nextest | 0, 1,669 passed and zero skipped |
| 391 | First-green package-scoped CI nextest | 0, 1,669 passed and zero skipped |
| 392 | First-green configured Octet | 0, warning-only with 6,798 warnings |
| 405 | Bounded read-only review | 0, identified the missing offline-error case |
| 425 | Offline `expect` mutation check | 101, 21 passed and one failed |
| 435 | Final focused suite | 0, 22 passed |
| 438 | Final configured Octet | 0, warning-only with 6,789 warnings |
| 439 | Bounded follow-up source review | 0, no further defect in the requested scope |
| 393 | Final workspace/all-target Clippy | 0, `-D warnings` |
| 399 | Final standard nextest | 0, 1,670 passed and zero skipped |
| 400 | Final standard CI nextest | 0, 1,670 passed and zero skipped |
| 407 | Complete production-source corpus | 0, exact replay inventory matches |
| 408 | Explicit-policy Cairn validation | 0, 56 changes and no reported issues |
| 409 | Full Nix | 1, six dangling lifecycle references |
| 410 | Final format and whitespace | 0 |
| 414 | Public Octet artifact import | 0, six artifacts imported |
| 415 | Public strict Octet gate | 1, deny |

The original red tests cover all four unexpected outcomes and workspace, open, and fetch errors. The original success tests still passed. The extraction changed private signatures but retained their old panic behavior. It did not expose a public runtime defect.

The first review found an unprotected offline-error path. Task 425 deliberately restored `expect` at that boundary. The new test failed with `offline multiprocess verification: Io("No such file or directory (os error 2)")`. This is mutation evidence, not a claim that the first green source contained that panic.

The follow-up also removed repair-induced `expect` findings through fallible test returns and direct error assertions. Final Octet reports no findings in `tests/content_replication/outcomes.rs`. No suppression or policy change was used.

The first nextest pair included `-p molten`. The final pair uses the unchanged standard package selection. The failed attempt to stop queued task 391 is retained in `queue-correction.md`. No task was force-started, and the serial budget remained unchanged.

The coordinator paused the serial group before follow-up edits. Its running first-green Octet command finished before source changes. The queued final checks began only after the final source and configured artifacts were ready.

## Final acceptance boundary

Strict receipt: `blake3:09f64e02c2eff3d818fb1b65b8866a8fb603ba4b2feadada138dfc6622811fab`.
Import receipt: `blake3:438cf2f4a255af6737c12e9dd213e2ea07611accce111b7acd61afa22100b35f`.

The strict receipt denies only `strict-status-clean` and `no-critical-findings`. Artifact presence, schemas, scope, fingerprint, current metadata, and linkage checks pass. There is no finding baseline or review-reference override.

The critical families remain 216 collection-growth findings, 76 unwrap findings, and 17 ambient-clock findings. The configured catalog has no `no_panic` entry. This is a source-policy observation, not proof of whole-system panic freedom.

The corpus contains 1,368 input paths, 13,711 objects, and 1,171 object-bearing paths. Its object set remains `b3:9f2fa8a57ecee48e32835536c050ee25ed42cb6ca5086c420a7d819d36addb13`. Production source did not change. Exact replay arguments match the complete sorted `src` inventory. Object identity does not prove behavioral correctness or resolved dependencies and effects.

Cairn uses the admitted immutable root `/nix/store/69f8rrsv266dvpamj90b87wspv93832c-source`. Policy selection is explicit, with installation receipt false. Structural validation does not complete lifecycle acceptance.

Full Nix passes 13 Tracey guard tests and then reports:

```text
requirements=2781
referenced=791
uncovered=1990
baseline_entries=1924
dangling=6
error: dangling traceability references are not permitted
```

The log also retains cache timeouts and unavailable-store-input diagnostics. These do not erase the executed Tracey rejection. The six F12/ChaosControl references, inherited debt, and required broader feature, target, VM, fault, and release evidence remain unresolved. No accepted specification or baseline was changed to pass a gate.

## Reproduction

Ordinary checks use eight-minute deadlines, two Cargo jobs, two test threads, one Nix job, and two cores. The separate cold prerequisite had a 24-minute limit. It rebuilt the same reviewed Cargo derivation. No active deadline was extended.

```sh
export CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2
export CARGO_TARGET_DIR=/tmp/molten-completion-20260913-target
nix develop --no-write-lock-file --builders '' --max-jobs 1 --cores 2 -c cargo test --locked -p molten --test content_replication
nix develop --no-write-lock-file --builders '' --max-jobs 1 --cores 2 -c cargo nextest run --locked --test-threads 2
nix develop --no-write-lock-file --builders '' --max-jobs 1 --cores 2 -c cargo nextest run --locked --test-threads 2 --profile ci
nix develop --no-write-lock-file --builders '' --max-jobs 1 --cores 2 -c cargo clippy --locked --workspace --all-targets -- -D warnings
nix flake check --no-write-lock-file --builders '' --max-jobs 1 --cores 2 -L
```

Task envelopes retain the full bounded commands and output paths. The archive retains original, red, first-green, mutation, and final source, logs, numeric exits, both Octet artifact sets, the final ledger, and reviews. Historical evidence is not rebound to this source.

The actual worktree CI JUnit file was copied and compared. Its file BLAKE3 is `3dc6bd61fa898880e73c82ff7b84f222393dd66cee92c852d3f50b41f28b455a`. The textual strict-receipt file hash is `576182a89fdc1a17a0e6a36c516cd6d94005009ec962398103f578a7c968d8d5`. These file hashes differ from semantic receipt identities by design.

Task audits compare the ID key, command marker, path, group, terminal state, and actual result. All inspected test and gate envelopes match. Tasks 312, 382, 425, 409, and 415 remain failures. A deliberately wrong command marker rejects. A valid task envelope does not override a failed gate.

The reviews used bounded read-only workers. They did not run tests or grant lifecycle approval. A separate next-owner review had unverified catalog attribution and did not select the next implementation.

`inputs.b3` binds the selected build/configuration files, README, all Rust files under `src crates tests tools`, and the ten retained Raft wire fixtures. `payloads.b3` binds this report, the input manifest, and the experiment archive. Commit and publication observations remain separate from this frozen experiment.
