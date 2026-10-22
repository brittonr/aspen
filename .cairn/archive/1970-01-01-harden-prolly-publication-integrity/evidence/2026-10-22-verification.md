# Harden Prolly publication integrity: verification

## What changed

- `crates/molten-core/src/prolly_map/failures.rs`: the pure `classify_port_failure` classifier. It takes the phase
  (validation, staging, publication, readback, deletion) and the failure kind (read, codec, validation, write,
  commit) and returns the outcome class: a definite rejection, not applied, or unknown. A commit failure on a mutating
  phase is unknown, because the transaction may already have become durable.
- `src/prolly_map/store.rs`: the Redb adapter classifies its failures through the classifier instead of hand-assigning
  `outcome_unknown = false`. A commit failure on staging, publication, or deletion now reports an unknown outcome.
- `src/prolly_map/storeretention.rs`: the retention boundary of the local store. The store keeps a pin table and a
  retention generation, and the compare-and-delete operation compares the current durable roots, the current pins, and
  the retention generation against the admission inventory inside the same write transaction that deletes the
  candidates. A mismatch returns a typed outcome and deletes nothing.
- `src/prolly_map/ports.rs`: the port traits are split. The block store port keeps the read, stage, and publish
  operations; the retention port owns the pins, the inventory, and the compare-and-delete.
- `src/prolly_map/service.rs`: the publication path pins the blocks it stages before it stages them and releases the pin
  once the publication reaches a terminal outcome. A staging failure with an unknown outcome is resolved by
  re-staging the content-addressed blocks, so the staging ambiguity stays separate from the head ambiguity. The
  reconciliation reads the durable root back and keeps the outcome unknown when the readback fails, is corrupt, or shows
  a conflicting head. The garbage collection deletes through the compare-and-delete operation.
- `src/prolly_map/tests/faults.rs` and `src/prolly_map/tests/support.rs`: the fault fixtures and the memory port's
  fault modes.
- `docs/prolly-semantic-state-map.md`: the phase-aware classification, the readback classes, and the indivisible
  deletion boundary.

## Validation

- `cargo test -p molten --lib prolly`: 11 passed, including the four new fault fixtures.
- `cargo test -p molten-core --lib prolly`: 18 passed, including the four classifier fixtures.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo test --workspace` with a fresh `CARGO_TARGET_DIR`: exit 0. The molten library suite reported 1,702
  passed, the core suite 501 passed, the native system-extension integration suite 9 passed, and the doc-test phase
  passed.
- `cargo octet check --artifact-dir target/octet-precommit`: `Status: clean`, 0 findings, 0 warnings, 0 errors.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0 with no findings, `spec_findings: []` and
  `substance_findings: []`.
- `cargo nextest` cannot run in this checkout: `cargo-nextest` always calls `cargo metadata --all-features`, and that
  call panics in the pinned cargo on the optional `rad://` sources. The panic reproduces on the unmodified tree and is
  unrelated to this change, which touches no manifest. The documented Cargo fallback was used.

## Deferred scope

No third-party fault-injection dependency was adopted. The fixtures inject the faults through the repository's own
memory port, so there is no new dependency, provenance, or license record.

## Non-claims

- An unknown outcome is not a success. The publication stays unknown until a readback resolves it, and the resolution
  is one of applied, not applied, or unknown.
- The compare-and-delete boundary compares the durable roots, the pins, and the retention generation. It does not
  prove the reachability of every candidate: the planner computes the reachability, and the adapter refuses a stale
  admission.
- A refused deletion is not a data-loss event and a completed deletion is not a proof that the content was
  unreachable in any other store.
