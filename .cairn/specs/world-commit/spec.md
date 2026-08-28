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

### Requirement: World closure uses typed bounded DAG synchronization

r[molten.world_distribution.closure] Molten MUST project world commits and typed root edges into the generic DAG-sync boundary. Received objects MUST match requested identities, domains, bounds, and closure before activation.

#### Scenario: Complete closure is received

- GIVEN a peer supplies every requested commit and typed root object within declared bounds
- WHEN closure validation runs
- THEN Molten MUST report a complete immutable closure for local admission

#### Scenario: Peer substitutes one domain

- GIVEN bytes match a digest but the object domain differs from the requested root type
- WHEN closure validation runs
- THEN Molten MUST reject the object and keep the closure incomplete

### Requirement: Head claims replicate separately from immutable content

r[molten.world_distribution.head_claims] Molten MUST exchange detached signed head claims separately from world objects. Local authentication, current policy, authority, ancestry, and conflict rules MUST decide whether any claim can affect a local head.

#### Scenario: Remote claim is locally authorized

- GIVEN the statement authenticates and current local branch policy admits its transition
- WHEN local claim admission runs
- THEN the claim MAY enter local head-transition planning

#### Scenario: Two remote claims compete

- GIVEN two admissible claims name different successors for the same expected state
- WHEN local claim admission runs
- THEN Molten MUST preserve a conflict set and MUST NOT select by arrival order

### Requirement: Retention roots cover recovery and unresolved work

r[molten.world_distribution.retention_roots] Molten MUST include current and competing heads, executions, task checkpoints, replay and simulation pins, merge conflicts, promotion and reconciliation state, rollback and evidence holds, remote leases, and incomplete transfers in world retention projection.

#### Scenario: Promotion outcome is unresolved

- GIVEN a promoted effect has an uncertain external observation
- WHEN retention roots are projected
- THEN the related commit, intent, reservation, attempt, and observation objects MUST remain reachable

#### Scenario: Active execution root is missing

- GIVEN an execution profile may retain a world root but provides no current observation
- WHEN retention projection runs
- THEN the inventory MUST remain incomplete

### Requirement: Reachability does not grant deletion authority

r[molten.world_distribution.gc_boundary] World reachability and retirement reports MUST remain evidence inputs. Existing retention policy, remote clearance, tombstone, apply, execute, audit, and destructive authority gates MUST run before deletion.

#### Scenario: Commit is unreachable but legally held

- GIVEN complete reachability finds no active runtime path but retention policy records a legal hold
- WHEN garbage collection is requested
- THEN Molten MUST retain the content

#### Scenario: Remote state is unknown

- GIVEN a required peer observation is stale or unavailable
- WHEN deletion planning runs
- THEN Molten MUST retain affected content or report an unresolved blocker

### Requirement: Partial synchronization is resumable and non-authoritative

r[molten.world_distribution.partial] Molten MUST record partial sync progress under one operation identity. Resume MUST revalidate prior objects, requested root, peer policy, and remaining closure. Partial receipt MUST NOT grant activation or availability claims.

#### Scenario: Transfer stops after some roots

- GIVEN a bounded transfer ends before closure completion
- WHEN the operation is inspected
- THEN Molten MUST report exact durable progress and block activation

#### Scenario: Resume observes a changed request

- GIVEN the requested root or peer policy differs from the saved operation
- WHEN resume planning runs
- THEN Molten MUST reject reuse of the stale operation

### Requirement: Distribution verification covers authority and retention denial

r[molten.world_distribution.verification] Molten MUST test complete and partial transfer, corruption, domain confusion, bounds, claim conflicts, stale authority, retention omissions, remote uncertainty, and destructive overclaims.

#### Scenario: Focused distribution rail runs

- GIVEN positive and negative fixtures use reviewed DAG, replication, binding, and retention cohorts
- WHEN the focused verification rail runs
- THEN it MUST report closure and retention facts without claiming convergence or permanent durability

### Requirement: Execution snapshots use closed profile classes

r[molten.world_snapshot.profiles] Molten MUST distinguish logical and opaque execution-snapshot profiles. Each profile MUST declare required roots, ownership, completeness, compatibility, restore, replay, retention, and merge behavior. Unknown profiles MUST deny.

#### Scenario: Logical profile is complete

- GIVEN every declared Molten logical root is present under one profile
- WHEN profile validation runs
- THEN Molten MUST classify the profile by its logical restore contract

#### Scenario: Unknown profile is supplied

- GIVEN a descriptor names an unsupported profile class
- WHEN validation runs
- THEN Molten MUST reject it instead of treating it as logical or opaque

### Requirement: Logical profiles bind resumable Molten state

r[molten.world_snapshot.logical] A logical profile MUST bind exact durable-state, task, history, scheduler, virtual-time, entropy, effect-state, runtime, schema, and policy roots required by its declared resume boundary.

#### Scenario: Required task root is absent

