# Following an artifact exchange

Mode: Walkthrough

This walkthrough follows the checked-in `local_iroh_publish_fetch_verifies_bundle_refs` test, not a hypothetical network deployment. The input is its small inline harness suite; the output is a locally stored canonical bundle plus publish and fetch receipt values. It is useful when reviewing what an exchange result actually establishes before handing an artifact to another subsystem. Return to the [Handbook](../README.md); use the [bounded closure companion](../../technical/replication/bounded-dag-closure-exchange.md) for graph theory rather than treating this bundle path as a general synchronization protocol.

**Execution status:** source-checked, not executed for this documentation batch. No terminal commands or live-peer setup are prescribed. The fixture uses a capability-rooted local store and an `iroh-local:` ticket. The transport name does not turn this example into a live Iroh transfer.

## 1. Identify the exact input

Open the [exchange test](../../../src/iroh/parts/exchange/tests/m000/p000/body.rs), starting at `local_iroh_publish_fetch_verifies_bundle_refs`. Its suite has one native actor, `a`, one grant to assert `ready`, and one assertion operation. The limits record is part of that concrete fixture, not recommended production sizing.

The test creates a temporary directory and opens `CapabilityExchangeRoot`. That root is the filesystem capability used by the exchange functions. Keep the distinction between this storage root and a trust root: opening a directory neither authenticates a peer nor authorizes execution of its artifacts.

**Observable boundary:** the suite parses into a Preserves value before the harness runs. An input parse failure means there is no run report or sealed bundle to exchange.

## 2. Follow report production and sealing

The fixture calls `run_suite_value`, then passes `run.report_value` to `sealed_repro_bundle_value_with_command`. The command vector containing `molten` is metadata supplied by this test, not an instruction to reproduce the whole example by invoking a bare executable.

Inspect these calls as separate stages. The report describes the harness run; sealing constructs the bundle subsequently given to exchange. Neither the existence of a report nor successful sealing proves that a remote system has the dependency closure needed for an unrelated workload.

**Observable boundary:** the test now holds a bundle value in memory. No publish ticket exists until the next stage returns.

## 3. Trace local publication

In [the exchange implementation](../../../src/iroh/parts/exchange/p000/body.rs), `publish_bundle_with_root` first obtains a reproduction verification receipt, computes the canonical bundle hash, and serializes canonical bytes. It separately computes a content reference from those bytes and rejects a mismatch with the bundle reference.

Only after those operations does it write the bytes through the capability root. It builds the ticket by prefixing the actual bundle reference with `iroh-local:`. Its returned `Repro` carries `ticket`, `bundle_ref`, and `receipt_value`; the publish receipt also refers to the verification receipt by its canonical hash.

**Observable boundary:** a successful return means this helper completed its local write and constructed evidence. It is not proof of remote possession, federation membership, installation permission, or indefinite retention.

## 4. Follow receiver verification

The fixture calls `fetch_bundle_with_root` using the same root, the returned ticket, and `Some(&published.bundle_ref)` as the expected identity. This is deliberately a local round trip.

Fetch checks the ticket prefix, compares the advertised reference with the optional expected reference, reads the corresponding blob, parses canonical bytes, and recomputes the bundle hash. It then obtains a reproduction verification receipt for the parsed bundle. The test compares the published and fetched bundle references.

The worked failure case immediately following the success path supplies an intentionally wrong expected reference. The mismatch is rejected before reading bundle bytes. That malformed fixture value is a negative test input, not a usable content address to copy into a recipe.

**Observable boundary:** equality of the two returned references establishes the fixture's identity comparison. It does not establish application authority.

## 5. Account for optional output effects

The concrete test sets both `out` and `ledger_root` to `None`. Therefore it does not demonstrate explicit output-file creation or ledger import. In the implementation, supplying `out` writes a textual Preserves representation; supplying `ledger_root` imports the bundle and verification receipt. The explicit output helper can create parent directories and write the selected file.

If reviewing a caller that enables those options, record their destinations separately and choose fresh isolated paths. The output write precedes ledger imports, so a later error must not be interpreted as proof that no earlier effect occurred. Do not blindly repeat an uncertain invocation against shared state.

## 6. Stop at the proven boundary

The neighboring chain-segment fixture exercises another path with anchors, checkpoints, fork policy, and destination indexing. It is not an implicit stage of this reproduction-bundle example. Likewise, [DAG synchronization](../../dag-sync.md) owns receiver-selected graph plans and fenced progress, not this ticket helper.

A useful handoff records the exact fixture, canonical bundle reference, enabled output options, and whether evidence was observed at runtime or only inspected in source. Keep receipts as evidence, never as new authority. Canonical Preserves plus BLAKE3 define artifact identity; Rust structure layout and peer labels do not.

## Sources

- [Handbook](../README.md)
- [Architecture: canonical identity and admitted effects](../../architecture.md)
- [Bounded DAG closure companion](../../technical/replication/bounded-dag-closure-exchange.md)
- [Concrete local exchange and chain fixtures](../../../src/iroh/parts/exchange/tests/m000/p000/body.rs)
- [Publish and fetch implementation](../../../src/iroh/parts/exchange/p000/body.rs)
- [Explicit output helper and include map](../../../src/iroh/exchange.rs)
- [Governing DAG synchronization contract](../../dag-sync.md)
