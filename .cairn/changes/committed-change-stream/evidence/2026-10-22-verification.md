# Durable committed-change stream and publication eligibility: verification

## What changed

- `crates/molten-core/src/committed_stream/`: the pure committed-change stream. A commit advances the state reference,
  the events, and the stream position together. Each event carries a stable identity over the service, epoch,
  revision, ordinal, and payload reference, and references its payload by content identity. A subscription is bounded;
  a cursor behind the retained window reports an explicit retention gap, a cursor from a prior epoch is rejected, and a
  cursor for another service is refused. The snapshot-plus-watermark operation returns the position after the last
  committed event. Replayed events deduplicate by stable identity.
- `crates/molten-core/src/content_replication/eligibility.rs`: the publication eligibility gate. The requirement is the
  profile's replica policy. The receipt binds the content identity, the operation, the receiver set, the retention
  generation, the placement epoch, and the storage guarantee kind, and carries a canonical identity over them. A local
  guarantee never satisfies a profile that requires replicated availability across fault domains, so a replica count
  alone never satisfies the gate.
- `src/content_replication/service/eligibility.rs`: the shell gate between the transfers and the metadata publication.
  The receipts join the published evidence and the reconcile outcome. Content the profile cannot admit receives no
  receipt and stays reported as under replicated.
- `src/content_replication/service/{mod,facts,repair}.rs`: the reconcile shell split into planning, fact observation,
  repair projection, and publication steps so each stays inside the Octet function and file budgets. Effect order is
  unchanged.
- `docs/architecture.md` and `docs/content-replication.md`: the stream surface, its non-claims, and the gate.
- The spec delta gains `molten.observations.cursors` (the cursor requirement previously reused the stream ID, which
  Cairn's delta merge rejects as a conflicting operation) and `molten.observations.tests`.

## Validation

- `cargo test -p molten-core --lib committed_stream content_replication`: 51 passed, including the eight stream
  fixtures and the five gate fixtures.
- `cargo test -p molten --lib content_replication`: 33 passed, including the shell gate fixtures.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo octet check --artifact-dir target/octet-precommit`: `Status: clean`, 0 findings.
- `cargo test --workspace` with a fresh `CARGO_TARGET_DIR`: exit 0. The molten library suite reported 1,704 passed,
  the core suite 514 passed, the native system-extension integration suite 9 passed, and the doc-test phase passed.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0, `spec_findings: []` and
  `substance_findings: []`.
- Inherited-debt guard: 2,911 requirements, 977 referenced, 1,934 uncovered matching the reviewed baseline, 0 dangling,
  verdict pass.
- `cargo nextest` still cannot run in this checkout: `cargo metadata --all-features` panics in the pinned cargo on the
  optional `rad://` sources, reproduced on the unmodified tree. The documented Cargo fallback was used.

## Deferred scope

The fault-conformance phase for a subscriber restart after publication but before acknowledgement stays open. The
world-fault conformance profile exercises world reconciliation and has no committed-change subscriber role yet. The
resume fixture covers the subscriber-restart semantics at the stream boundary.

## Non-claims

- The stream orders transitions within one service and epoch. It claims no global order across services.
- The stream never implies atomicity across independent stores.
- Possessing an event, a cursor, or a stream position authorizes nothing; content reads still require the current
  capability checks.
- An eligibility receipt proves the observed storage guarantee met the profile requirement at publication. It does not
  prove permanent durability or global availability.