- GIVEN a logical profile declares resumable tasks but omits the task root
- WHEN completeness validation runs
- THEN Molten MUST classify the snapshot as incomplete

### Requirement: Opaque profiles require exact machine completeness

r[molten.world_snapshot.opaque] An opaque profile MUST bind an exact machine-snapshot descriptor and complete CPU, memory, device, disk, backend, topology, and runtime cohort facts. Missing or incompatible facts MUST block restore.

#### Scenario: Exact ChaosControl cohort matches

- GIVEN the snapshot inventory and restore host observations match the admitted exact cohort
- WHEN opaque compatibility runs
- THEN Molten MAY produce an opaque restore plan

#### Scenario: CPU state inventory differs

- GIVEN the restore host lacks one required CPU state group or feature inventory entry
- WHEN compatibility runs
- THEN Molten MUST deny restore without filling defaults

### Requirement: Compatibility binds behavior-relevant cohorts

r[molten.world_snapshot.cohort] Molten MUST compare exact architecture, runtime, ABI, schema, topology, device, storage, scheduler, time, entropy, and effect profile identities required by the selected snapshot class.

#### Scenario: Snapshot bytes match but runtime cohort differs

- GIVEN an object identity is valid but one required runtime behavior identity differs
- WHEN compatibility admission runs
- THEN Molten MUST reject the snapshot for that restore target

### Requirement: Copy-on-write children remain parent-bound and isolated

r[molten.world_snapshot.cow] Molten MUST use reviewed VM Cohort mechanics for opaque checkpoint clones. Each child MUST bind one parent and isolated memory, device, disk, and endpoint overlay identities.

#### Scenario: Child overlays are isolated

- GIVEN two children derive from one admitted checkpoint with distinct overlay identities
- WHEN clone validation runs
- THEN writes from either child MUST NOT be attributed to the other child or parent

#### Scenario: Overlay identity collides

- GIVEN a proposed child reuses an active sibling overlay identity
- WHEN clone admission runs
- THEN Molten MUST deny the clone before host effects

### Requirement: Restore recreates handles and rechecks authority

r[molten.world_snapshot.restore] Snapshot bytes MUST exclude live host handles and bearer authority. Restore MUST obtain new handles and recheck current policy, capability, revocation, resource, adapter, and runtime facts before activation.

#### Scenario: Snapshot is complete but authority expired

- GIVEN all deterministic state restores but current authority admission denies
- WHEN activation runs
- THEN Molten MUST keep the restored world inactive

#### Scenario: Descriptor contains a live handle

- GIVEN a candidate descriptor includes a file descriptor, socket handle, credential, or private key
- WHEN safe schema validation runs
- THEN Molten MUST reject or omit that field according to the closed schema

### Requirement: Snapshot verification covers completeness and overclaims

r[molten.world_snapshot.verification] Molten MUST test logical and opaque success paths plus incomplete state, cohort mismatch, unsafe handle, stale authority, isolation, merge denial, portability, and correctness overclaims.

#### Scenario: Focused snapshot rail runs

- GIVEN positive and negative fixtures use reviewed Molten, ChaosControl, and available VM Cohort cohorts
- WHEN the focused verification rail runs
- THEN it MUST report supported restore profiles and exact compatibility non-claims

### Requirement: Benchmark profiles are typed and content-bound

r[molten.world_bench.profile] Molten MUST define typed Nickel benchmark profiles that bind dataset, preparation, operation sequence, warm or cold state, profile class, adapters, bounds, repetitions, hardware cohort, and named acceptance thresholds. Rust MUST revalidate every projection before execution.

#### Scenario: Complete profile is repeated

- GIVEN the same validated profile, source revision, dataset, adapters, and preparation state
- WHEN two benchmark plans are identified
- THEN they MUST have the same canonical plan identity

#### Scenario: Threshold is unexplained

- GIVEN a profile contains a nontrivial numeric threshold without a named typed field
- WHEN profile validation runs
- THEN Molten MUST reject the profile

### Requirement: Structural sharing metrics use exact facts

r[molten.world_bench.metrics] Required results MUST record logical bytes, physical bytes written, new objects, reused objects, copied pages, mapped pages, traversed references, compared keys, emitted conflicts, transferred bytes, retained objects, and planned deletions where applicable. Missing required metrics MUST invalidate the result.

#### Scenario: Branch starts from unchanged roots

- GIVEN a branch is created without mutation
- WHEN branch cost is measured
- THEN the result MUST distinguish root-reference publication from copied bytes and new object materialization

#### Scenario: Physical bytes are reported as logical bytes

- GIVEN compression or deduplication makes physical and logical byte counts differ
- WHEN receipt validation runs
- THEN Molten MUST reject a result that collapses the two metrics

### Requirement: Preparation state is explicit

r[molten.world_bench.datasets] Dataset construction, prepopulation, cache warming, compaction, and prior object availability MUST be identified before measured operations. Unknown preparation state MUST block accepted comparison.

