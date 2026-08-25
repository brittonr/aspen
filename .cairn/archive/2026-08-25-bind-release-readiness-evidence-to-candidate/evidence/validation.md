# Validation Evidence

## Baseline

Before implementation:

- `cargo test --lib release_candidate_` passed 2 tests.
- `cargo test --test cliharness` passed 59 tests.

## Focused Validation

After implementation:

- `cargo fmt --all --check` passed.
- `cargo test --lib release_candidate_` passed 2 tests.
- `cargo test --test cliharness` passed 61 tests.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `nix build .#checks.x86_64-linux.release-candidate-binding -L --option builders "" --no-link` passed.
- The focused Nix conformance fixture emitted `blake3:3e74bb53f2a48e69af0858c24253c749fbaf5b33fb476ebb775ca3e8f6214128`.

The Nix fixture proves deterministic command wiring only. It is not candidate release evidence.

## Broad Validation

`nix build .#checks.x86_64-linux.molten -L --option builders "" --no-link` passed.
The broad rail ran 1,416 tests with no failures or skips.
Its CI receipt is `blake3:bfe5d0808b236b2286cb64909df2917afa5f182c630e2c4f9429fd976df98266`.

An earlier broad attempt omitted the new untracked CLI test from the flake source. Staging the file corrected the source closure before the successful run.

## Lifecycle

Strict Cairn validation and proposal, design, and task gates passed before sync.

Final gate receipts before sync:

- proposal: `d1896f9956be017847323255bd85417f61c026f6ebb4ed9074d7628dd2793110`
- design: `d5b6d41d25be779cb49eb504ba236002dff3235cfec5d98cadd41ebab0bb7728`
- tasks: `c87166e1a6572be20386ad2a972b467e3b654e8ed0b263dffdd7bb3c0e976b13`

The sync dry-run passed with plan `72d1d18cc1ad8a7952d610a4c108d3f2b823fc5f8c0802d6f083525c0ca22ab2`.
The executed sync added both requirements to the accepted operator-workflow specification.
The sync receipt is `5041891cb5aa62802777a98d3a4c46e77b229b9a4b60accdcc2ddebfd0ed232b`.
Strict Cairn validation passed after sync.

The archive dry-run passed with plan `f658c026709b5c5429f544dc149ed50cfb542cbd54462bb7cfe966fac4abb16b`.
Archive execution moved the package to `2026-08-25-bind-release-readiness-evidence-to-candidate`.
The archive receipt is `8b4062db529295afc388e6ddc02de89baa4bd26eac48c08eaf7de6d6db39c68d`.
Strict Cairn validation passed after archive execution.

## Non-Claims

These checks validate declared candidate identity bindings and deterministic gate behavior. They do not prove external artifact truth, production deployment success, workload safety, or release eligibility.
