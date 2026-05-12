# redb-raft-kv-extraction Specification

## Purpose
Defines requirements for keeping the Redb Raft KV extraction stack reusable across type, storage, facade, and runtime adapter layers while isolating Aspen app compatibility shells.

## Requirements
### Requirement: Raft network adapter boundary is feature-gated

The Redb Raft KV extraction family MUST keep `aspen-raft-network` adapter dependencies behind explicit feature gates so reusable adapter checks can run without root Aspen app, handler bundles, cluster bootstrap, or unrelated runtime shells.

ID: redb-raft-kv-extraction.raft-network-adapter-boundary

#### Scenario: Minimal adapter graph compiles

- GIVEN `aspen-raft-network` with default or documented minimal adapter features
- WHEN `cargo check` and `cargo tree` run for that graph
- THEN the graph MUST compile and negative scans MUST show forbidden app/runtime crates are absent.

#### Scenario: Aspen runtime compatibility remains available

- GIVEN Aspen runtime consumers that need the full network adapter integration
- WHEN their documented feature bundles compile
- THEN compatibility evidence MUST show the feature-gated boundary did not break the node/runtime integration shell.
