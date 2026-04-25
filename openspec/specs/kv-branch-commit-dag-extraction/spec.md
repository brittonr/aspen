# kv-branch-commit-dag-extraction Specification

## Purpose
TBD - created by archiving change extract-kv-branch-commit-dag. Update Purpose after archive.
## Requirements
### Requirement: Commit DAG avoids Raft compatibility dependencies
`aspen-commit-dag` MUST expose and use its commit-hash helper surface without depending on `aspen-raft`, root `aspen`, handler crates, binary shells, concrete transport crates, trust, secrets, SQL, or cluster bootstrap crates in its reusable feature sets.

ID: kv-branch-commit-dag-extraction.commit-dag-avoids-raft-compat

#### Scenario: Hash helpers are owned by the reusable family
ID: kv-branch-commit-dag-extraction.commit-dag-avoids-raft-compat.hash-helpers-owned-by-family

- **GIVEN** `aspen-commit-dag` computes commit IDs, mutations hashes, branch-tip keys, and fork integrity checks
- **WHEN** the crate is built with its reusable feature set
- **THEN** the hash helper types and functions it uses SHALL be reachable from `aspen-commit-dag` or a documented leaf helper crate
- **AND** source imports from `aspen_raft::verified` SHALL be absent

#### Scenario: Commit DAG dependency graph excludes app shells
ID: kv-branch-commit-dag-extraction.commit-dag-avoids-raft-compat.commit-dag-graph-excludes-app-shells

- **WHEN** `cargo tree -p aspen-commit-dag --edges normal` is checked
- **THEN** the graph SHALL exclude `aspen-raft`, root `aspen`, `aspen-core-shell`, `aspen-cluster`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-transport`, `aspen-raft-network`, `iroh`, `iroh-base`, `irpc`, `aspen-trust`, `aspen-secrets`, and `aspen-sql`

### Requirement: KV branch feature topology stays explicit
`aspen-kv-branch` MUST keep copy-on-write branch overlay behavior in the default reusable feature set and commit-history integration behind a named `commit-dag` feature.

ID: kv-branch-commit-dag-extraction.kv-branch-feature-topology

#### Scenario: Default branch overlay avoids commit DAG
ID: kv-branch-commit-dag-extraction.kv-branch-feature-topology.default-avoids-commit-dag

- **WHEN** `cargo tree -p aspen-kv-branch --no-default-features --edges normal` and `cargo tree -p aspen-kv-branch --edges normal` are checked
- **THEN** neither graph SHALL include `aspen-commit-dag` unless the `commit-dag` feature is explicitly enabled
- **AND** both graphs SHALL exclude root Aspen app/runtime shells and concrete transport crates

#### Scenario: Commit DAG feature composes only reusable crates
ID: kv-branch-commit-dag-extraction.kv-branch-feature-topology.commit-dag-feature-is-reusable

- **WHEN** `cargo tree -p aspen-kv-branch --features commit-dag --edges normal` is checked
- **THEN** the graph MAY include `aspen-commit-dag`
- **AND** the graph SHALL still exclude `aspen-raft`, root `aspen`, handler crates, binary shells, concrete transport crates, trust, secrets, and SQL crates

### Requirement: Downstream consumer proof covers branch and DAG APIs
A downstream-style fixture MUST prove that an external consumer can use the canonical branch overlay and commit DAG APIs without depending on root Aspen, `aspen-raft`, compatibility re-exports, handlers, binaries, or concrete transport.

ID: kv-branch-commit-dag-extraction.downstream-consumer-proof

#### Scenario: Fixture uses canonical crates directly
ID: kv-branch-commit-dag-extraction.downstream-consumer-proof.fixture-uses-canonical-crates

- **GIVEN** a fixture manifest outside the workspace root package
- **WHEN** `cargo metadata` and `cargo check` are run for that fixture
- **THEN** the fixture SHALL depend directly on `aspen-kv-branch` and `aspen-commit-dag`
- **AND** it SHALL exercise at least one branch overlay API and one commit DAG API
- **AND** metadata SHALL show no dependency on root `aspen`, `aspen-raft`, handlers, binaries, or transport crates

### Requirement: Extraction inventory tracks branch/DAG family
The crate extraction inventory and policy MUST include `aspen-kv-branch` and `aspen-commit-dag` with readiness state, class, owner, feature contract, dependency exceptions, representative consumers, and next action.

ID: kv-branch-commit-dag-extraction.inventory-and-policy

#### Scenario: Manifest and checker policy cover both crates
ID: kv-branch-commit-dag-extraction.inventory-and-policy.manifest-and-policy-cover-family

- **WHEN** `docs/crate-extraction.md`, the branch/DAG manifest, and `docs/crate-extraction/policy.ncl` are read
- **THEN** both `aspen-kv-branch` and `aspen-commit-dag` SHALL be present
- **AND** their readiness state SHALL not exceed `extraction-ready-in-workspace` until human license/publication policy is resolved
- **AND** `scripts/check-crate-extraction-readiness.rs --candidate-family kv-branch-commit-dag` SHALL verify the documented boundary
