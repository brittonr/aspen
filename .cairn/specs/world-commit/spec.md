# Molten World Commit Specification

## Purpose

Define one immutable Molten identity for a coherent, typed, recoverable runtime snapshot without transferring authority or evidence ownership.

## Requirements

### Requirement: Molten world commits have canonical product-owned identity

r[molten.world_commit.core] Molten MUST define `molten-world-commit-v1` as canonical Preserves data. It MUST derive identity with domain-separated BLAKE3 framing over the complete immutable core.

#### Scenario: Equivalent worlds have one identity

- GIVEN two callers supply semantically equal version, parent, profile, and typed-root values in different input orders
- WHEN Molten canonicalizes and identifies both commits
- THEN both results MUST have identical canonical bytes and commit identities

#### Scenario: Behavior-relevant input changes

- GIVEN one parent, root identity, root type, profile, or version differs
- WHEN Molten identifies the changed commit
- THEN the resulting commit identity MUST differ

### Requirement: Every root retains its semantic domain

r[molten.world_commit.typed_roots] Molten MUST use distinct root-reference types for artifacts, schemas, durable state, tasks, history, effects, scheduler state, time, entropy, runtime profiles, policies, authority observations, and optional opaque machine snapshots. It MUST reject cross-domain substitution.

#### Scenario: Complete logical profile is admitted

- GIVEN a logical profile supplies every required typed root within declared bounds
- WHEN world-commit validation runs
- THEN validation MUST admit the typed root set for capture planning

#### Scenario: Artifact identity is supplied as task state

- GIVEN a caller places a valid artifact digest in the task-root field
- WHEN world-commit validation runs
- THEN validation MUST reject the reference as a domain mismatch

### Requirement: Capture publishes only a coherent observed cut

r[molten.world_commit.capture] Molten MUST publish a world commit only after every required immutable root is durable and every mutable revision fence still matches its captured observation. Drift, missing roots, incomplete inventories, or uncertain publication MUST NOT produce a success receipt.

#### Scenario: Every observation remains current

- GIVEN all required roots are durable and every rechecked revision equals its captured revision
- WHEN final commit publication succeeds
- THEN Molten MUST report one published world-commit identity and the exact observed fences

#### Scenario: Scheduler revision changes during capture

- GIVEN the scheduler root changes after initial observation
- WHEN the shell rechecks revision fences
- THEN publication MUST stop and the prior store state MUST NOT contain a successful commit record for that capture

### Requirement: Restore planning remains separate from runtime admission

r[molten.world_commit.restore] Molten MUST validate root closure and produce a deterministic restore plan without performing I/O. The shell MUST rerun current schema, artifact, policy, authority, resource, runtime, and effect admission before execution.

#### Scenario: Complete commit produces a restore plan

- GIVEN every required typed object exists and passes identity and schema checks
- WHEN restore planning runs
- THEN it MUST return a deterministic ordered adapter plan

#### Scenario: Commit is intact but current authority is absent

- GIVEN closure validation passes but current runtime authority admission denies
- WHEN the shell attempts activation
- THEN execution MUST remain denied even though commit integrity passed

### Requirement: Signatures and evidence remain detached

r[molten.world_commit.detached_evidence] Molten MUST exclude signatures, attestations, mutable head claims, currentness observations, and operator annotations from world-commit hash inputs. Detached envelopes MAY bind those facts to the commit identity.

#### Scenario: New evidence arrives

- GIVEN a valid commit receives an additional detached evidence envelope
- WHEN the evidence is stored
- THEN the world-commit identity MUST remain unchanged

#### Scenario: Attestation appears inside core bytes

- GIVEN candidate core data contains an embedded signature or attestation field
- WHEN canonical validation runs
- THEN the candidate MUST be rejected as an unsupported core schema

### Requirement: Verification covers success and denial boundaries

r[molten.world_commit.verification] Molten MUST test canonical identity, typed roots, coherent capture, closure, restore planning, opaque-profile boundaries, malformed inputs, stale observations, missing objects, domain confusion, secret disclosure, and overclaims.

#### Scenario: Focused world-commit rail runs

