# Wasm integer-square-root evidence

## Result and owner

The private Wasm statistics helper now uses `u128::isqrt` from the unchanged compiler cohort. Its contract remains an unsigned floor square root.

Molten owns the comparison code and its tests. The durable contribution is a smaller arithmetic implementation with direct boundary and comparison checks. The Molten Wasm performance owner maintains this boundary.

This is a source-policy repair, not a demonstrated reachable-panic repair. The old divisor was a positive constant with a compile-time assertion. The old implementation passed all new contract tests before replacement.

Whole-Molten acceptance remains open. This report does not authorize lifecycle mutation, mainline integration, deployment, or worktree removal.

## Source and authority

- Parent: `ba1d203280df4c645f8e88914d78d052ba310ba1`.
- Branch: `completion/wasm-integer-sqrt-20260914`.
- Worktree: `/home/brittonr/.local/share/molten-completion/implementation-wasm-sqrt`.
- Checked tree before evidence packaging: `4b68efa3678841dd4e9696f27c6e1d9627ef570a`.
- External experiment: `/home/brittonr/git/OnixResearch/aspen/.pi/molten-completion/wasm-integer-sqrt`.

Task 583 fetched `origin`, created the branch from `origin/molten`, and retained the published parent by fast-forward. It verified the original primary HEAD, staged ZIP/audit identities, and clean parent worktree.

Only these source paths changed:

```text
README.md
src/wasm/performance/comparison.rs
src/wasm/performance/comparison/roots.rs
src/wasm/performance/tests/comparison.rs
src/wasm/performance/tests/comparison/exact.rs
```

The helper stays private. No public API, dependency, port, threshold, schema, error policy, runtime profile, or accepted specification changed. The arithmetic remains pure. Existing shells still own effects.

The parent catalog records `F6275` and `F6276` at `src/wasm/performance/comparison.rs:500`. These are two observations of one `expect` call. Finding identifiers are local to that catalog, not stable cross-run identities.

The pinned compiler supplies the reused primitive. A targeted Trellis search found no square-root component. No sibling source copy or new shared component forms part of this repair.

## Contract checks and mutation

Task 587 used `wasm::performance` and ran zero tests. Its zero exit is not baseline evidence. Task 598 corrected the filter to `wasm_performance` on unchanged source and passed 15 tests.

Task 594 added nine tests without replacing the old root implementation. All 24 tests passed. The pre-change source, patch, input hashes, and archive remain in the experiment.

The root tests cover zero, one, small nonsquares, square neighbors, bit boundaries, and full-width values. The oracle checks integer lower and upper square bounds. It also rejects deliberately incorrect and overflowing root candidates.

The comparison tests assert complete result fields and unchanged inputs. They cover constant samples, nonsquare variance, threshold equality, empty/singleton denials, zero-baseline denial, and candidate-overflow precedence.

The nonsquare fixture requires a confidence value of `1_131_606`. The expected identity uses the production `benchmark_comparison_ref` function. This proves identity consistency for these comparisons, not independent canonical-encoding correctness.

Task 606 deliberately changed the floor result to a ceiling result. It exited 101 with 20 passes and four intended assertion failures. Three root-boundary cases and the nonsquare confidence case rejected the mutation. The latter observed `1_131_607` instead of `1_131_606`.

Task 611 restored the floor primitive and passed all 24 tests. Task 697 passed the same 24 tests after the naming follow-up. That final run completed compilation in 4m 32s and tests in 0.05s within its original deadline.

The mutation is not evidence that the old source returned a ceiling root. These finite cases are not an exhaustive proof over every `u128` value. Fixture sample counts do not establish measured benchmark performance or statistical validity.

## Review and naming follow-up

Two read-only workers reviewed separate arithmetic and caller contracts. A third worker reviewed the resulting source and attempted to find a concrete counterexample. None found a scoped defect. None ran tests or granted lifecycle or release approval.

The final reviewer identified the shared-hasher and fixture-only evidence limits described above. The coordinator accepted those limits rather than broadening the claims.

Task 623 produced a first-green configured catalog with 6,786 warnings. It included three new test-name findings. Task 695 preserved that source and catalog before any name change.

The follow-up changed three function names and their call sites. It changed no function body, assertion, input, or product behavior. `naming-followup.md` records the exact boundary. The earlier catalog remains under `octet-first-green/` with its original command and result.

The existing file-length and module-file-count findings remain. Other production findings also remain outside this scope, including transport and lifecycle paths. The earlier lifecycle example was not a complete inventory.

## Final checks

| Task | Check | Actual result |
|---|---|---|
| 697 | Final focused suite | Exit 0, 24 passed |
| 700 | Configured Octet | Exit 0, warning-only, 6,783 warnings |
| 701 | Standard default nextest | Exit 0, 1,679 passed, zero skipped, 12 binaries |
| 702 | Standard CI nextest | Exit 0, 1,679 passed, zero skipped, 12 binaries |
| 703 | Workspace/all-target Clippy with `-D warnings` | Exit 0 |
| 704 | Explicit-policy Cairn validation | Exit 0, 56 changes, no issues |
| 705 | Complete production corpus | Exit 0, 1,370 inputs, 13,723 objects |
| 706 | Full Nix | Exit 1, six traceability references rejected |
| 707 | Formatting | Direct exit file 0, empty log, task envelope unavailable |
| 708 | Public Octet import | Exit 0, six artifacts imported |
| 709 | Public `strict-ci` gate | Exit 1, deny, 6,783 warnings and 307 critical findings |

