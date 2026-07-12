## Context

Aspen uses an `octet-src` non-flake input to substitute one Rust source dependency. That input is a distinct concern from proof-tool packaging. The node-replication pilot currently assembles an independent Rust toolchain and Verus wrapper from profile metadata, producing a different Nix closure than Octet's reviewed package.

## Goals

- Separate Octet runtime-source substitution from Octet toolchain consumption.
- Execute the compatibility probe with Octet's production verifier package.
- Validate and bind Octet's packaged profile and source revision.
- Preserve the deterministic blocked decision and runtime-dependency denial.

## Non-Goals

- Add `verified-node-replication` to Aspen or Molten runtime dependencies.
- Upgrade or patch Verus to make the pilot pass.
- Claim that central packaging transfers upstream proof authority.

## Design

A new revision-pinned flake input named `octet-toolchain` supplies the profile and production verifier packages. The existing `octet-src` input remains non-flake and continues to serve Cargo source substitution only. The pilot module receives `octetPackages`, validates the central profile through one reusable `jq` predicate, exercises a mutated profile as a negative test, and invokes `octet-production-verus` for both compatibility probes.

The decision payload records the Octet revision, central profile output and BLAKE3 digest, and verifier Nix output. Binary, solver, and toolchain identities continue to come from the package's identity output. The expected internal-error decision remains fail closed.

## Evidence Boundaries

This change establishes package provenance and reproducibility for the probe only. It does not make the upstream crate compatible, discharge trusted boundaries, add runtime code, establish NUMA value, or support distributed-system claims.
