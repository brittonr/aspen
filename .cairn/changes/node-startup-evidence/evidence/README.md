# Verification-only implementation checkpoint

Base commit: `d3c108c90eca778a73ba725b98a6b5cef809fcc2`.

The first three tasks are complete. Real pinned-tool execution, source/binary association, startup integration, and VM replay remain open. This checkpoint is not an approved runtime cohort.

## Implemented and checked

- The pure core admits exact cohort identities and a closed ten-member read plan.
- The capability adapter measures the descriptor before member reads. It rejects unknown fields, mismatched bytes, unapproved binaries, symlinks, special files, and excess sizes.
- The snapshot evaluator uses explicit source metadata and the existing strict gate formulas. It rejects synthetic toolchain markers, nonzero findings, suppression, stale context, and replay-command-only coverage.
- `node startup-evidence verify` emits a read-only report. Both execution and startup authority remain false. No production or consumer guard changed.
- The positive snapshot test also passed from `/tmp`, outside the source workspace.

Final focused results: 5 core tests, 36 Octet tests (29 existing plus 7 new), 5 adapter tests, 2 CLI tests, and 4 existing startup/content guard tests. Total: 52. Core all-target Clippy and selected root/CLI Clippy passed with `-D warnings`.

The Nickel contract accepts its structural fixture and rejects the synthetic-toolchain fixture. These placeholder hashes approve no real bundle. Stable rustfmt reported unsupported nightly options. Full pinned formatting and Octet acceptance are not claimed.

## Tool observations

The desktop has a working `nightly-2026-05-26` compiler. Its reported commit is `31a9463c6e2794a59ce57a8f37abc6966afc2a58`. Both pinned Octet source inputs declare `nightly-2026-03-21`. Build and lint compilers have distinct roles.

The installed worker/desktop wrapper resolves to `/nix/store/sh2ny8qjyx8vrdmn7b9mh4zl3b17sawx-cargo-octet-0.1.0/bin/cargo-octet`. Its recorded engine derivation uses source `/nix/store/fy1chsqgzmsrqvwr9bvxyp3r2gsqnpni-source`.

The engine source file differs from pinned Octet `fc38f59330b626961d166febfdf1a5aa6575460f`:

- Pinned `cargo-octet/src/engine.rs`: `2369ac8f5d19140de821a5652c55cd34e06140214dd497350a062c4cd9b58d26`.
- Installed derivation source file: `19f682fa5ef9a049cf17b80eb9caa924c390c09c2d157f83596455927cf44c07`.

This disproves treating the default wrapper's recorded source as that pinned cohort. It does not establish absence of every matching tool elsewhere. No compiler or Octet tool was installed, replaced, or rebuilt. Molten alone was compiled with the existing Rust 1.97.1 for focused checks.

## Failures retained

The first baseline command omitted systemd's working directory and found no Cargo manifest. Its corrected baseline passed 29 tests. Initial new-code compilation exposed a module path, an index type inference error, error conversion, and an unavailable direct `tempfile` import. The fixes use explicit types and the existing capability-backed test workspace. No manifest or lockfile changed.

## Limits and review

Review was single-agent and correlated. Positive model data establishes verifier behavior, not real Octet execution or source provenance. The approved source inventory is an explicit operator expectation. This verifier does not independently discover the whole repository or prove compiler/executor correctness.

The adapter assumes operator-owned roots. It does not promise hostile-parent protection, a filesystem transaction, or power-loss durability. No new VM, native replay, host blob listener, Stage0, package rebuild, physical deployment, release, or default promotion occurred.

Cairn gates use the unchanged repository policy. Explicit `--policy` and automatic selection produced the same task receipt. Structural gates do not complete the two remaining tasks. No sync, archive, or main-branch integration is authorized by this checkpoint.