#### Scenario: Warm cache is undeclared

- GIVEN objects are available from an earlier run but the profile declares cold state
- WHEN benchmark admission runs
- THEN the result MUST be rejected as preparation drift

### Requirement: Snapshot profile results remain distinct

r[molten.world_bench.snapshot_profiles] Logical typed-state and opaque machine-snapshot benchmark cohorts MUST use distinct result classes. Molten MUST NOT claim semantic or performance equivalence across those classes.

#### Scenario: Opaque snapshot reports fewer written bytes

- GIVEN one opaque fixture writes fewer bytes than one logical fixture
- WHEN results are summarized
- THEN the summary MUST NOT claim the opaque profile is universally superior or semantically equivalent

### Requirement: Retention benchmarks do not grant deletion authority

r[molten.world_bench.retention] Retention measurements MAY cover reachability, pin evaluation, protected-object reuse, candidate classification, and deletion-plan size. The benchmark verdict MUST NOT authorize deletion.

#### Scenario: Reachable object enters a deletion plan

- GIVEN an object remains reachable from an admitted world, branch, witness, capsule, quarantine, or policy root
- WHEN retention correctness validation runs
- THEN the result MUST fail regardless of benchmark speed

### Requirement: Extraction decisions require accepted evidence

r[molten.world_bench.extraction_decision] A pure classifier MUST return retain-current, optimize-in-place, or evaluate-shared-component from accepted benchmark receipts and typed policy. It MUST require repeated product-neutral limits and at least two credible consumers before recommending shared-component evaluation.

#### Scenario: One Molten fixture misses a duration target

- GIVEN only one product-specific timing result misses policy
- WHEN extraction classification runs
- THEN it MUST NOT recommend automatic creation of a shared repository

### Requirement: Benchmark receipts preserve finite-run limits

r[molten.world_bench.receipt] Receipts MUST bind source revision, profile, dataset, preparation, adapters, hardware cohort, limits, exact metrics, duration observations, results, and unsupported rows. They MUST NOT claim asymptotic proof, universal performance, storage correctness, or release eligibility.

#### Scenario: Receipt claims big-O proof

- GIVEN a finite benchmark receipt claims it proved asymptotic complexity
- WHEN receipt validation runs
- THEN Molten MUST reject the overclaim

### Requirement: Benchmark verification covers misleading inputs

r[molten.world_bench.verification] Molten MUST test stable results, structural reuse, mutation accounting, profile separation, preparation drift, missing metrics, stale revisions, unsafe retention, and extraction overclaims.

#### Scenario: Focused benchmark rail runs

- GIVEN positive and negative fixtures use one reviewed cohort
- WHEN benchmark verification runs
- THEN it MUST report exact metric coverage and all bounded non-claims

### Requirement: Replay traces bind every expected world transition

r[molten.world_replay.transition_chain] Molten MUST define a canonical bounded transition trace that binds one initial world commit and an ordered sequence of transition inputs, deterministic profiles, expected parents, and expected successor commits.

#### Scenario: Every replayed step matches

- GIVEN a complete trace and closure for a supported logical profile
- WHEN Molten restores and replays each step
- THEN every actual successor commit MUST equal the expected successor before the next step runs

#### Scenario: One intermediate successor differs

- GIVEN a replay produces an unexpected commit before a later step
- WHEN transition verification compares the result
- THEN it MUST stop at the first mismatching step
- AND it MUST NOT report final-trace success

### Requirement: Replay divergence is complete-world and refs-only

r[molten.world_replay.divergence] Molten MUST compare expected and actual commit identities and typed roots. It MUST report the earliest differing step, root domain, and bounded field path without exposing secret bytes or bearer material.

#### Scenario: Entropy root diverges

- GIVEN all earlier steps match and one step produces a different entropy root
- WHEN divergence classification runs
- THEN the report MUST name that step and the entropy-root domain
- AND later differences MUST NOT replace the earliest result

### Requirement: Replay capsules bind complete typed closure

r[molten.world_replay.capsule] Molten MUST define a canonical capsule manifest over every required trace, commit, typed root, artifact, schema, policy, runtime cohort, snapshot descriptor, transition input, and content manifest. Each member MUST bind its role, identity, codec, and byte length.

#### Scenario: Complete capsule is exported

- GIVEN every reachable member is available and within declared bounds
- WHEN capsule planning runs
- THEN the resulting manifest MUST enumerate the complete typed closure with one stable identity

#### Scenario: Required schema is absent

- GIVEN one world root references a schema that is not in the capsule closure
- WHEN capsule validation runs
- THEN validation MUST fail before replay or import publication

### Requirement: Import validates before availability

r[molten.world_replay.import] Molten MUST validate capsule identity, canonical encodings, member bounds, complete closure, object identities, supported profiles, and protection policy before imported objects become available for restore or replay.

#### Scenario: Imported member is tampered

