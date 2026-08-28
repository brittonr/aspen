# Addressable actor verification

## Scope

This evidence covers the Molten-owned addressable actor composition. It does not transfer ownership from system-extension, placement, coordination-delivery, durable-state, logical-time, resource, supervision, authority, or evidence components.

The implementation adds:

- canonical actor keys and a versioned profile;
- dormant, starting, running, draining, stopped, degraded, and recovering states;
- generation-, placement-, sequence-, and wake-bound plans;
- a closed survival matrix;
- checkpoint, sleep, wake, restore, drain, delivery-completion, and unknown-effect transitions;
- a thin shell with compare-and-commit, fresh pre-effect admission, Redb restart, status, and Preserves receipts;
- deterministic simulation and child-process generation-fence cases.

## Pinned reference

The bounded design reference is `rivet-dev/actors` revision
`71f371ba4eab1234d8b6b6c419e6748cc6fc9911` under Apache-2.0.

Reviewed file BLAKE3 identities:

- license: `9730ca2805f3a9f8b81e75ce828f611b26f01c762b1b4186976c5df18039d22e`;
- actor runtime: `8eaa12389fa10271cb51d54880834e2a62abab5eb83a089a2d53542bf7e5e100`;
- actor keys: `f67e0a25bdccaeaa8fed49f28c8ed007e184a2e9c80d500bf3bef184680c9cbd`;
- persisted classes: `60608ff1b2c7d71792c43c44c12323079e07291c2def0605adb424dc850ef5f5`.

Rivet APIs, formats, benchmarks, global-key claims, transport behavior, and service guarantees are not compatibility targets.

## Profile and schema checks

The generated Nickel profile BLAKE3 is
`78c51049a7af0fb68a433d9de1168586e592da7949895db022639953ed7053ca`.

`addressable-actor-profile` passed format, deterministic export, seven negative fixtures, functional-core effect scanning, and required port checks.

`addressable-actor-schema-inventory` passed for all eleven Preserves boundary artifacts.

The negative Nickel fixtures reject a moving source, a missing port, process-survival overclaim, a missing non-claim, automatic external retry, receipt authority, and a zero idle limit.

## Focused Rust checks

The pure core ran 11 positive and negative tests. They cover key admission, message and timer wakes, start fencing, idle sleep, bounded drain, durable restore, runtime-only restore denial, delivery acknowledgement, authority and resource denial, failed recovery, generic system-extension mapping, unknown effects, and bounded status.

The shell ran 15 positive and negative tests. They cover commit-before-effect ordering, fresh admission before every effect, changed-generation denial, unknown commit reconciliation, unknown-effect quarantine, deterministic simulation, capability-rooted Redb restart, canonical records, and a child-process stale-generation fence.

Focused Clippy passed with warnings denied.

## Full workspace checks

The final serial all-target, all-feature workspace run passed:

- 1,446 main-library tests;
- 74 main-binary tests;
- 61 CLI harness tests;
- 364 `molten-core` tests;
- all remaining workspace, integration, example, node-host, and release-policy targets;
- one expected DoltLite live-oracle test remained ignored because it requires the Nix-built cohort.

The same command completed workspace Clippy with warnings denied and `cargo fmt --check`.

The native system-extension process suite completed inside the serial run. Its eight long-running cases took 509.16 seconds.

## Octet

`addressable-actor-octet-deny-all` passed with:

- status: clean;
- findings: 0;
- warnings: 0;
- errors: 0;
- config hash: `b3:ca1303958015b206349a0d19f5ed93da06eb0d52d7592a3d81a71132e9eebfd5`;
- profile hash: `b3:c1469c37b0041dea6c11ec7bb5907b25050bf6891291be5ff56346745b82fd36`.

The focused lock BLAKE3 is
`df846e09b9941489c7c32292012d75f3a9d15367d7b8a41e5af9c9ed9e3842d6`.

## Nix

These focused checks passed with empty builders and secret-key files:

