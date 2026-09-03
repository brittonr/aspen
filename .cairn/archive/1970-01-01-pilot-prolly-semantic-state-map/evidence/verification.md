# Verification evidence

## Baseline

The branch started from canonical `origin/molten` commit `1be3e9598adf21715e17f83afe089979cb6b8b10`.

Before implementation, these checks passed:

- 323 `molten-core` tests and 7 documentation tests;
- 4 world benchmark shell tests;
- 5 world distribution shell tests;
- 4 world fault shell tests.

No baseline failure blocked comparison.

## Architecture and reuse

A bounded portfolio search evaluated published component reuse, direct DoltLite use, path-local updates, and rebuild-first updates.

No published Onix component owns the same ordered-map semantics and authority boundary. The pilot reuses only matching components:

- Content Identity Core revision `7f55597b5dc879b7601856e8d7fd0dbacaa2a498` for tagged identity framing;
- Schema Migration Core revision `4fe90e130f2871cf69a6febcdc70785adca98aea` for explicit future migrations;
- the existing world benchmark classifier;
- the pinned DoltLite oracle for independent normalized observations.

The selected rebuild-first mechanism is the weakest checked design. It reads a complete supplied snapshot, applies edits, rebuilds the canonical tree, and stages only new block identities.

The pure core has no filesystem, Redb, capability, process, environment, or clock types. Redb and node-state capabilities remain in `src/prolly_map`.

## Profile and format

The typed profile is `config/prolly-map/profile.ncl`.

Profile identity:

`blake3:820d0424eac0ce727d80750e485dcaac320137baa5b4ef21d0d67708ca8a41d5`

The profile binds codecs, ordering, binary node format, BLAKE3 domains, boundary seed, exact size accounting, byte bounds, fanout bounds, hard limits, and format version.

`prolly-map-profile` regenerates and byte-compares the profile, benchmark, and proof-obligation JSON projections. Seven negative Nickel fixtures fail export as required.

Ten Preserves boundary artifacts register profile, node, root, edit, diff, GC, differential, benchmark, and publication records. Node payload bytes use the separate documented `MPL1` binary codec.

## Core behavior

Fourteen focused core tests cover:

- exact profile identity and drift denial;
- canonical node encoding and decoding;
- duplicate, overlapping, missing, extra, oversized, and tampered data;
- point and range reads;
- chosen-key pressure and forced size bounds;
- 120 bounded insertion permutations;
- insert, update, delete, batch, replay, and compaction histories;
- structural sharing and staged-block selection;
- complete added, removed, and modified diffs;
- closure, reachability, incomplete graphs, and active pins;
- bounded benchmark facts and overclaim denial;
- DoltLite normalized agreement and divergence;
- extraction classification;
- explicit proof obligations and non-claims.

Equal final maps produced equal roots and block sets. Compaction preserved the root.

A one-value update reused four blocks, staged only changed blocks, reported one modified entry, and skipped four equal nodes.

## Shell and recovery

Six focused shell tests cover:

- capability-rooted Redb open;
- immutable block staging;
- block identity collision denial;
- transactional compare-and-advance;
- stale publication;
- restart and readback;
- publication receipts;
- unknown outcome before and after apply;
- one readback with no blind retry;
- GC denial without exact current roots, pins, candidates, generation, policy, and deletion authority;
- admitted deletion of exact revalidated candidates.

Publication receipts set future-mutation and deletion authority to false.

## Benchmark

The named cohort is `logical-bounded-rebuild-first-v1` with 64 entries and one value update.

Measured structural facts:

- logical bytes: 4,608;
- block count: 6;
- retained block bytes: 5,993;
- reused blocks: 4;
- diff records: 1;
- skipped equal nodes: 4;
- GC candidates: 2;
- restart mismatches: 0;
- maximum admitted node bytes: 4,096.

The generated benchmark contract binds these observations and thresholds. Timing does not prove correctness.

The existing extraction classifier reports `retain-current`. One credible Molten consumer does not satisfy the required two-consumer gate. No repository or dependency is approved.

## Differential boundary

The map compares ordered semantic rows and outcomes with the pinned DoltLite oracle.

The comparison ignores backend-root spelling. Agreement applies only to one case and exact cohorts. It does not prove format parity or correctness.

## Proof boundary

`config/prolly-map/proof-obligations.ncl` records seven obligations against Trellis reference revision `0bf65150d4c75da5887d5cc53392c3da6b94b9d2`.

Six obligations are model-checked. Formal refinement from Trellis models to the production Rust profile and node codec remains open.

No obligation claims collision impossibility or database correctness.

## Octet and Clippy

The focused `prolly-map-octet-deny-all` check is clean:

- findings: 0;
- warnings: 0;
- errors: 0;
- config hash: `b3:473a2b0d9d00000d7f581cb06341c1b45e289f7694bdd910192ac1c1b88a9925`;
- profile hash: `b3:bad620bb7bd01b8e5cfbff6ae208e47e7ee5d88e3a6db12d9755cb668774f45b`.

`cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.

The pinned Trellis reference rail passed public-function annotation coverage, ensures coverage, proof-gap inventory, policy fixtures, lifecycle fixtures, and verified primitive manifests.

Its broad repository verifier is not acceptance evidence for this map. It also reported inherited failures at revision `0bf6515`: sibling Aspen path lookup, README/module index drift, an old Cargo 1.76 edition mismatch, release-profile drift, missing Clippy, and existing Verus failures in unrelated modules. The bounded run was stopped during the unrelated full Verus corpus. Formal Prolly refinement remains explicitly open.

## Full Rust suite

`cargo test --workspace --all-targets --all-features` passed:

- 1,415 Molten library tests;
- 74 Molten binary tests;
- 61 CLI harness tests;
- 12 content-replication tests;
- 6 profiling-boundary tests;
- 6 executable-extent tests;
- 3 simulation-boundary tests;
- 2 execution-boundary tests;
- 2 node-host facade tests;
- 8 native system-extension tests;
- 4 world-commit integration tests;
- 342 `molten-core` tests;
- 5 node-host boundary tests;
- 5 release-policy binary tests.

The live DoltLite test remains ignored in the broad suite because it requires explicit Nix cohort variables. Its accepted oracle evidence remains pinned in the prior archive.

## Generated plans

The pinned unit2nix tool generated each plan twice with identical bytes.

- `build-plan.json`: 713 crates, 4 workspace members, 989 build units, and 1,019 test units.
- `release-policy-build-plan.json`: 254 crates, 2 workspace members, 323 build units, and 2 roots.
- The release plan contains the `molten-release-policy` binary and no dev-dependency projection.
- Cargo lock SHA-256: `c5d0d470a081ac0e22094371520cac94bf6bece4f65c1a4f61fb874dea7ae5de`.
- Main-plan BLAKE3: `7653a43835719a44952af72ac1b0ac6d01013800c258ac57816ff758d99fd52a`.
- Release-plan BLAKE3: `e0a0e290109f8077b084882f7eabe07fdda10a2a2a3a220454f4532739b12371`.

SHA-256 appears only because unit2nix defines that interoperability field.

## Nix checks

These focused checks passed with local builders and secret-key files disabled:

- `prolly-map-profile`;
- `prolly-map-schema-inventory`;
- `prolly-map-octet-deny-all`.

`nix flake check --no-build` evaluated every package, check, app, shell, and formatter.

The known full-flake inherited Tracey path blocker remains outside this change.

## Claim boundary

The pilot does not prove BLAKE3 collision impossibility, whole-database correctness, branch authority, merge correctness, effect safety, replication correctness, GC execution safety, universal performance, production readiness, or release eligibility.

## Lifecycle

Cairn validation passed under policy hash `8280151b7a53822eed460149ecb600bf11418d7463c935e0742009b334f4e7dd`.

Gate receipts:

- proposal input `c051fe2c4f2e9289cf7c70465977fe412ba9f7c5201d979b53ab4bef934dc054`, receipt `2b5bda4fcfe9434be0a3a22b27735a843dc2697729ff483afd69395345718cc0`;
- design input `12236973fcd54865645b00fc4b93a72def33dc33efd3a5b407be8e5ec534f261`, receipt `1ec4117d800d301d7221cab306577b5bcda801a97835135fdf0c75f755e05cb4`;
- tasks input `448ef62c3c0a6f89c80e89ac13db288ff1f0a006d2fae881016717fa5cabbd63`, receipt `246a4d427097dd578d9e21c6f995138f20925b8a8af5e0a1866ae9aa9deff646`.

Sync dry-run was unblocked. Its plan hash was `d79e188ae596bed2a9d659315d72b65798588c0c03459bb2946d6f1686834a44` and receipt was `374e333f02f045252a4ad1bdcb908c8b0eb7b43426cff294fa02e4268716134a`.

Executed sync receipt `2747e814ed459ed111b9076a5a89b43919731953d34b766e0f9882d8fe75f8d4` produced accepted world-commit spec hash `3eec8da7c4e18711fcca05d11cde6873ce8f8ca3dd205b9948af7424fe82f4ad`.

All 12 `molten.prolly_map.*` requirements are present in `.cairn/specs/world-commit/spec.md`.

Archive dry-run plan `95da69f7afdb2cd91474439d271f3ebf3a9331231b7899969a360cb4704294ce` was unblocked with receipt `102c34051264b6b40e1cc6aa67ac0f15ddf4bda52fea5f1b2a73e92ee67f17ba`.

Executed archive receipt `bb05ad7482edf758fab22954cb5a1631c87cf0a92e2f707ea35ab908d019bb70` moved the change to `.cairn/archive/1970-01-01-pilot-prolly-semantic-state-map`.