- GIVEN transport returns bytes that do not match the declared member identity
- WHEN import verification runs
- THEN Molten MUST reject the member and MUST NOT publish capsule availability

#### Scenario: Valid import completes

- GIVEN every member verifies and the complete closure passes
- WHEN import publication commits
- THEN Molten MAY report capsule availability without moving a branch or activating a runtime

### Requirement: Replay execution remains profile-bound and authority-neutral

r[molten.world_replay.execution_boundary] Molten MUST use the declared logical or opaque restore profile and MUST rerun current authority, artifact, schema, resource, runtime, and effect admission before execution. Capsule possession MUST NOT grant those admissions.

#### Scenario: Capsule is complete but authority is absent

- GIVEN capsule validation passes and current execution authority denies
- WHEN replay activation is requested
- THEN replay MUST remain denied

#### Scenario: Opaque profile targets a different cohort

- GIVEN an exact machine snapshot descriptor does not match the destination cohort
- WHEN replay planning runs
- THEN Molten MUST reject the profile without falling back to logical restore

### Requirement: Replay receipts preserve bounded claims

r[molten.world_replay.receipts] Replay and import receipts MUST bind trace, capsule, profile, horizon, closure, actual transitions, divergence, redaction, and dependency identities. They MUST NOT claim universal determinism, semantic equivalence, capability transfer, effect completion, or release eligibility.

#### Scenario: Focused replay rail runs

- GIVEN positive and negative fixtures use reviewed dependency cohorts
- WHEN the world replay verification rail runs
- THEN it MUST report exact supported profiles and all bounded non-claims

### Requirement: Replay verification covers success and denial paths

r[molten.world_replay.verification] Molten MUST test complete replay, stable identities, capsule round trips, first divergence, closure failures, malformed encodings, unsupported profiles, secret disclosure, missing authority, and import-overclaim cases.

#### Scenario: Negative corpus is incomplete

- GIVEN replay fixtures omit malformed, missing-closure, profile-mismatch, or authority-denial cases
- WHEN verification coverage is evaluated
- THEN the replay change MUST remain incomplete

### Requirement: Promotion plans bind one candidate and intent closure

r[molten.world_promotion.plan] Molten MUST bind each promotion plan to the expected active head, candidate commit, exact effect-intent closure, branch policy, current authority inputs, and one operation identity.

#### Scenario: Candidate intent closure is complete

- GIVEN every candidate intent has one exact semantic identity and admitted release classification
- WHEN promotion planning runs
- THEN it MUST produce a complete bounded reservation plan

#### Scenario: Candidate contains an unclassified intent

- GIVEN one intent lacks a release, deny, simulate, or retain classification
- WHEN promotion planning runs
- THEN Molten MUST deny promotion planning as incomplete

### Requirement: Active-head movement and release reservation are locally atomic

r[molten.world_promotion.transaction] Molten MUST update the active head and publish the complete release-reservation set in one local transaction. A failed or stale transaction MUST leave both prior states unchanged.

#### Scenario: Promotion transaction commits

- GIVEN expected head, authority, policy, intent closure, and reservation identities remain current
- WHEN the local transaction commits
- THEN the candidate MUST become active and every admitted reservation MUST be durable

#### Scenario: Reservation insertion fails

- GIVEN one required reservation cannot enter the transaction
- WHEN promotion publication runs
- THEN the active head MUST remain unchanged and no partial reservation set may become dispatchable

### Requirement: Dispatch consumes only committed current reservations

r[molten.world_promotion.dispatch] Molten MUST dispatch only from committed reservations after rechecking current capability, policy, semantic handler, adapter generation, and reservation ownership. Retries MUST reuse the same logical release identity.

#### Scenario: Reservation remains admitted

- GIVEN a committed reservation passes every current dispatch gate
- WHEN a dispatcher claims it
- THEN one attempt MAY run under the reservation identity

#### Scenario: Authority changed after promotion

- GIVEN promotion succeeded but current capability admission now denies
- WHEN dispatch admission runs
- THEN Molten MUST record a blocked reservation and MUST NOT execute the effect

### Requirement: Uncertain outcomes require observation-first reconciliation

r[molten.world_promotion.reconciliation] Molten MUST classify publication and effect observations as not published, published, unknown, or conflicting before retry decisions. Unknown or conflicting outcomes MUST NOT trigger a blind new logical operation.

#### Scenario: Local commit result is unknown

- GIVEN the transaction returned no reliable completion result
- WHEN recovery begins
- THEN Molten MUST observe the durable operation identity before planning any repeat mutation

#### Scenario: External acknowledgment was lost

- GIVEN an effect may have completed but no acknowledgment is durable
- WHEN reconciliation runs
- THEN Molten MUST retain an uncertain state unless an admitted observation resolves it

### Requirement: Acknowledged observations use explicit successor commits

r[molten.world_promotion.observation_commit] Molten MUST bind an acknowledged effect observation to one recorded-effect transition from the promoted candidate to an explicit successor. This transition MUST NOT mutate the promoted commit or grant dispatch authority.