- GIVEN positive and negative fixtures use the reviewed schema and dependency cohort
- WHEN the focused verification rail runs
- THEN it MUST report all supported root profiles and bounded non-claims

## Executable-extent requirements

### Requirement: Executable extent dependencies are pinned and reviewed

r[molten.world_extents.dependency] Molten MUST consume the executable-extent mechanism and Mantle producer contract from immutable reviewed revisions. Adoption MUST verify package identity, license, API boundary, Octet gate, positive and negative vectors, and source parity before use.

#### Scenario: Dependency uses a sibling path

- GIVEN Molten configuration references an ambient sibling checkout instead of an immutable source
- WHEN dependency admission runs
- THEN adoption MUST fail before build or mapping

### Requirement: Code and mapping identities remain nominally distinct

r[molten.world_extents.identity_domains] Molten MUST use separate types for semantic code, built artifact, extent manifest, individual extent, and live mapping identities. A value from one domain MUST NOT substitute for another.

#### Scenario: Artifact digest is supplied as a mapping identity

- GIVEN a valid build artifact digest appears in a live mapping field
- WHEN extent admission runs
- THEN Molten MUST reject the value as an identity-domain mismatch

### Requirement: Extent world roots bind exact producer and runtime cohorts

r[molten.world_extents.profile] An extent code-root profile MUST bind the source artifact, Mantle bundle, target triple, executable format, ABI cohort, page profile, ordered extent descriptors, complete closure, runtime cohort, and policy identity.

#### Scenario: Complete extent profile is inspected

- GIVEN every bound object is available and exact profile facts match
- WHEN world closure validation runs
- THEN the extent root MAY enter mapping admission

#### Scenario: Ordinary artifact is supplied to an extent-required policy

- GIVEN bytes are a valid ordinary artifact but no admitted extent manifest exists
- WHEN extent-required admission runs
- THEN Molten MUST deny the profile without silent fallback

### Requirement: Mapping admission remeasures exact bytes and layout

r[molten.world_extents.admission] Molten MUST remeasure every extent and validate digest, length, offset, alignment, overlap, target, format, ABI, permission, page profile, and complete closure before mapping.

#### Scenario: Producer receipt passes but one extent changed

- GIVEN a valid producer receipt names bytes that no longer match the extent digest
- WHEN consumer remeasurement runs
- THEN Molten MUST reject mapping despite producer success

### Requirement: W^X is an explicit state transition

r[molten.world_extents.wx] Molten MUST use the shared pure W^X transition contract. No admitted state or transition MAY make the same extent writable and executable at the same time.

#### Scenario: Sealed extent becomes executable

- GIVEN exact bytes are sealed read-only and fresh read-back passes
- WHEN current runtime and execution authority admit activation
- THEN the shell MAY map the extent executable and read-only

#### Scenario: Adapter requests writable executable memory

- GIVEN a mapping request includes write and execute permission together
- WHEN mapping planning runs
- THEN Molten MUST reject the request before host mapping effects

### Requirement: Materialization and mapping are capability-relative

r[molten.world_extents.materialization] The shell MUST materialize, verify, seal, and map through admitted capability handles. It MUST NOT verify one path and reopen an ambient path for mapping.

#### Scenario: Path target is substituted after verification

- GIVEN an ambient path changes after bytes were verified
- WHEN mapping uses the already verified handle
- THEN the substituted path MUST NOT change the mapped object

### Requirement: Extent validity does not grant activation

r[molten.world_extents.activation] Molten MUST recheck current artifact, runtime, resource, policy, and execution authority before activation. Extent validity or possession MUST NOT grant execution.

#### Scenario: Valid extent lacks current execution authority

- GIVEN manifest and mapping admission pass but current policy denies execution
- WHEN activation runs
- THEN the extent MUST remain inert

### Requirement: Extent receipts preserve producer and consumer boundaries

r[molten.world_extents.receipts] Extent receipts MUST separately record producer bundle evidence, consumer remeasurement, mapping observations, activation admission, unmap, and non-claims. They MUST NOT claim build correctness, semantic equivalence, sandboxing, authority, retention, or release eligibility.

