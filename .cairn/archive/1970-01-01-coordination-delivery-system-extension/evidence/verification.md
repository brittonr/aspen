# Coordination delivery verification

## Baseline and dependency closure

Implementation started from canonical `origin/molten` revision
`d7ffdd00d14641a4d79e2e4146e55f71c785341b`.

All declared dependencies are archived:

- `.cairn/archive/2026-07-24-fabric-consistency-service-runtime`;
- `.cairn/archive/2026-07-12-fabric-durable-state-ports`;
- `.cairn/archive/2026-07-12-fabric-observability-integrity-runtime`;
- `.cairn/archive/2026-07-11-fabric-time-scheduler-runtime`; and
- `.cairn/archive/2026-08-01-fabric-whole-system-simulation`.

The pre-change baseline passed 337 `molten-core` tests, seven `molten-core`
doctests, and four focused existing coordination tests.

## Architecture and reuse

A bounded portfolio search rejected changing base FIFO, selecting an external
broker as authority, and treating Animus process queues as the delivery state
machine.

The selected implementation composes accepted Molten primitives:

- `crates/molten-core/src/coordination_delivery/` owns deterministic policy,
  identities, state, transitions, timer intents, worker admission, and status;
- `src/coordination_delivery/` owns capability-rooted Redb, system-extension
  host binding, compare-and-commit, readback reconciliation, timers, status,
  simulation adapters, and canonical receipts.

The pure core has no filesystem, database, process, environment, network, or
clock dependency. It consumes supplied logical-time and currentness facts.

The base `src/coordination/` FIFO implementation was not changed.

## Profile and schemas

The typed source is `config/coordination-delivery/profile.ncl`. Its generated
projection is `config/coordination-delivery/generated/profile.json`.

The policy identity is
`blake3:05be03f3c3a2af25a8ba2f4f603b205b8105abdc15c61b4438518370b5e09d8a`.

The profile binds strict FIFO ordering, logical time, no retry jitter, explicit
attempt and collection bounds, separate authority references, closed failure
classes, exact port-binding references, and six required non-claims.

Seven negative Nickel fixtures reject zero attempts, wall-clock time, inline
payloads, missing non-claims, capacity growth, retry jitter, and receipt
authority.

Seventeen Preserves boundary inventory artifacts cover policy, manifest, host
binding, item, token, attempt, state, ack, nack, expiry, retry, DLQ, redrive,
transition, status, receipt, and worker plan records.

## Transition and shell evidence

Focused core tests cover:

- enqueue, claim, ack, delegated completion, nack, extension, expiry, retry,
  attempt exhaustion, poison-item DLQ, redrive, cleanup, and duplicate replay;
- stale currentness, wrong owner, token drift, expired ack, missing authority,
  unsupported failure, metadata overflow, missing worker admission, and policy
  drift;
- stable policy, manifest, state, request, token, timer, and operation BLAKE3
  identities.

Focused shell tests cover:

- capability-rooted Redb commit and reopen;
- running system-extension host admission;
- unknown-before and unknown-after commit readback without blind retry;
- stale expected state and stale commit observations;
- timer failure after a durable commit;
- canonical receipt and status projections;
- crash/restart, partition, duplicate, and unsupported simulation faults through
  the accepted simulation fault vocabulary; and
- a real child-process fixture that rejects an old delivery token, accepts the
  current token, and preserves the completed state across reopen.

Receipts always deny future mutation authority, worker-effect authority,
exact-once claims, and release eligibility.

## Rust verification

Focused verification passed:

- 11 pure core tests;
- 16 shell, profile, simulation, restart, uncertainty, receipt, and
  multiprocess tests.

Final full verification passed:

- 1,431 main library tests;
- 74 binary tests;
- 61 CLI harness tests;
- 353 `molten-core` tests;
- all remaining workspace targets, including the eight long process cases;
- workspace Clippy across all targets and all features with warnings denied.