#### Scenario: Acknowledged observation becomes recorded history

- GIVEN one acknowledged attempt has an exact reservation and observation identity
- WHEN observation-commit planning runs
- THEN the transition MUST bind the promoted candidate as parent, the observation as input, and the supplied successor

#### Scenario: Observation is unknown or mismatched

- GIVEN an attempt is unacknowledged, lacks an observation, names another reservation, or keeps the same successor
- WHEN observation-commit planning runs
- THEN Molten MUST deny the transition before history publication

### Requirement: Promotion receipts state bounded non-claims

r[molten.world_promotion.non_claims] Promotion, reservation, attempt, and reconciliation receipts MUST distinguish local eligibility, dispatch attempts, observations, and external completion. They MUST NOT claim generic exactly-once execution or atomic external effects.

#### Scenario: Promotion succeeded without dispatch

- GIVEN the active head and reservations committed but no dispatcher ran
- WHEN the promotion receipt is inspected
- THEN it MUST report eligibility without claiming any external effect occurred

### Requirement: Promotion verification covers crash windows

r[molten.world_promotion.verification] Molten MUST test pre-commit, uncertain-commit, post-commit, pre-dispatch, post-dispatch, lost-acknowledgment, duplicate, conflict, denial, and overclaim paths.

#### Scenario: Focused promotion rail runs

- GIVEN deterministic fake adapters cover every declared crash boundary
- WHEN the promotion verification rail runs
- THEN it MUST report local atomicity and external-effect non-claims

### Requirement: Molten consumes branch policy without transferring runtime authority

r[molten.world_branch_authority.adoption] Molten MUST consume one reviewed immutable Basalt world-branch authority policy cohort. Basalt decisions MUST remain supplied policy facts and MUST NOT mint, move, store, or enforce capabilities.

#### Scenario: Basalt admits a normalized branch action

- GIVEN the policy identity, normalized capability facts, and authority inputs are current
- WHEN Basalt returns an admitted branch mode
- THEN Molten MAY build a realization plan under its own runtime gates

#### Scenario: Policy receipt is presented as a capability

- GIVEN a passing Basalt receipt exists without current realization evidence
- WHEN branch activation runs
- THEN Molten MUST deny activation

### Requirement: Branch authority uses explicit derivation obligations

r[molten.world_branch_authority.derivation] Molten MUST support closed copyable, attenuated, linear, simulation-only, promotion-gated, replace-before-activation, and non-branchable modes. Each admitted mode MUST produce explicit obligations, and unknown modes MUST deny.

#### Scenario: Attenuated capability is requested

- GIVEN policy permits attenuation and names required limits
- WHEN planning runs
- THEN the plan MUST require a verifiably narrower destination grant

#### Scenario: Unknown branch mode appears

- GIVEN a policy output contains an unsupported mode
- WHEN Molten validates the output
- THEN it MUST deny instead of treating the mode as copyable

### Requirement: Linear authority cannot remain active on both branches

r[molten.world_branch_authority.linear] Molten MUST realize linear movement through generation-fenced durable transfer. Destination activation MUST require evidence that the source no longer owns active use.

#### Scenario: Linear transfer commits

- GIVEN the source owns the exact current generation and every transfer gate passes
- WHEN the durable transfer commits
- THEN source use MUST become unavailable before destination activation succeeds

#### Scenario: Source still appears active

- GIVEN current observations cannot prove source deactivation
- WHEN destination activation runs
- THEN Molten MUST deny activation as ambiguous ownership

### Requirement: Simulation authority cannot reach live adapters

r[molten.world_branch_authority.simulation] Molten MUST bind simulation-only grants to exact deterministic simulation adapters. Missing or failed simulation support MUST deny instead of falling back to live effects.

#### Scenario: Simulation adapter is available

- GIVEN an exact admitted simulation profile implements the requested capability
- WHEN branch activation runs in simulation mode
- THEN Molten MAY bind only that simulation adapter

#### Scenario: Simulation adapter is missing

- GIVEN a live adapter exists but no admitted simulation adapter exists
- WHEN branch activation runs
- THEN Molten MUST deny without invoking the live adapter

### Requirement: Authority is rechecked at activation and promotion

r[molten.world_branch_authority.activation] Molten MUST recheck current policy, UCAN, revocation, replay, scope, durable ownership, adapter, and effect facts at branch activation and promotion.

#### Scenario: Stored observation became stale

- GIVEN the world commit references an earlier passing authority observation
- WHEN current revocation or policy facts differ
- THEN activation MUST use current facts and deny when they no longer admit use

#### Scenario: Promotion reservation is complete and committed

- GIVEN the exact promotion plan has one complete committed reservation set and a selected reservation for the candidate
- WHEN promotion-gated activation admission runs
- THEN Molten MAY admit branch activation without authorizing effect dispatch

#### Scenario: Promotion observation authorizes dispatch

