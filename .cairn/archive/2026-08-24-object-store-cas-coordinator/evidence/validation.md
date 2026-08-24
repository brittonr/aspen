# Validation evidence

## Scope

This change adds one pure CAS lease decision core under `molten-core`.
It adds no store, clock, network, transport, or consensus effect.

Base source commit: `06b622da4a9b19e5c2e6f767f9556c3fe5d6e2d8`.

## Baseline

`nix develop -c cargo test -p molten-core` passed before implementation.
The baseline ran 173 tests with no failures.

The proposal, design, and tasks gates passed before implementation.

## Functional core

`crates/molten-core/src/fabric_durability/ownership.rs` owns the deterministic decision.
The input contains current, expected, and proposed leases plus the membership posture.

A decision acquires only when the expected lease matches, the epoch advances, and nodes are replaceable.
Every rejection preserves the current lease.

The decision identity uses domain-separated BLAKE3 framing.
The output records contract and implementation identities plus explicit non-claims.

## Positive and negative validation

The focused suite covers these cases:

- a matching expected lease with an advanced epoch;
- an expected-owner mismatch;
- an expected-epoch mismatch;
- a proposed epoch that does not advance;
- a lost lease;
- a fixed-membership assumption;
- an invalid owner.

`nix develop -c cargo test -p molten-core` passed after implementation.
The final package suite ran 179 tests with no failures.

`nix develop -c cargo clippy -p molten-core --all-targets -- -D warnings` passed.
`nix develop -c cargo fmt --check` passed.

The focused `cargo octet check -p molten-core -- --all-targets` run reported existing workspace findings.
It reported no finding for `fabric_durability/ownership.rs`.
This warning-only run is not strict Octet acceptance evidence.

## Generated plans and Nix

Cargo generated the changed `Cargo.lock` metadata.
The repository-owned unit2nix tool regenerated both build plans.
No lock or build plan was edited by hand.

The generated plans bind Cargo lock SHA-256 `374c30f3707eba16e4515d6e5ce3d0bbbe36570736760b9c626f535522369403`.
SHA-256 is used because unit2nix defines this interoperability field.

`nix build .#checks.x86_64-linux.molten --no-link -L` passed with local builders.
The Nix nextest rail ran 1,367 tests with no failures or skips.
Its CI receipt is `blake3:d69eb15ffd6b9710c7b2fa854ff2f2494af3c82c0e8ab331cdb980101e1416f4`.

## Cairn

Strict Cairn validation passed before and after sync.
The post-sync result covered 78 accepted specifications.
The lifecycle result had no issues or substance findings.

Final gate receipts before sync:

- proposal: `045d6815861589fea37666176701bc1f6916fc0b0a91abdd849a0597c6f28d88`
- design: `179ea2cff4fb61d034f7c1526f1b12445790e85e41c663096659ec358c800ada`
- tasks: `dabd5a997e9b0b770cf9f6c992df73db9111ab89f0e2be732a586902402ad389`

The sync dry-run passed with plan `4f29322ff599aff1373f13030d3db71b9d4ad2b50024c80c844a843fd1b731f7`.
The executed sync added all four requirement identifiers to the accepted specification.

The archive dry-run passed with plan `8ca16747f598293cd68edf7b764a885e034327ee89723d2ff83fa0af06fca5c3`.
Archive execution moved the complete package to `2026-08-24-object-store-cas-coordinator`.
The archive receipt is `2f7b0bf533d81469c37b456f7ef8919d8fda5e016844a09c6c7864ae18053cf5`.
Strict validation passed after archive execution.

## Non-claims

This change does not implement durable storage or transport.
It does not prove data integrity, runtime correctness, consensus, availability, or release readiness.
The Celld and WalTier references remain bounded, non-parity design inputs.
