# Molten World Commit Specification Delta

## Purpose

Add a bounded, history-independent Prolly map for keyed semantic-state roots without transferring world policy into storage.

## ADDED Requirements

### Requirement: Prolly profiles bind every structural decision

r[molten.prolly_map.profile] Molten MUST define a typed Prolly profile that binds key and value codecs, comparison, node format, boundary domain and seed, encoded-size accounting, minimum, target, and forced maximum chunk bounds, fanout bounds, BLAKE3 domains, and format version. Every node and root MUST bind the profile identity.

#### Scenario: Exact profile is supplied

- GIVEN all structural fields are canonical and within policy bounds
- WHEN profile validation runs
- THEN Molten MUST derive one stable domain-separated BLAKE3 profile identity

#### Scenario: Boundary seed is omitted

- GIVEN a candidate profile omits or changes one identity-relevant structural field
- WHEN profile validation runs
- THEN Molten MUST reject the profile before node admission

### Requirement: Nodes are canonical ordered immutable values

r[molten.prolly_map.canonical_nodes] Leaf and internal nodes MUST use canonical encodings, sorted unique key ranges, explicit child identities, encoded byte lengths, and domain-separated BLAKE3 identities. Unknown versions, malformed ranges, duplicate keys, and noncanonical encodings MUST fail closed.

#### Scenario: Canonical node is admitted

- GIVEN a node encoding, profile identity, key order, ranges, lengths, and child identities are valid
- WHEN node validation runs
- THEN Molten MUST admit the immutable node under its canonical identity

#### Scenario: Child ranges overlap

- GIVEN two internal-node children claim overlapping or unsorted key ranges
- WHEN node validation runs
- THEN Molten MUST reject the node

### Requirement: Boundaries are deterministic and byte-bounded

r[molten.prolly_map.boundaries] Boundary decisions MUST use canonical key bytes, current encoded size, and the versioned profile. Normal splits MUST be size-aware. A forced maximum encoded byte bound MUST terminate every admitted node even under chosen keys or variable-size values.

#### Scenario: Ordinary key stream reaches a natural split

- GIVEN canonical keys advance the size-aware boundary function within profile bounds
- WHEN node construction runs
- THEN it MUST split at the deterministic natural boundary

#### Scenario: Chosen keys avoid natural splits

- GIVEN an adversarial key sequence avoids all normal split decisions
- WHEN encoded size reaches the forced maximum
- THEN construction MUST split or reject before exceeding the bound

### Requirement: Equal canonical maps have history-independent roots

r[molten.prolly_map.history_independence] Any admitted insert, update, delete, batch, replay, or compaction history that yields the same canonical ordered key-value map under one profile MUST yield the same Prolly root identity.

#### Scenario: Different histories converge

- GIVEN two admitted mutation sequences produce the same canonical entries
- WHEN each sequence completes under one profile
- THEN both results MUST have the same root identity and canonical closure

#### Scenario: Compaction changes the root without changing state

- GIVEN compaction receives one valid map and preserves every semantic entry
- WHEN it proposes a different canonical root under the same profile
- THEN Molten MUST classify the result as a determinism defect

### Requirement: Storage effects remain outside the map core

r[molten.prolly_map.storage_boundary] The map core MUST consume supplied nodes and return read, edit, publication, closure, and GC plans. It MUST NOT read files, open databases, perform CAS, commit transactions, inspect clocks, or delete blocks. Shell adapters MUST execute only admitted plans and report observations explicitly.

#### Scenario: Edit plan publishes successfully

- GIVEN the core returns a valid staged block and root publication plan
- WHEN the shell completes storage and compare-and-advance effects
- THEN the core MAY validate the observed identities and terminal outcome

#### Scenario: Commit outcome is unknown

- GIVEN storage loses acknowledgement after mutation may have committed
- WHEN reconciliation runs
- THEN the shell MUST inspect durable state and MUST NOT guess success or failure

### Requirement: Diff is complete and merge-policy neutral

r[molten.prolly_map.diff] Prolly diff MUST report all bounded added, removed, and modified entries. It MAY skip equal canonical subtrees under the declared BLAKE3 assumption. It MUST NOT decide semantic mergeability or move branch heads.

#### Scenario: One localized value changes

- GIVEN two valid maps differ in one bounded key range
- WHEN diff runs
- THEN it MUST report the changed entry and MAY skip equal subtrees outside that range

#### Scenario: Adapter tries to resolve a conflict

- GIVEN a diff contains concurrent semantic changes
- WHEN the map core classifies the result
- THEN it MUST return differences without selecting a merge winner

### Requirement: Reachability is separate from deletion authority

r[molten.prolly_map.retention] The core MUST derive reachability from supplied roots, pins, and complete graph facts. An incomplete graph MUST produce an incomplete plan that cannot authorize deletion. The shell MUST revalidate retention and generation facts before destructive effects.

#### Scenario: Complete graph has unreachable nodes

- GIVEN all live roots, pins, and graph edges are complete
- WHEN reachability planning runs
- THEN it MAY identify candidate-unreachable node identities

#### Scenario: One child edge is unknown

- GIVEN the observed graph is incomplete
- WHEN GC planning runs
- THEN the plan MUST deny deletion authority for affected candidates

### Requirement: Proof and differential claims remain bounded

r[molten.prolly_map.proof_boundary] Trellis obligations, property tests, model tests, and DoltLite differential observations MUST state their exact assumptions and scopes. None of these artifacts MAY claim collision impossibility, whole-database correctness, authority, effect safety, GC execution safety, or release eligibility.

#### Scenario: Local proof obligation passes

- GIVEN one reviewed proof establishes sorted insertion under its explicit model
- WHEN evidence is reported
- THEN the claim MUST remain limited to that obligation and model

### Requirement: Benchmarks control extraction

r[molten.prolly_map.benchmark] Molten MUST measure sharing, changed-region diff work, retained bytes, restart behavior, GC planning, and adversarial boundary behavior under named profiles. Raw timings and counts MUST remain measurements.

#### Scenario: Pilot meets local targets

- GIVEN the Molten profile passes its bounded benchmark targets
- WHEN classification runs
- THEN the result MAY support local adoption without proving general performance

### Requirement: Extraction needs two matching consumers

r[molten.prolly_map.extraction] Molten MUST keep the pilot product-local unless at least two credible consumers require the same semantics, authority boundary, and evidence contract.

#### Scenario: Only Choregraph history appears similar

- GIVEN another repository uses immutable history but does not require the same ordered-map contract
- WHEN extraction is evaluated
- THEN Molten MUST NOT count that repository as a matching consumer

### Requirement: Verification covers success and failure paths

r[molten.prolly_map.verification] Verification MUST cover canonical reads, edits, history independence, structural sharing, diff, restart, closure, retention, malformed nodes, wrong profiles, adversarial boundaries, missing children, tamper, stale publication, unknown outcomes, incomplete graphs, active pins, and policy overreach.

#### Scenario: Negative corpus is incomplete

- GIVEN required malformed, adversarial, crash, retention, or overclaim fixtures are absent
- WHEN verification coverage is evaluated
- THEN the change MUST remain incomplete

### Requirement: Differential evidence does not imply format parity

r[molten.prolly_map.differential] Differential cases MAY compare normalized semantic observations with the pinned DoltLite oracle. They MUST NOT require cross-format root equality or treat agreement as correctness proof.

#### Scenario: Oracle and map agree

- GIVEN both adapters produce equal normalized observations for one case
- WHEN differential evidence is emitted
- THEN it MUST report bounded agreement for that exact case and cohorts