- GIVEN a promotion observation claims dispatch authority or has an incomplete, uncommitted, or crossed reservation
- WHEN promotion-gated activation admission runs
- THEN Molten MUST deny before branch activation

### Requirement: Branch-authority evidence excludes bearer material

r[molten.world_branch_authority.evidence] Branch plans and receipts MUST exclude bearer tokens, private keys, credentials, secret entropy, raw capability paths, and private policy bodies. They MUST state that decisions and observations do not prove future enforcement.

#### Scenario: Receipt projection receives a bearer token

- GIVEN shell inputs include private capability material
- WHEN safe receipt projection runs
- THEN the output MUST omit the material and retain only approved identities and bounded decisions

### Requirement: Branch-authority verification covers widening and escape

r[molten.world_branch_authority.verification] Molten MUST test every supported mode and negative widening, duplicate linear use, stale currentness, simulation escape, promotion bypass, secret disclosure, and enforcement overclaims.

#### Scenario: Focused branch-authority rail runs

- GIVEN positive and negative fixtures use the reviewed Basalt cohort
- WHEN the focused verification rail runs
- THEN it MUST report supported modes and current-enforcement non-claims

### Requirement: World operator requests use explicit typed facts

r[molten.world_operator.plan] Every world workflow request MUST name exact commit, branch, expected generation, profile, policy, limit, authority-observation, and operation identities required by the selected actions. Planning MUST remain pure and compute a domain-separated BLAKE3 plan identity.

#### Scenario: Complete request is planned

- GIVEN all required immutable and mutable observations are supplied within bounds
- WHEN workflow planning runs
- THEN it MUST return one deterministic ordered operation graph and plan identity

#### Scenario: Request asks for latest head implicitly

- GIVEN a mutating request omits the expected head or generation
- WHEN workflow planning runs
- THEN Molten MUST reject the request before shell effects

### Requirement: Mutating commands are preview-first

r[molten.world_operator.preview_apply] Molten MUST preview every checkpoint, branch, run, promotion, import-publication, and retention mutation. Apply MUST require the exact plan identity and MUST recheck mutable facts inside each owning mutation boundary.

#### Scenario: Preview remains current

- GIVEN an operator submits the exact plan and all mutable observations still match
- WHEN apply runs
- THEN each component MAY execute its admitted operation in dependency order

#### Scenario: Head changes after preview

- GIVEN the branch generation changed after planning
- WHEN apply admission runs
- THEN the workflow MUST stop before using the stale head plan

### Requirement: The CLI is a thin composition root

r[molten.world_operator.composition] The `molten world` command family MUST call existing world cores and application services. It MUST NOT duplicate capture, merge, authority, effect, replay, replication, retention, or snapshot domain logic in CLI dispatch.

#### Scenario: Command plans a promotion

- GIVEN a valid promotion request
- WHEN CLI dispatch runs
- THEN it MUST delegate policy and transition decisions to their owning cores
- AND it MUST retain effect execution in explicit shell adapters

### Requirement: Operator commands have a closed typed surface

r[molten.world_operator.commands] Molten MUST provide typed checkpoint, branch, run, diff, conflicts, replay, simulate, verify, promote, export, import, and garbage-collection planning requests. Raw command strings and ambient credential lookup MUST NOT enter the workflow core.

#### Scenario: Raw shell command is supplied

- GIVEN a workflow request contains executable command text instead of a declared operation descriptor
- WHEN request validation runs
- THEN Molten MUST reject it as an unsupported authority surface

### Requirement: Aggregate receipts preserve component claims

r[molten.world_operator.receipt] A world workflow receipt MUST link ordered component plans, receipts, completion states, and the first blocker. It MUST preserve each component's evidence role and MUST NOT convert aggregate completion into whole-stack correctness, authority, or release eligibility.

#### Scenario: Workflow stops after a replay divergence

- GIVEN earlier operations completed and replay finds a divergence
- WHEN the workflow stops
- THEN the aggregate receipt MUST link completed receipts and the divergence receipt
- AND it MUST NOT report later promotion or export actions as complete

### Requirement: Dogfood covers one complete logical slice

r[molten.world_operator.dogfood] Molten MUST dogfood checkpoint, attenuated branch creation, deterministic run, successor capture, diff, conflict inspection, replay, verification, promotion reservation, export, import, and retention planning in one bounded logical workflow. It MUST separately test one exact opaque restore and replay profile.

#### Scenario: Logical fixture completes

- GIVEN the admitted logical fixture and deterministic adapter cohort
- WHEN the dogfood workflow runs
- THEN every operation MUST produce its expected plan and receipt linkage

#### Scenario: Opaque fixture requests semantic merge

- GIVEN the exact opaque fixture has divergent machine snapshots
- WHEN the workflow requests semantic merge
- THEN Molten MUST reject the operation without falling back to logical merge

### Requirement: Diagnostics remain bounded and redacted