Two earlier parallel broad runs exposed existing `fabric_execution` child-process
races as broken-pipe `UnknownAfterStart` observations. Each affected test passed
alone. The final complete workspace run used one test thread and passed every
target without changing the inherited process fixtures.

## Octet

`coordination-delivery-octet-deny-all` compiles the production pure delivery
core with an exact compatibility surface for the already accepted fabric-time
API.

The full pinned catalog passed with:

- findings: 0;
- warnings: 0;
- errors: 0.

The check is evidence about the focused source shape. It does not prove runtime,
store, clock, broker, worker, payload, authority, exact-once, or release
correctness.

## Nix and deterministic generation

The focused profile, schema inventory, and Octet checks build under empty
builder and secret-key options. `nix flake check --no-build` evaluates all flake
outputs.

The pinned unit2nix tool generated the normal workspace plan with `--workspace`
and the release-policy plan with `--package molten-release-policy --bin
molten-release-policy`. The release plan did not use `--include-dev`.

A second generation byte-compared equal to both checked-in plans.

- `build-plan.json` BLAKE3:
  `38ccf8f70eeb3c5f863b54430678933cced797f8a194b5a15bfeb96e17a33e12`;
- `release-policy-build-plan.json` BLAKE3:
  `8d194948a331a37a55fae6e19bf734af4a1e84d2e3a6f06bacfd732d8e863ddd`;
- `Cargo.lock` SHA-256 required by unit2nix:
  `6393384ce712610bed165680cd20cc1b097a56fbaf8584c46084b44562aed247`.

The normal plan retains 713 crates, four workspace members, 989 build units,
and 1,019 test units. The release plan retains 254 crates, two workspace
members, 323 build units, and the `molten-release-policy` binary root.

An attempted all-feature unit-graph generation reproduced the known Cargo
`rad://` package-ID panic. It is not the canonical plan command and is not
acceptance evidence.

## Lifecycle

Cairn validation passed under policy hash
`8280151b7a53822eed460149ecb600bf11418d7463c935e0742009b334f4e7dd`.

Gate receipts:

- proposal input `e3e73d31ad26f0ac4c4645fba2eb1bdbda7c7c787c3f3e441a8317b18676bd64`, receipt `bf6247498fc3596218ca7bc7a22f51b77ad4e84c82d5ef09f97bfcca83f6b810`;
- design input `8008e8eff8a015aa49356a774c84cd68774299549dd17d1f95f7fa219806f11f`, receipt `529e0c6fb2a8b9e2a80ed42901c6ba1748f0199624d7da0e095525f721ddaacf`;
- tasks input `4fea2ec32919bab194b5541e7806dc2a5ba417dbac23892c78c6c37ec8417696`, receipt `0f5e14bc0958c443110fa81981903020be8ceb9eee8759fce9c0f1b713aee24e`.

Sync dry-run was unblocked. Its plan hash was
`550e79387e9817e74708137f860f6681982c6649f7039a3bf819a719d59b2583` and
receipt was
`812f4f5e2df2b357af22e5317f1152066bdc379c5d253c734c6ea4c52cca6322`.

Executed sync receipt
`56f8e83e47a76a0d645a80e1f173d33c79ff821d5981fff4fcbfeafcf7042382`
produced accepted coordination spec hash
`3e66677782eb81a98371346ef5d7467353959e9ef94d4fc300dad8695a79dc99`.

All eight `molten.coordination_delivery.*` requirements are present in
`.cairn/specs/coordination/spec.md`.

Archive dry-run plan
`980d474bd1552bea63541909f337e7c7477346d28dd0feb8b28fca9fc97a19e4`
was unblocked with receipt
`2ec4d1351b9ae87c643e6f7a55e7dfa9250e46373a503cbdc314a660a0766163`.

Executed archive receipt
`01e5d4346af81c97279416aeac083a6f02a43720aad375699c5cb39bcc86f68e`
moved the change to
`.cairn/archive/1970-01-01-coordination-delivery-system-extension`.