#### Scenario: Mapping receipt claims sandboxing

- GIVEN a receipt treats W^X mapping as proof of host sandboxing
- WHEN receipt validation runs
- THEN Molten MUST reject the overclaim

### Requirement: Extent verification covers hostile layout and authority cases

r[molten.world_extents.verification] Molten MUST test exact mappings, identity separation, misalignment, overlap, truncation, substitution, target and ABI mismatch, partial closure, W^X denial, missing authority, explicit fallback, and overclaims.

#### Scenario: Focused extent rail runs

- GIVEN positive and negative fixtures use reviewed producer and mechanism revisions
- WHEN extent verification runs
- THEN it MUST report every supported profile and bounded non-claim

## Branch-head requirements

### Requirement: Head claims bind exact branch transitions

r[molten.world_heads.claim] Molten MUST define canonical detached head claims that bind branch identity, expected head, successor head, expected generation, successor generation, purpose, policy identity, and signer observations.

#### Scenario: Claim advances one branch

- GIVEN a claim names the current branch head and the next admitted generation
- WHEN claim validation runs
- THEN it MUST bind exactly one proposed transition without changing either commit identity

#### Scenario: Claim omits the expected head

- GIVEN a signed claim names only a successor
- WHEN validation runs
- THEN Molten MUST reject the claim as insufficient for compare-and-swap publication

### Requirement: Head publication is generation-fenced compare-and-swap

r[molten.world_heads.cas] Molten MUST recheck the current head and generation inside the mutation boundary. It MUST atomically record an admitted successor or leave the prior head current.

#### Scenario: Current state matches the plan

- GIVEN the persisted head and generation match the admitted claim
- WHEN the local transaction commits
- THEN the successor MUST become current with one transition operation record

#### Scenario: Another writer advanced first

- GIVEN the persisted generation changed after planning
- WHEN the stale transition enters the mutation boundary
- THEN Molten MUST deny publication and report expected and observed state

### Requirement: Statement authentication does not grant branch authority

r[molten.world_heads.authentication] Molten MUST use Artifact Auth only to authenticate exact statement bytes under supplied observations. Current Basalt, UCAN, signer-role, threshold, and durable currentness policy MUST still authorize every mutation.

#### Scenario: Signature and authority both pass

- GIVEN statement authentication passes and current branch authority admits the signer set
- WHEN transition admission runs
- THEN the claim MAY proceed to compare-and-swap publication

#### Scenario: Signature passes but authority denies

- GIVEN a cryptographically valid statement lacks current branch-mutation authority
- WHEN transition admission runs
- THEN publication MUST remain denied

### Requirement: Competing claims remain explicit conflicts

r[molten.world_heads.conflicts] Molten MUST preserve bounded competing valid claims for the same expected head and generation. It MUST NOT select a winner by wall-clock time, arrival order, lexical identity, or last-writer-wins behavior.

#### Scenario: Two authorized successors compete

- GIVEN two valid claims target the same branch state with different successors
- WHEN conflict classification runs
- THEN Molten MUST return an explicit conflict set and block automatic advance

### Requirement: Durable generations reject stale claims relative to intact state

r[molten.world_heads.rollback] Molten MUST reject claims with old, repeated, skipped, or contradictory generations unless an explicit fenced recovery policy admits repair. It MUST classify this protection as relative to the observed durable generation state. It MUST NOT claim whole-store rollback detection without an independent currentness or witness observation.

#### Scenario: Old signed claim is replayed

- GIVEN a previously valid claim names a generation below the durable current generation
- WHEN the claim is presented again
- THEN Molten MUST reject it as stale or replayed relative to the observed current state

#### Scenario: Durable generation cannot be observed

- GIVEN the head store cannot prove its current generation
- WHEN mutation admission runs
- THEN Molten MUST deny ordinary head movement instead of resetting generation state

#### Scenario: Head and generation store are rolled back together

- GIVEN local head and generation state are both restored to an older valid image and no independent currentness observation exists
- WHEN the local head protocol evaluates the image
- THEN Molten MUST report whole-store rollback detection as unproven
- AND it MUST NOT issue a strong rollback-resistance receipt

