# Validation evidence

## Scope

This change moves every Molten Nickel surface to one reviewed cohort.
It does not move product policy or authority into Nickel.

Base source commit: `f6247dc2e0071017320374216521b20095a216c1`.

## Cohort

The accepted cohort is:

- Nickel CLI `1.17.0`
- upstream commit `1320a983e6c3d1e2fb53dd2464b084b4903b1426`
- `nickel-lang 2.2.0`
- `nickel-lang-core 0.18.0`
- `nickel-lang-parser 0.3.0`
- `nickel-lang-vector 0.2.0`

Nix generated the `nickel-cli` lock graph from the exact upstream flake input.
Cargo generated the embedded evaluator lock changes.
The repository-owned unit2nix tool regenerated both build plans.
No lock or generated plan was edited by hand.

The generated plans bind Cargo lock SHA-256 `5318f8813d165a238774b0a8fbe3c6a81d18b6203a1cbf1c710840e02f042dc6`.
SHA-256 is used because unit2nix defines this interoperability field.

## Compatibility and boundaries

The existing embedded evaluator API compiled without a compatibility shim.
Existing policy, authority, configuration, receipt, runtime-profile, and redaction tests passed.

`nickel-toolchain-cohort` passed.
It checks the CLI version, source revision, crate versions, positive profile export, malformed refs, bounds, unsupported metadata, missing adapters, and missing imports.

A successful Nickel evaluation still passes through Molten decoding, policy, authority, resource, and effect gates.

## Validation

`nix develop -c cargo test -p molten` passed.
The package ran 1,265 library tests, 51 binary tests, and 119 integration tests.

`nix develop -c cargo clippy -p molten --all-targets -- -D warnings` passed.
`nix develop -c cargo fmt --check` passed.

The Octet run reported existing workspace findings.
This warning-only result is not strict Octet acceptance evidence.

`nix build .#checks.x86_64-linux.molten --no-link -L` passed with local builders.
The Nix nextest rail ran 1,378 tests with no failures or skips.
Its CI receipt is `blake3:5189852ec177363a00280f67214ba34fcfc433e9fa9d79f2a960ce11b8a075c4`.

Strict Cairn validation passed with no issues.
Final gate receipts before sync:

- proposal: `a043cb9fc4524bda0424a13e2ff02772cce5b0dd9692db4f8dc62b2b0d2e4274`
- design: `9a53fc403f0e4ca51877ce65e4119ae26d99239c1a2756be40e62fa62975c676`
- tasks: `42dde0b41bbeb573d14084f9806f5df8561fb8ecaaa11f430d953b312184a3c0`

The sync dry-run passed with plan `6e375b653c89515a3ef6c9ef49ef5020db1544c964d942157960ad28fb36b34b`.
The executed sync added all five requirements to the accepted Nickel toolchain specification.
Strict validation passed after sync.

The archive dry-run passed with plan `c990d15403d957c63d6c5ccdf566b5f11832977b38db47cf5af9410898808793`.
Archive execution moved the package to `2026-08-24-adopt-nickel-1-17-cohort`.
The archive receipt is `e775b6f0012a0432601745a493c0f99761a7c950c41133e5962001f9324b4705`.
Strict validation passed after archive execution.

## Non-claims

The cohort identity does not prove policy correctness, authority correctness, runtime correctness, or release readiness.
Nickel remains an evaluator dependency under Molten-owned product decisions.