r[molten.world_operator.diagnostics] Machine and human summaries MUST name stable operation, profile, blocker, and receipt references. They MUST NOT emit private keys, bearer tokens, raw environment values, unbounded state, or implicit host paths.

#### Scenario: Authority denial includes secret input

- GIVEN an adapter observes a secret while authority admission denies
- WHEN the operator summary is rendered
- THEN the summary MUST expose only the bounded denial class and safe references

### Requirement: Operator verification covers success and failure

r[molten.world_operator.verification] Molten MUST test complete workflows, stable plans, stale observations, denied authority, unresolved conflicts, uncertainty, incomplete capsules, unavailable profiles, raw commands, secret disclosure, and garbage-collection overclaims.

#### Scenario: Focused operator rail runs

- GIVEN positive and negative fixtures use the reviewed dependency cohort
- WHEN operator verification runs
- THEN it MUST report each supported or blocked profile and its bounded non-claims

### Requirement: Every world mutation has a registered failure contract

r[molten.world_faults.inventory] Molten MUST maintain a closed versioned inventory for capture, head, promotion, witness, outbox, replication, import, retention, and garbage-collection mutations. Each row MUST name its owner, operation identity, expected pre-state, effects, linearization point, durable record, uncertain window, reconciliation entry, and required cases.

#### Scenario: New mutation is not registered

- GIVEN product code adds a world-related durable mutation absent from the inventory
- WHEN conformance coverage runs
- THEN the rail MUST fail before claiming complete mutation coverage

### Requirement: Fault profiles use named semantic phases

r[molten.world_faults.profile] Fault profiles MUST use bounded named operation phases and explicit schedules. They MUST bind profile, adapter, limit, source revision, and operation identities with BLAKE3. Source-line locations and wall-clock timing MUST NOT establish semantic phase or winner selection.

#### Scenario: Lost response follows possible submit

- GIVEN a profile interrupts after possible external submission but before response
- WHEN the harness records the observation
- THEN the result MUST enter the owning uncertain-outcome path

#### Scenario: Profile contains an unexplained numeric threshold

- GIVEN a fault profile uses a numeric limit without a named typed field and contract
- WHEN profile validation runs
- THEN Molten MUST reject the profile

### Requirement: The shell injects faults without owning decisions

r[molten.world_faults.shell_boundary] The imperative harness MAY control interruption, restart, storage, and schedules. It MUST pass observations to the owning pure cores and MUST NOT synthesize success, compensation, conflict, or recovery decisions.

#### Scenario: Adapter reports a successful write without durable read-back

- GIVEN the owning contract requires read-back and the shell has no matching observation
- WHEN conformance comparison runs
- THEN the harness MUST NOT mark the operation complete

### Requirement: Concurrency uses explicit fenced schedules

r[molten.world_faults.concurrency] Concurrent tests MUST bind operation IDs, expected generations, pre-state identities, and declared interleaving points. Competing valid transitions MUST preserve compare-and-swap, conflict, authority, and effect-release rules.

#### Scenario: Two promotions target one generation

- GIVEN two admitted promotion plans name the same expected head and different successors
- WHEN the deterministic schedule interleaves final publication
- THEN at most one transition MAY publish at that generation
- AND the other MUST become stale, superseded, or conflicting without duplicate effect release

### Requirement: Restart recovery remains conservative

r[molten.world_faults.recovery] Recovery MUST classify durable observations as already-complete, safe-to-retry, superseded, conflict, uncertain, denied, corrupt, or manual-review. Missing, contradictory, or incomplete state MUST NOT become success or cleanup authority.

#### Scenario: Commit outcome is ambiguous after restart

- GIVEN an operation record exists but required publication read-back is missing
- WHEN recovery classification runs
- THEN Molten MUST return uncertain or manual-review according to the owning contract

### Requirement: Interruption cases cover every mutation window

r[molten.world_faults.interruption] Each registered mutation MUST have positive uninterrupted coverage and negative before-submit, after-possible-submit, after-durable-write, lost-response, restart, and recovery coverage where those phases apply.

#### Scenario: Required phase has no fixture

- GIVEN an inventory row declares a lost-response window without a matching fixture
- WHEN coverage validation runs
- THEN that row MUST remain incomplete

### Requirement: Rollback tests preserve witness boundaries

r[molten.world_faults.verification] Local-profile rollback tests MUST NOT claim detection when head and generation state roll back together. Strong rollback cases MUST use independent admitted witness state.

#### Scenario: Only local image is restored

- GIVEN the harness restores an older complete local image and no independent witness state exists
- WHEN rollback classification runs
- THEN the result MUST state that whole-store rollback detection is unproven

### Requirement: Fault receipts are bounded evidence

r[molten.world_faults.receipt] Conformance receipts MUST bind the inventory, profile, source revision, adapters, schedules, limits, cases, observations, decisions, and unsupported rows. They MUST NOT claim universal crash safety, physical power-loss coverage, storage correctness, or release eligibility.