- `addressable-actor-profile`;
- `addressable-actor-schema-inventory`;
- `addressable-actor-octet-deny-all`.

`nix flake check --no-build -L` evaluated all package, check, app, shell, and formatter derivations. It reported `all checks passed`.

## Generated-plan blocker

Main unit2nix regeneration was attempted with workspace, all-feature, and dev-dependency scope. Cargo panicked at
`cargo-util-schemas/src/core/package_id_spec.rs:248:40` with
`called Option::unwrap() on a None value` while parsing active `rad://` package IDs.

Release-policy regeneration was attempted separately without `--include-dev`. Its build graph reached 323 units and two roots. Cargo metadata then hit the same package-ID panic.

No generated plan was partially accepted. Cargo manifests and the main lock did not change, so the checked-in plans remain byte-identical:

- `build-plan.json` BLAKE3: `38ccf8f70eeb3c5f863b54430678933cced797f8a194b5a15bfeb96e17a33e12`;
- `release-policy-build-plan.json` BLAKE3: `8d194948a331a37a55fae6e19bf734af4a1e84d2e3a6f06bacfd732d8e863ddd`;
- Cargo lock SHA-256: `6393384ce712610bed165680cd20cc1b097a56fbaf8584c46084b44562aed247`.

SHA-256 is recorded only because unit2nix defines that interoperability field.

## Cairn lifecycle

Cairn validation passed under policy hash
`8280151b7a53822eed460149ecb600bf11418d7463c935e0742009b334f4e7dd`.

Gate receipts:

- proposal input `839bf1c983f7b2dab30b6825b2e65e3410505b45a344254ecb53dbe9d4ec94be`, receipt `7fd6b9acb9efdc9278c62e955e19de460b5b0fa5f725d53f1d3b2285201872f0`;
- design input `1d393b24001d4c5fef7a74e2eaeecd80bc9b5b9fbbb075a575b2b858989e0ecd`, receipt `123efc3b9044b8fd24861206796d2fc3049b3da6bf1312ddc1416982e4525737`;
- tasks input `010321a4e3d89b19c33264f296e1c7223d51d6780de8e7674a2751534b98d895`, receipt `b4fc5d30eb59c615b80617dbca54097289456eaa1fbb627473599d55d6188bd3`.

Sync dry-run was unblocked. Its plan hash was
`96d2a2e66c0668a070a70dde2ecfd6544384e89d0e19cc4329adbf71be1d6f88` and
receipt was
`499c45cc597a5398749da0672b5ec307025f9fcd4c1777f5ec37af6bfe1241a0`.

Executed sync receipt
`ed8ea829f208dfb4b979b7e2d03f1bafb85810ef06ba3b6092f7c04a02187b71`
produced accepted spec hash
`0a96a2325f525339767277ee6268c387491635fd2f3cdae1cd53a0af56fa9774`.

All six `molten.addressable_actor.*` requirements are present in
`.cairn/specs/addressable-actor-runtime/spec.md`.

Archive dry-run plan
`6a3844736e2d503a18005bdc1defa19a93ff3f19c1e3683f6205e147ffa1bf0f`
was unblocked with receipt
`5a7978275636c62ab26a33b945aaa13350d35b40b4ca0e2dfe46bc3ce97ab820`.

Executed archive receipt
`4fa17428a44194c5c50f4bfa9a40afaa38d742299bd1a26c93e1e0c7c3475601`
moved the change to
`.cairn/archive/1970-01-01-add-addressable-actor-runtime-profile`.

## Non-claims

The evidence does not prove:

- exactly-once delivery or effects;
- global actor-key uniqueness;
- process, stream, session, callback, or in-flight-delta survival;
- transport delivery;
- checkpoint correctness;
- store, scheduler, placement, adapter, or policy correctness;
- production or release readiness;
- mutation, activation, effect, retry, cleanup, or release authority from any receipt.

Unknown external outcomes remain explicit and do not authorize automatic retry.
