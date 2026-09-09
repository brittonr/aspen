# Prolly Storage Work Reduction Specification

## ADDED Requirements

### Requirement: Operation-level storage measurements

r[molten.prolly.metrics] The Prolly service observations MUST report operation-level
measurements distinguishing blocks read, blocks decoded, subtrees not traversed,
transactions opened, and bytes copied. The existing `skipped_equal_nodes` MUST be
split into a structural `shared_node_count` and behavioral avoided-traversal counts.

#### Scenario: Baseline is recorded before optimization

- GIVEN the rebuild-first implementation within the standard profile cap
- WHEN the measurement baseline is captured
- THEN the baseline MUST record the counter values for edit, diff, and
  snapshot-load operations before any optimization merges.

#### Scenario: Sharing is no longer reported as avoided work

- GIVEN a diff over snapshots with shared subtrees
- WHEN the observations are read
- THEN `shared_node_count` and the avoided-traversal counts MUST be separate values.

### Requirement: Bounded read sessions amortize transactions

r[molten.prolly.readsession] `ProllyBlockStorePort` MUST expose an application-owned,
bounded read-session operation; the Redb adapter MAY hold one read transaction per
session. The pure core MUST observe only bounded block requests and closing
observations, and Redb types MUST NOT appear in `molten-core`. Sessions MUST carry
explicit lifetime and resource bounds, and operations that may wait on external
factors MUST use a canonical retention pin rather than holding a local transaction.

#### Scenario: One session serves a full snapshot load

- GIVEN a snapshot load that reads many blocks
- WHEN the load executes within one read session
- THEN the adapter MUST open at most one read transaction for the session and the
  measurements MUST show the reduction.

#### Scenario: Session misuse is rejected

- GIVEN a block read attempted after session close or beyond a resource bound
- WHEN the request reaches the port
- THEN the port MUST return a typed error without leaking the underlying
  transaction.

### Requirement: Diff skips validated subtrees with exact equality

r[molten.prolly.diff] The diff planner MUST compare child identities and key ranges
and skip matching validated subtrees, requesting blocks only for differing regions.
Its diff records MUST be exactly equal to the retained reference implementation for
every input, and equal-root and validation behaviors MUST be unchanged: a matching
root string MUST NOT confer validity on an untrusted snapshot.

#### Scenario: Mostly unchanged roots traverse less

- GIVEN two snapshots differing in a small region
- WHEN the demand-driven diff executes
- THEN the diff records MUST exactly equal the reference output and the
  avoided-traversal counts MUST exceed zero.

#### Scenario: Validation is not bypassed

- GIVEN an untrusted snapshot whose root string matches a known valid root
- WHEN the diff planner processes it
- THEN the planner MUST still require validation under the existing contracts before
  skipping or trusting any subtree.

### Requirement: Incremental edits preserve exact canonical roots

r[molten.prolly.incremental] The incremental edit planner MUST produce exactly the
same canonical root as the rebuild-first reference for the same profile and inputs,
including under content-defined boundary shifts. The planner MUST operate under
explicit work bounds with a correct rebuild fallback, and the rebuild-first
implementation MUST be retained as a reference oracle.

#### Scenario: Canonical equality holds across boundary shifts

- GIVEN adversarial edit sequences that shift content-defined boundaries at tiny
  node capacities
- WHEN `incremental_edit` and the rebuild reference both execute
- THEN their canonical roots MUST be exactly equal in every case.

#### Scenario: Work bounds fall back safely

- GIVEN an edit whose incremental work exceeds the configured bound
- WHEN the planner reaches the bound
- THEN the planner MUST complete via the rebuild fallback and still produce the
  exact canonical root.

### Requirement: Decoded-node cache is advisory and identity-bound

r[molten.prolly.cache] A decoded immutable-node cache MUST key entries by canonical
node reference, profile, and codec or validation version, MUST cache only the
verified immutable representation, and MUST keep retention advisory: changing
retention MUST NOT change canonical outputs, retry behavior, ordering, admission, or
emitted evidence. Cached node contents MUST never be mutated under an existing
content reference.

#### Scenario: Retention changes cannot alter outputs

- GIVEN two runs with different cache retention settings
- WHEN identical Prolly operations execute
- THEN canonical roots, diff records, and emitted evidence MUST be identical.

#### Scenario: Hot working set survives a scan

- GIVEN a large scan interleaved with a small hot working set
- WHEN the cache admits and evicts under capacity
- THEN the scan MUST be able to displace entries without allowing eviction paths to
  become untestable at the profile's tiny-capacity test settings.
