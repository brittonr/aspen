# Validation evidence

## Scope

This change separates selected membership, time, entropy, transport, durability, and shared system-extension capability contracts from concrete mechanisms.
It preserves existing pure-core decisions and canonical artifacts.
It does not claim live correctness or release readiness.

Base source commit: `59aa8d153548bca4242f76bceb3245b0a2374c39`.

## Inventory and ownership

`docs/fabric-port-ownership.md` records each selected capability, owner, inputs, outputs, typed failures, effects, and composition root.

Application-owned `ports.rs` modules now own the selected contracts.
Application-owned `shell.rs` modules own admission, ordering, and uncertain outcomes.
The maintained `adapters.rs` modules contain static, scripted, virtual, operating-system, Iroh, Redb, and in-memory mechanisms.
Pure transition and policy functions remain in `molten-core` or existing canonical core modules.

The shared `FabricEffectPort` now returns `FabricPortResult` rather than a raw string failure.
Membership persistence, clock, entropy, transport, and durability ports also return typed failures.
Authority denial in the membership shell occurs before intent persistence or role effects.

## Compatibility baseline

Before migration, focused tests passed for:

- membership: 3 tests;
- time: 16 library tests and 2 CLI tests;
- transport: 15 library tests and 3 CLI tests;
- durability: 7 tests.

The same focused suites passed after migration.
Canonical Preserves values, BLAKE3 transition refs, receipt meanings, and supported simulation or live behavior did not change.

## Positive and negative checks

The pure architecture audit passed compliant fixtures.
Its negative fixtures detected adapter-owned traits, raw string port errors, host effects in core scopes, duplicated adapter policy, and concrete adapter construction in core scopes.

Existing positive and negative suites passed for:

- static and deterministic membership providers;
- authority denial before protected effects;
- intent-before-effect and commit ordering;
- uncertain membership role and persistence outcomes;
- live and virtual clock behavior;
- backward-time and timeout handling;
- bounded entropy and exhaustion;
- deterministic and Iroh transport behavior;
- malformed frames, partitions, cancellation, timeout, and uncertain delivery;
- simulated and Redb durability behavior;
- storage failure, precommit failure, postcommit uncertainty, commit, and recovery;
- system-extension effect routing and simulation substitution.

`cargo test --workspace --all-targets` passed.
`cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
`cargo fmt --all --check` passed.

`cargo octet check` reported 5,833 existing warning-level findings.
The base cohort reported 5,856 findings.
This change adds no finding and removes 23 findings through narrower modules and justified source allowances.
The warning-only result is not strict Octet acceptance evidence.

`nix build .#checks.x86_64-linux.fabric-port-boundaries --no-link -L --option builders ""` passed.
`nix build .#checks.x86_64-linux.molten --no-link -L --option builders ""` passed.
The Nix nextest rail ran 1,405 tests with no failures or skips.
Its CI receipt is `blake3:4a063650eda4039968cb22cf471d6f357d670054db5b693547e383fd819c6472`.

Strict Cairn validation passed with the current Cairn policy.
Final gate receipts before sync:

- proposal: `26273a5ef8a9329c99c3ad2de562c731c86bc0b214fb7cbdad751313c66425d6`
- design: `5fa182e5ff931063fadc18097796b6106436dfa94c5a6710cca16ffd92aadfce`
- tasks: `725680caa7fd8a20c0e4d9842756d5b2313a82467921adcbf89dcddbd6f4597a`

The sync dry-run passed with plan `fa929ba252eb9440279c51477168387d7d5d7783a5a8d81b00ec48d2f84847cb`.
The executed sync added all 13 requirements to the accepted project specification.
The sync receipt is `705169256e83806fbdda0449088e3c82af397a1fd60263f809c35c354a1d1a7a`.
Strict validation passed after sync.

The archive dry-run passed with plan `f68148cafd48a71f81fe5f2382b6950bb55b352d7b4e9263d3a950d61ade9a5a`.
Archive execution moved the package to `2026-08-24-separate-fabric-ports-and-adapters`.
The archive receipt is `1b39c1001c1aa58c63fcc0f78458f77305f7b1128bb1f8bd756a2a1761bc1b19`.
Strict validation passed after archive execution.

## Non-claims

This source boundary does not prove live transport correctness, durable storage, clock accuracy, entropy quality, authority correctness, simulation parity, or release readiness.
The architecture audit proves only the checked source shapes.