### Requirement: Branch-head verification covers conflicts and authority failures

r[molten.world_heads.verification] Molten MUST test valid advances, merge ancestry, stale state, rollback, replay, signer failures, currentness failures, competing claims, uncertain persistence, and bounded non-claims.

#### Scenario: Focused branch-head rail runs

- GIVEN positive and negative fixtures use the reviewed Choregraph and Artifact Auth cohorts
- WHEN the branch-head verification rail runs
- THEN it MUST report the supported local transition boundary without claiming distributed consensus

## World diff and merge requirements

### Requirement: World diff reports every typed root conservatively

r[molten.world_merge.diff] Molten MUST compare typed world roots deterministically and classify each root as equal, changed, absent, unavailable, incompatible, or excluded by profile.

#### Scenario: Two commits differ in durable state

- GIVEN both commits have complete comparable roots and only durable state differs
- WHEN world diff runs
- THEN the report MUST identify the durable-state change and preserve equal classifications for unchanged roots

#### Scenario: Referenced object is unavailable

- GIVEN one root object cannot be loaded within the declared observation
- WHEN world diff runs
- THEN the report MUST classify that root as unavailable instead of equal or changed

### Requirement: Merge admission is typed and default-deny

r[molten.world_merge.admission] Molten MUST require an exact common ancestor, declared source heads, complete inputs, admitted schemas, and a closed merge mode. Tasks, scheduler state, authority state, effect attempts, external observations, and opaque machine snapshots MUST remain non-mergeable by default.

#### Scenario: One durable-value side changed

- GIVEN one side equals the base and the other side has a compatible durable-value root
- WHEN ancestor-replacement admission runs
- THEN the changed root MAY enter merge planning

#### Scenario: Candidate requests task-root merge

- GIVEN divergent task roots have no accepted root-specific merge profile
- WHEN merge admission runs
- THEN Molten MUST deny the merge before output publication

### Requirement: Merge handlers are exact pure bounded artifacts

r[molten.world_merge.handlers] Molten MUST bind application handlers by exact immutable behavior, schema, bound, and policy identities. Handlers MUST operate only on already-loaded canonical values and MUST NOT perform I/O, time, entropy, authority lookup, or effects.

#### Scenario: Exact handler returns a value

- GIVEN the handler identity and input schemas match the admitted profile
- WHEN the pure handler returns a bounded compatible value
- THEN the value MAY enter result planning

#### Scenario: Handler requests an external effect

- GIVEN a handler requires storage, network, clock, entropy, or authority access
- WHEN handler admission runs
- THEN Molten MUST reject it from the pure merge boundary

### Requirement: Unresolved conflicts are explicit durable results

r[molten.world_merge.conflicts] Molten MUST return deterministic typed conflict artifacts for unresolved semantic differences. It MUST NOT select winners through ordering, timestamps, or content identity.

#### Scenario: Both sides change one key differently

- GIVEN a keyed durable value differs from the base on both sides with incompatible values
- WHEN keyed merge runs
- THEN the result MUST contain one typed conflict and no merged root

### Requirement: Successful merge publishes one new causal commit

r[molten.world_merge.result] Molten MUST persist every result root before publishing a merge commit. The commit MUST name all declared source heads as parents, and unresolved conflicts MUST leave heads unchanged.

#### Scenario: Merge has no conflicts

- GIVEN every output root is durable and the result plan remains current
- WHEN merge publication succeeds
- THEN Molten MUST publish one new commit with the declared parent set

#### Scenario: Output publication fails

- GIVEN one merged root cannot be durably published
- WHEN merge publication runs
- THEN Molten MUST NOT publish a successful merge commit or move a branch head

### Requirement: Merge verification includes unsafe runtime roots

r[molten.world_merge.verification] Molten MUST test supported clean merges and negative ancestry, schema, migration, handler, conflict, bound, runtime-root, and partial-publication cases.

#### Scenario: Focused merge rail runs

- GIVEN positive and negative fixtures use reviewed history and schema cohorts
- WHEN the merge verification rail runs
- THEN it MUST report supported modes and all default-denied root classes
