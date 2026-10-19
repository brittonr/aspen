# Verification: offline cluster run-index intake

## Source and regression

The recovered F09/F10 catalog flags F628/F629 at the pre-append `src/cluster_harness/runner.rs:1004`. `cluster harness-verify --run-dir` reads an operator-supplied local index. Before the source fix, the two over-count and over-byte parser regression tests failed because both inputs were admitted. After it, `nix develop --command cargo test -p molten --lib run_index_rejects` passed two exact/count-boundary tests, accepting 4,096 entries and exactly 4 MiB but denying 4,097 entries or 4 MiB+1 byte. The direct real CLI denied each oversized file with its expected admission error; `nix develop --command cargo test --test cliharness cli_cluster_harness_offline_verifier_rejects_oversized_index -- --nocapture` passed. The existing checked two-node run and real offline verification test `cli_cluster_harness_executes_checked_fixture_and_verifies_offline` passed, proving ordinary positive verification was preserved.

## Focused quality checks and limits

Owner `nix develop --command cargo clippy -p molten --all-targets -- -D warnings`, `cargo fmt --package molten --check`, and `cargo fmt --package molten-core --check` passed. Scoped Octet library and binary catalogs at `/tmp/molten-run-index-owner-{lib,bin}-20260929` report warning-only, 1,512 and 1,558 findings, zero errors, and zero autofixable findings; neither flags the bounded index append. These scopes do not subtract from the full 3,615-row catalog. Direct standalone `rustfmt --check` on the included legacy p018 body reports unrelated pre-existing style differences; package cargo fmt passes. No network-facing admission or production-evidence claim follows from this local harness CLI result.

## Fixed-field parser allocation follow-up

Review found that `line.split('\t').collect::<Vec<_>>()` allocated a temporary
field vector for every admitted index row. The parser now takes precisely four
borrowed fields from the split iterator and checks a fifth without collecting.
Blank lines remain skipped; empty tab-separated fields remain present; a
missing or fifth field is denied before the entry-count check on the same row.
`nix develop --command cargo test -p molten --lib run_index_ -- --nocapture`
passed all three isolated parser tests, including missing/fifth/empty fields
and malformed row 4,098 before count denial. The checked two-node positive
CLI and oversized-index negative CLI passed again in the isolated owner.
The same three parser tests and both real CLI tests passed in composition
with an independent `CARGO_TARGET_DIR` to avoid shared-target ambiguity.
Owner `cargo fmt --package molten --check` and all-target Clippy passed.
The refreshed `/tmp/molten-run-index-fixed-fields-lib-20260929` Octet scope
is warning-only with 1,512 findings and zero errors/autofixable findings;
no parser warning site appeared. This is a per-row allocation repair, not a
changed intake limit or broader strict-ci result.

## Cairn structural and advisory checks

- `cairn validate --root . --policy cairn-policy/generated/cairn-policy.json --strict`: exit 0, 56 changes, empty issues/change/spec/profile arrays, receipt `5a2b66dad82f3f18b3f731b3811d6e34b554c1173bff4dae0369b7ba82643349`.
- `cairn gate proposal bound-offline-cluster-run-index --root . --policy cairn-policy/generated/cairn-policy.json`: `valid: true`, no issues, advisory, receipt `59c3c77eae0c554d3ecbc626dbf47ea3a309b9b4aa0dc682cc4285e579fcc4bc`.
- `cairn gate design bound-offline-cluster-run-index --root . --policy cairn-policy/generated/cairn-policy.json`: `valid: true`, no issues, advisory, receipt `618e2fb380209e2fbcc534378fed17b91e0e50020da51f8ad6e622eab5375d2c`.
- `cairn gate tasks bound-offline-cluster-run-index --root . --policy cairn-policy/generated/cairn-policy.json`: `valid: true`, no issues, advisory, receipt `b1caf0bb9e9086efff4437db5f47748c8628d9e1ccbcc411fcb0f3dccafe295e`.

Review-package validation is structural/advisory only. Accepted specs, unchanged full strict-ci, sync/archive, waiver, commit, and pin change were not performed.
