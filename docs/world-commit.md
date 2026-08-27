# Molten world commits

<!-- r[impl molten.world_commit.core] -->
<!-- r[impl molten.world_commit.typed_roots] -->
<!-- r[impl molten.world_commit.capture] -->
<!-- r[impl molten.world_commit.restore] -->
<!-- r[impl molten.world_commit.detached_evidence] -->

A Molten world commit identifies one coherent, profile-relative runtime snapshot.
It composes existing subsystem roots without taking ownership from those subsystems.

The first protocol is product-owned and has the schema `molten.world-commit.v1`.
It does not claim compatibility with any external `RealmCommit` design.

## Ownership

The pure model is in `crates/molten-core/src/worldcommit/`.
It owns validation, bounds, capture plans, revision comparison, closure checks, replay classes, and restore plans.

The canonical codec and shell are in `src/worldcommit/`.
They own Preserves projection, BLAKE3 identity, narrow ports, local publication, detached evidence mapping, and operator readback.

Subsystem owners retain these responsibilities:

- artifact and schema meaning;
- durable state, tasks, history, effects, scheduler, time, and entropy;
- runtime profile and policy admission;
- current authority and resource admission;
- opaque machine-snapshot fidelity;
- storage, retention, replication, and deletion authority.

## Immutable core

The canonical record contains only these fields:

1. the exact schema and version;
2. one closed snapshot profile;
3. zero or more ordered parent commit references;
4. one typed reference for each required root;
5. the profile-relative completeness declaration.

Signatures, attestations, mutable heads, currentness facts, and operator annotations are not core fields.
The decoder rejects unknown or additional fields.

Identity uses BLAKE3 derive-key mode with the context `onixresearch.molten.world-commit.identity.v1`.
The framed input includes the frame version, canonical byte length, and canonical packed Preserves bytes.

Equivalent input orders normalize to the same parent and root order.
A changed version, profile, parent, root type, or root reference changes the commit identity.

## Typed roots

The first version has these closed root domains:

| Root | Logical | Opaque | Mixed | Replay class |
| --- | --- | --- | --- | --- |
| artifact | required | required | required | verify only |
| schema | required | required | required | verify only |
| durable state | required | excluded | required | logical replay |
| tasks | required | excluded | required | logical replay |
| history | required | excluded | required | logical replay |
| effects | required | excluded | required | logical replay |
| scheduler | required | excluded | required | logical replay |
| time | required | excluded | required | logical replay |
| entropy | required | excluded | required | logical replay |
| runtime profile | required | required | required | verify only |
| policy | required | required | required | verify only |
| authority observation | optional | optional | optional | historical evidence only |
| opaque machine snapshot | excluded | required | required | opaque restore |

Opaque and mixed profiles require an exact cohort reference.
A logical profile rejects a cohort reference.
An authority observation never grants current authority.

## Fenced coherent capture

The pure core receives explicit observations and performs no I/O.
Each mutable observation carries its source, root domain, observed revision, and adapter-owned schema-validation result.
Each collection also has a caller-supplied bound under a hard protocol limit.

The shell performs capture in this order:

1. Observe every required typed root.
2. Build and validate the pure capture plan.
3. Persist each missing immutable root object.
4. Synchronize and read back each local object.
5. Recheck every mutable revision and inventory-completeness fact.
6. Build the canonical commit and its identity.
7. Publish the commit object as the final capture mutation.

Drift, missing material, unvalidated schemas, incomplete inventories, identity mismatch, and uncertain publication produce a deny receipt.
They do not produce a successful commit identity.

This process provides a fenced coherent local cut.
It does not claim one atomic transaction across independent stores or services.

## Closure and restore

Closure validation checks every declared root and reachable parent under an explicit object-and-edge bound.
It reports the first missing root in canonical root order.
It also rejects duplicate observations, identity drift, schema drift, missing parents, and parent cycles.

A complete closure produces a deterministic restore plan.
The plan verifies schemas and artifacts before it restores mutable runtime state.
It then requires current policy, authority, resource, runtime, and effect admission before activation.

Closure proves object presence and identity only.
It does not prove compatibility, authorization, restorability, successful activation, or runtime correctness.

## Detached evidence

`project_world_commit_to_valence` maps canonical commit bytes to a Valence Preserves bridge row.
The row has a boundary verification role and identity-only non-claims.

`project_world_commit_artifact_auth_statement` maps the commit and its parents to an Artifact Auth statement.
The function does not sign, verify, or grant authority.

Adding either projection does not change the world-commit identity.

## Local store

`LocalWorldCommitStore` uses the capability-rooted node storage namespace.
It stores roots, commits, and capture receipts in separate bounded subdirectories.

Each immutable file is synchronized and read back before success.
The commit file is written after all root writes and revision rechecks.

This first adapter proves bounded local file completion only.
It does not claim multi-host durability, distributed consensus, race-free multi-writer publication, replication, retention, or rollback resistance.

## Operator commands

Every command requires one explicit commit identity:

```sh
molten world-commit inspect --state-root STATE blake3:COMMIT
molten world-commit validate --state-root STATE blake3:COMMIT --out closure.preserves
molten world-commit explain --state-root STATE blake3:COMMIT
molten world-commit plan-restore --state-root STATE blake3:COMMIT --out restore-plan.preserves
```

Rendered lines are diagnostic views.
The canonical closure report and restore plan remain the normative artifacts.

## Verification

The pre-change baseline passed 199 `molten-core` unit tests, 7 compile-fail documentation tests, and 1,295 root-library tests.

The focused rails are:

```sh
cargo test -p molten-core world_commit
cargo test --lib world_commit
cargo check --bin molten
cargo clippy --workspace --all-targets -- -D warnings
cargo octet check
```

The positive fixtures cover stable identity, logical and opaque profiles, fenced capture, local publication, closure, and restore order.
The negative fixtures cover domain substitution, duplicate roots, stale revisions, missing roots, cycles, malformed schemas, embedded evidence, schema denial, incomplete inventories, bounds, sensitive diagnostics, and uncertain publication.