The default nextest run recorded 3m 57s of compilation and 92.275s of tests. The CI run recorded 0.46s of compilation and 84.429s of tests. These durations describe these runs, not a benchmark comparison.

The actual CI JUnit copy matches `target/nextest/ci/junit.xml`. Its BLAKE3 file hash is `dee1a591a60b1ef8b6d20ef5df1c64029a4d5de80c15e5cb693137e471d3bc48`.

The final configured catalog has no findings in the two new test files. It has no `no_unwrap` observation in the changed comparison helper. The catalog still contains 74 unwrap, 216 collection-growth, and 17 ambient-clock observations. These counts do not establish whole-system panic freedom.

The complete corpus includes all 1,370 Rust files under `src`. Its 13,723 objects span 1,173 object-bearing paths. The replay command exactly matches the sorted input inventory. Its object-set hash is `b3:5a48dc419b1e40153373cdfe4b613e48996cc4936c881165aa8f485f04c3b048`.

The public import receipt is `blake3:55ab29e1f1c1601dc863d6c46b3e8c9369a911aa28f52c1300b097f54f2030f1`. Import proves neither behavioral correctness nor strict acceptance.

The public strict receipt is `blake3:c9251047840a0ca2efeafd40353495e5b963ae56e243f9a7ed116b9b9bedd280`. Only `strict-status-clean` and `no-critical-findings` fail. Artifact presence, source scope, current metadata, and linkage checks pass. The receipt has no baseline and no review references.

The strict-receipt file hash is `344ccb87e235902b6a92dff75d36dbc312a9680028daec393768813076aff264`. This byte identity differs from the semantic receipt identity by design.

Full Nix passed 13 Tracey guard tests before it rejected these references:

```text
molten.audit_f12.bounds
molten.audit_f12.compatibility
molten.audit_f12.saturation
molten.audit_f12.validation
molten.consensus.chaoscontrol_chain_observation
molten.consensus.chaoscontrol_operation_identity
```

The executed guard reports 2,781 requirements, 791 referenced, 1,990 uncovered, 1,924 baseline entries, and six dangling references. Cache timeouts and unavailable-input diagnostics also remain in the full Nix log. They do not erase the executed Tracey rejection.

Cairn selected the immutable canonical policy explicitly. Its installation receipt is false. Structural validation does not complete the open F12, Cargo, or ChaosControl changes.

Strict acceptance, full Nix, lifecycle completion, broader feature/target coverage, VM/fault evidence, and release readiness remain open.

## Evidence and reproduction

Task 699 froze the five source paths and wrote the checked tree and input manifest. The manifest binds repository configuration, every Rust source under `src`, `crates`, `tests`, and `tools`, and existing Raft wire fixtures.

`experiment.tar.gz` retains original baseline, pre-change, mutation, first-green, and final observations. The archive uses sorted members and deterministic gzip. Packaging refuses existing output paths and requires a complete tar comparison.

The final test, Clippy, Cairn, corpus, import, and strict/Nix task exports match their expected identities and actual results. The deliberately incorrect task marker rejects. A matched failed task remains a failure.

Tasks 707 and 724 lack their expected task envelopes at export. Their preserved files contain `{}`. Task 707 has a direct zero exit and empty formatting log. Task 724 was a receipt-preservation step with a direct successful tool observation. The cause of the missing queue records is unknown.

No historical command was repeated to create a replacement receipt. The source hashes, actual gate receipts, failed exports, and direct results remain intact. `receipt-limits.md` records this evidence limit.

`inputs.b3` binds the checked source. `payloads.b3` binds this report, the input manifest, and the deterministic archive. Commit and publication observations remain separate from the frozen experiment.

The standard reproduction commands are:

```sh
export CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2
export CARGO_TARGET_DIR=/tmp/molten-completion-20260913-target
nix develop --no-write-lock-file --builders '' --max-jobs 1 --cores 2 -c cargo test --locked -p molten --lib wasm_performance
nix develop --no-write-lock-file --builders '' --max-jobs 1 --cores 2 -c cargo nextest run --locked --test-threads 2
nix develop --no-write-lock-file --builders '' --max-jobs 1 --cores 2 -c cargo nextest run --locked --test-threads 2 --profile ci
nix develop --no-write-lock-file --builders '' --max-jobs 1 --cores 2 -c cargo clippy --locked --workspace --all-targets -- -D warnings
nix flake check --no-write-lock-file --builders '' --max-jobs 1 --cores 2 -L
```

Each recorded ordinary check has an eight-minute deadline. Commit and publication limits are twelve minutes. Worker limits are five minutes with bounded read-only tools. No active deadline extension, wrapper bypass, profile change, suppression, or dependency change is permitted.