#### Scenario: Focused matrix passes

- GIVEN every required bounded case passes for one reviewed cohort
- WHEN the final receipt is emitted
- THEN it MUST identify that cohort and preserve all physical-failure non-claims

### Requirement: Oracle source and build identity are exact

r[molten.world_state_oracle.source] The DoltLite oracle MUST bind the exact upstream source revision, imported scope, applicable notices, build inputs, feature set, backend format, and adapter version. Remote support MUST remain disabled for this pilot.

#### Scenario: Oracle source is admitted

- GIVEN the source revision, license material, build inputs, and disabled-remote profile match the reviewed cohort
- WHEN oracle admission runs
- THEN Molten MAY plan a disposable test execution

#### Scenario: Remote support is enabled

- GIVEN the candidate build enables DoltLite remote behavior
- WHEN oracle admission runs
- THEN Molten MUST reject the build before test execution

### Requirement: Oracle stays behind a test-owned boundary

r[molten.world_state_oracle.boundary] Molten MUST expose DoltLite only through a test-owned semantic-state oracle port. Molten cores MUST NOT depend on SQLite types, DoltLite types, file paths, process state, or hidden current-branch state.

#### Scenario: Core receives normalized facts

- GIVEN the shell executes one admitted DoltLite operation
- WHEN it returns the result to comparison logic
- THEN the result MUST use Molten-owned semantic observations and explicit infrastructure errors

#### Scenario: Vendor type reaches the core

- GIVEN an adapter attempts to pass a SQLite handle, row, error, or branch-global into a Molten core
- WHEN the architecture gate runs
- THEN the gate MUST reject the dependency direction

### Requirement: Oracle observations are canonical and identity-separated

r[molten.world_state_oracle.observations] The oracle MUST use explicit primary keys and deterministic ordering. It MUST emit canonical ordered semantic observations with a Molten-owned BLAKE3 identity. DoltLite object identifiers MUST remain backend-local evidence.

#### Scenario: Two backends agree semantically

- GIVEN Molten and DoltLite produce the same ordered keys, values, and operation outcomes
- WHEN differential comparison runs
- THEN it MUST report bounded semantic agreement without asserting root-format equality

#### Scenario: Schema depends on rowid

- GIVEN an oracle schema or case depends on implicit SQLite rowid identity
- WHEN fixture admission runs
- THEN Molten MUST reject the case as noncanonical

### Requirement: Compatibility differences are typed and ratcheted

r[molten.world_state_oracle.compatibility] The repository MUST maintain a Nickel compatibility ledger with closed statuses `compatible`, `adapted`, `intentional`, `unsupported`, and `engine-gap`. Every row MUST bind source contract, evidence, fixture, and explanation. Each `unsupported` or `engine-gap` row MUST also bind a tracked issue and negative fixture.

#### Scenario: Exception count does not increase

- GIVEN all rows have valid evidence and exception counts do not exceed policy maxima
- WHEN the compatibility gate runs
- THEN it MUST report the exact status totals

#### Scenario: Unsupported row lacks a negative fixture

- GIVEN one unsupported or engine-gap row omits its issue or negative fixture
- WHEN the compatibility gate runs
- THEN it MUST fail before acceptance

### Requirement: Oracle cases cover branch, concurrency, format, and GC behavior

r[molten.world_state_oracle.behavior] The oracle suite MUST cover history-independent primary-key state, detached reads, branch isolation, stale-snapshot denial, compare-and-advance races, reader-safe GC, exact format rejection, and serialization behavior.

#### Scenario: Different mutation histories converge

- GIVEN two admitted operation sequences produce the same canonical primary-key state
- WHEN each sequence commits under the same oracle profile
- THEN the oracle MUST report equal backend roots for that pinned cohort

#### Scenario: Stale reader attempts a write upgrade

- GIVEN a reader holds an older snapshot and another writer advances the branch
- WHEN the stale session attempts to write without refresh
- THEN the oracle MUST report the expected stale-snapshot denial class

### Requirement: Oracle verification preserves Molten ownership and non-claims

r[molten.world_state_oracle.verification] The pilot MUST test positive and negative cases for source admission, canonical observations, branch behavior, concurrency, GC, format handling, ledger enforcement, and claim boundaries. It MUST keep complete-world atomicity, durable conflicts, typed merge, authority, effect release, retention policy, and stack-global identities under Molten ownership.

#### Scenario: Oracle behavior conflicts with Molten policy

- GIVEN DoltLite keeps a conflict only inside a transaction or cannot atomically write multiple file-backed databases
- WHEN the compatibility result is classified
- THEN Molten MUST record an intentional difference and MUST NOT weaken its world-commit contract

#### Scenario: Negative corpus is incomplete

- GIVEN fixtures omit stale-writer, wrong-format, missing-pin, malformed, unsupported, or overclaim cases
- WHEN verification coverage is evaluated
- THEN the change MUST remain incomplete

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
