# Validation evidence

## Scope

This change adds a bounded, pure, model-only fast-path hazard system.
It does not select a live consensus engine or authorize production use.

Base source commit: `b61bfd5beec4e19b92522df24628d3dd68fdad5b`.

## Reference identity

The comparison cohort is the Jetpack OSDI 2026 work and `stonysystems/jetpack` commit `c03e318ec355b11edd42aac56c68d0765f88d1d2`.
External results are reference-only. They are not Molten proof or release evidence.

## Positive and negative checks

The focused fast-path suite passed 14 tests.
It covers valid bounded profiles, quorum derivation, non-conflicting commits, safe fallback, recovery, deterministic replay, minimization, evidence, and reference comparison.
It rejects unknown references, unsupported fault models, malformed bounds, impossible quorums, live or production selection, claim overreach, ordering faults, mixed views, missing promises, identity mismatch, duplicate application, and false non-conflict decisions.

Nickel accepted the positive profile.
Nickel rejected both production selection and unsupported reference fixtures.

`cargo test --workspace --all-targets` passed.
`cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
`cargo fmt --all --check` passed.

`cargo octet check` reported 5,856 existing warning-level findings.
This matches the base cohort and adds no finding.
The warning-only result is not strict Octet acceptance evidence.

`nix build .#checks.x86_64-linux.consensus-fastpath-model-profile --no-link -L --option builders ""` passed.
`nix build .#checks.x86_64-linux.molten --no-link -L --option builders ""` passed.
The Nix nextest rail ran 1,401 tests with no failures or skips.
Its CI receipt is `blake3:d56a0080be75e40e4866d76007a659438c9fe9f8aa4233280b99d9a955d1e34f`.

Strict Cairn validation passed with the current Cairn policy.
Final gate receipts before sync:

- proposal: `6ef77125455aa08d353722e35cf1667decdbea8e16e339c69712255d36d2134e`
- design: `3b80f3d3149961e2743364b5e1171408d70693d79d1e0077ec2aa1de35808ed7`
- tasks: `26778f07f5a4e1680fc3cb67384bded4f1c793a2c199e108460240f04a3c50e7`

The repository-wide traceability command ran and reported the inherited uncovered-requirement debt.
The change does not claim that this repository-wide debt is resolved.

The sync dry-run passed with plan `f28bf1a8a95b71e9d7920399b30e2028bd599238d62175617d61560e0c8e714e`.
The executed sync added all 11 requirements to the accepted consensus specification.
The sync receipt is `9d07ad3b43769a92de77f73c73474236ca2954912ba63cb9737bcd136e13ea14`.
Strict validation passed after sync.

The archive dry-run passed with plan `bdec06fad1fb0821d7e4a7d503a9c040fe3e6984a1b2f80b17955a0f4fd3a107`.
Archive execution moved the package to `2026-08-24-model-consensus-fast-path-hazards`.
The archive receipt is `a70e1a8196c998feedf73d7ef6e84ed4800314f066adee6eff5f98b74b399663`.
Strict validation passed after archive execution.

## Non-claims

The model does not prove a live implementation, network behavior, crash recovery, latency, throughput, linearizability of deployed nodes, or production readiness.
The exported repro bundles are model inputs for later simulation and ChaosControl work, not pass evidence.
