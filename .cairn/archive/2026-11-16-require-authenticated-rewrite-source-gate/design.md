## Context

`octet_gate::validate_octet_source_gate` checks receipt content and workspace freshness, not producer authenticity, so a fabricated `octet-gate-receipt-v1` with the right shape passes it. Node startup already authenticates its gate: `node/source-binding` reads the operator-selected `source-gate-trust.json` from a `DirectoryRoot`, authenticates the selected Nix output with the trusted key, measures the executing binary, admits the binding and validates the produced gate. The rewrite upgrade-plan hook and local dogfood still used the fabricated receipt.

## Decisions

### Decision: Compile the fabricated receipt only into tests

**Choice:** `synthetic_clean_octet_gate_receipt_for_tests` is `#[cfg(any(test, feature = "test-fixtures"))]`. The `test-fixtures` feature is enabled only by the self dev-dependency `molten = { path = ".", features = ["test-fixtures"] }` in the existing `[dev-dependencies]` table, so binary tests that call it through the library still compile while `cargo build`/`cargo check --lib --bins` exclude it.

**Rationale:** The compiler, not review, now lists every non-test caller; removing the three production callers was required for the library to build. Cargo enables dev-dependency features only when building test targets.

### Decision: Upgrade plans take an authenticated gate token and explicit refs

**Choice:** `apply_with_upgrade_plan(root, rewrite_input, &RewriteUpgradePlanInput { session_id, initiator_ref, capability_refs, policy_refs, source_gate })`, where `source_gate` is a `node_daemon::AuthenticatedGate`. Its fields are private; only `authenticated_gate_with_root` (the startup `load` path, measuring the executing binary) and a `cfg(test)` fixture construct it. The combined entrypoint validates explicit refs and strict current-workspace freshness before calling `apply`; the post-apply plan builder is private. The plan binds the gate receipt as its source-gate value and adds the binding and binding-validation refs to its evidence; `validate_upgrade_source_gates` still re-validates it for the final plan. The CLI requires `--source-state-root`, `--initiator-ref`, `--capability-ref` and `--policy-ref`, authenticates the source binding, then invokes this combined entrypoint. Admission refusal, including a structurally valid but stale produced gate or an unavailable current workspace, leaves neither a plan nor rewritten artifacts.

**Rationale:** Accepting caller-supplied gate files would only move the fabrication to the command line. Reusing startup admission keeps one authentication path. Explicit refs replace `default_local_ref(kind, session_id)` label hashes, which named no resolved policy or capability.

### Decision: Dogfood binds an unqualified fixture gate (option b)

**Choice:** Dogfood creates `octet_gate::UnqualifiedFixtureGate`, a crate-internal value labelled `octet-gate-fixture-v1` with `<evidence "unqualified-fixture">` and `<octet-run "none">`. It is not an `octet-gate-receipt-v1`, so `validate_octet_source_gate` (every consumer), `validate_produced_octet_gate_receipt`, upgrade-plan admission, strict job admission and scan-mode startup deny it. Dogfood no longer installs anything as registry kind `octet-gate-receipt`. Startup and loopback job admission gain `pub(crate)` fixture modes that bind only that exact value and its fixture validation ref; their receipts record `strict-octet-source-gate-bound` as `fail` and add `unqualified-fixture-source-gate`. The dogfood report and release gate carry `unqualified-fixture-source-gate`, and `molten dogfood local-node` prints `source-gate=unqualified-fixture`.

**Rationale:** Option (a), requiring authenticated source evidence, would make the `dogfood-local-node` Nix check unbuildable: the source-binding verifier runs the operator-selected `nix` against the store daemon, and neither that daemon nor a real Octet run is available in the build sandbox. The whole release-evidence chain in that check (bundle, promotion, export) would have to be dropped or moved into a VM. Option (b) keeps the check while making its source-gate status explicit. Real consumers cannot mistake the fixture for a strict gate, and dogfood receipts do not claim a strict gate. Only crate-internal code can construct the fixture or reach the fixture modes, not the CLI or other crates.

## Risks / Trade-offs

- The fixture modes are new code paths in node startup and job admission. They are `pub(crate)`, bind only the typed fixture, and mark their receipts. A crate-internal caller could still misuse them; current startup and job receipt consumers check check names, not statuses, so the `fail` status is a label rather than an enforced denial downstream.
- `rewrite apply --upgrade-plan-out` cannot run where no authenticated source binding exists, including in tests of the binary. The positive path is covered through the library with the `cfg(test)` source-binding fixture.
- unit2nix unifies the self dev-dependency's `test-fixtures` feature into the root crate's generated feature set. The owning flake override removes it and its Cargo feature metadata from non-test `molten` derivations. Normal and mutant test derivations request it explicitly, then derive `CARGO_CFG_FEATURE` from their actual `crateFeatures` through `overrideAttrs`; this avoids retaining the production derivation's empty metadata. Both plans were regenerated through the documented hash-first procedure. Capability subject observation includes this declared feature, so a test build cannot masquerade as the production feature set.
- A feature-independent guard (`scripts/check-test-fixture-callers.sh`, flake check `test-fixture-callers`) fails when the fabricated receipt is referenced outside its definition and test files (`tests/`, `src/**/tests/`, `src/**/tests.rs`). The `cfg(test)` source-binding fixture moved into `source-binding/tests.rs` so that the guard needs no exceptions.
