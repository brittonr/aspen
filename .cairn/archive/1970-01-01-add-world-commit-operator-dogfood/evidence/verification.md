# Verification

Date: 2026-08-28

## Completed boundary

Molten now has one preview-first `molten world` command family.
It plans typed world operations without creating a second runtime or daemon.

The pure core owns these facts:

- bounded request validation;
- deterministic dependency ordering;
- profile admission and first-blocker selection;
- domain-separated BLAKE3 plan identity;
- exact apply admission;
- component owner validation;
- aggregate receipt validation;
- bounded machine and human summaries.

The core has no file, process, network, environment, clock, credential, path, or storage access.

The shell composes one handler per operation kind.
Each handler declares one closed component owner.
The shell rejects crossed handlers and crossed receipt owners.

The standalone CLI has no ambient live handler registry.
An apply request writes a denial receipt and fails before component effects.
An embedding must supply reviewed handlers and current-facts adapters explicitly.

## Owner map

The owner map is closed:

- World Commit owns inspect and checkpoint.
- World Head owns branch creation.
- Fabric Simulation owns run and simulate.
- World Merge owns diff and conflicts.
- World Replay owns replay, verify, export, and import.
- World Promotion owns promote.
- World Distribution owns garbage-collection planning.

Aggregate records preserve each owner, operation, evidence role, state, and component reference.
They reject authority, deletion-authority, and sensitive-material overclaims.

## Reviewed dependencies

The implementation consumes these archived Molten boundaries:

- world commit core;
- branch heads;
- logical diff and merge;
- logical and opaque snapshots;
- branch authority;
- promotion and release reservation;
- replication and retention;
- replay capsules.

The opaque fixture uses ChaosControl snapshot descriptor revision `b8c440ea3b19df796542e58e8ee36200e1c3db85`.

## Baseline

Before core changes, these tests passed:

- 300 `molten-core` library tests;
- 1,390 `molten` library tests.

## Positive behavior

The logical dogfood fixture plans this complete chain:

1. inspect;
2. checkpoint;
3. branch;
4. simulate;
5. run;
6. diff;
7. conflicts;
8. replay;
9. verify;
10. promote;
11. export;
12. import;
13. garbage-collection planning.

The apply rail rechecks the head, generation, policy, authority observation, and profile before each mutation.
Unknown promotion outcomes stop export, import, and garbage-collection planning.
No automatic retry occurs.

The opaque fixture replays one exact opaque profile.
A semantic diff request fails before a handler runs.
No logical fallback is available.

## Negative behavior

Focused tests cover these failures:

- stale plan identity;
- implicit latest head through a missing required field;
- changed generation;
- missing profile;
- blocked, unsupported, and unavailable profiles;
- denied or crossed authority observation;
- unresolved conflict;
- uncertain promotion outcome;
- incomplete capsule;
- missing handler;
- crossed component owner;
- raw command field;
- sensitive-material flag;
- authority overclaim;
- deletion-authority overclaim;
- dependency cycle;
- missing dependency;
- duplicate operation;
- opaque semantic comparison.

## Retained fixture identities

Nickel exported each request twice.
The CLI generated every retained plan, receipt, and summary twice.
Repeated bytes matched with `cmp`.

Logical fixture BLAKE3 values:

- request JSON: `7156c8ff8d88255061d7da6e5be476469a94d9ee81bafa4fdbad29dcb334d200`;
- plan: `07f6a4c6463423e732a42bf26e57f718558bf09455979fcfe2f8ea6710fdb993`;
- receipt: `1f5c91ef0f440814b829d67cec6cad9cd2e4b3f14005efe243126de07ad4683c`;
- summary: `487c3a674723a09508dcfd05f32229b00dc38b167da7639005d86fd8e88e1d92`.

Opaque fixture BLAKE3 values:

- request JSON: `7307eace6f732ab18c2c1852d7bc0df17bc36756c31917eda09603c7583a44bd`;
- plan: `3ef9768dc26811dc727bc433763439bc55bcb8c671fbb72a72657d2b08ad6ac3`;
- receipt: `cd8ca4373ce35fb9414a634980f22df03b639d44e895fe405f94f1632f3c1856`;
- summary: `28bf09b51aec92b381f93c3deb0de1fc37765e8d63fd9ae26729981f0c56c86d`.

## Rust verification

These focused tests passed:

- 9 world-operator core tests;
- 7 world-operator shell tests;
- 6 world-operator binary and parser tests.

This full command passed:

```sh
cargo test --workspace --all-targets --all-features -- --test-threads=1
```

The full run included these results:

- 1,401 `molten` library tests;
- 74 `molten` binary tests;
- 61 CLI harness tests;
- 314 `molten-core` tests;
- 5 node-host boundary tests;
- 5 release-policy binary tests;
- every listed integration and feature-gated suite.

Formatting passed with `cargo fmt --all -- --check`.

Clippy passed with this command:

```sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Octet

`checks.x86_64-linux.world-operator-octet-deny-all` passed with zero findings, warnings, and errors.
It checks the pure core and its positive and negative tests with the full pinned catalog.

The broad supported scan completed as warning-only with 6,534 inherited warnings and no errors.
No broad finding names `src/world_operator`, `src/cli/runtime/world.rs`, or `src/cli/runtime/world/`.

A full-feature broad Octet attempt reached Cargo's known `rad://` package-ID panic before linting completed.
That attempt is not acceptance evidence.
The focused strict gate and all-feature Clippy remain the accepted checks for this change.

## Generated build plans

The repository-owned `unit2nix` tool generated both plans twice with identical bytes.

Main plan:

- 713 crates;
- 4 workspace members;
- 989 build units;
- 1,019 test units;
- 7 roots;
- BLAKE3 `40ae6e5cf5e59c5ac0246b712a770008784b77c73e65e9ddb3504dce54c33d29`.

Release-policy plan:

- 254 crates;
- 2 workspace members;
- 323 build units;
- 2 roots;
- BLAKE3 `9ec27b4dcc8ff82dba13d96ffa5fe8fa626493fabd0ae5fbfdb0ed4986fdb0d5`.

The release plan contains the `molten-release-policy` binary target.
It must be generated without `--include-dev`, because the test graph hides build-mode binary metadata.

Cargo lock SHA-256 is `4831149e509872705a47b22aeb5e48e06a2eb7b15524fc09ee7cbf766ffcf935`.
SHA-256 appears only because unit2nix requires that interoperability field.

## Nickel and Nix

Both positive Nickel profiles exported successfully.
Every checked negative Nickel fixture failed contract evaluation.

These focused Nix checks passed:

- `world-operator-octet-deny-all`;
- `world-operator-profile`;
- `world-operator-schema-inventory`;
- `world-operator-dependency-identity`;
- `release-dependency-profile`;
- `release-profile-validation`;
- `deterministic-drift-gate`.

`nix flake check --no-build -L` evaluated every compatible output and reported `all checks passed`.
The command used empty builders and empty secret-key files.

## Cairn lifecycle

Current Cairn validation passed with no issues.
The proposal, design, and task gates returned `PASS`.

Gate receipt hashes:

- proposal: `3b79452eaea0f893bda9774865d7249242f46ba33118a0d20db8274f90800676`;
- design: `0892022d7b57947ac6619bf13091b2b2fd4e268ae105b4087c6c25d74961cc27`;
- tasks: `b39a39ca5f474d4a604ba56e7e9a9f7f9a0a32a89b6c155a1e6668f7117e3314`.

## Inherited repository rail

The inherited built `contract-export-drift-gate` failure remains outside this change.
A complete built `nix flake check -L` pass is not claimed.

## Non-claims

A workflow plan does not execute a component operation.
An aggregate receipt does not replace a component receipt.
Aggregate completion does not prove component correctness.
Plans and receipts do not grant branch, effect, release, or deletion authority.
Opaque replay does not imply logical merge or semantic equivalence.
Dogfood evidence does not prove an arbitrary host, external effect completion, or whole-stack release eligibility.
