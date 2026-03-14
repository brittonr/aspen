# ADR-011: Incremental flake-parts Migration

## Status

Accepted

## Context

The `flake.nix` is ~4,300 lines using `flake-utils.lib.eachDefaultSystem`. As the flake grew with VM tests, dogfood infrastructure, and the verus toolchain, the monolithic structure became hard to navigate. NixOps4's use of `flake-parts` showed a cleaner module-based approach.

## Decision

Migrate from `flake-utils` to `flake-parts` incrementally:

1. **Phase 1 (done):** Wrap outputs in `flake-parts.lib.mkFlake`, keep `flake-utils` for `defaultSystems`, create module stubs in `nix/flake-modules/`, extract the fuzz devShell, move system-independent outputs to `flake.flake`.
2. **Phase 2 (future):** Define `lib.mkOption`-based interfaces in `rust.nix` to expose `craneLib`, `commonArgs`, and build artifacts across modules.
3. **Phase 3 (future):** Extract checks, apps, verus, VMs, and dogfood into their modules, consuming rust.nix options.

## Consequences

+ Structural parity with flake-parts verified — all outputs identical before and after
+ Module files document what each will contain and the migration path
+ Fuzz devShell extracted as proof-of-concept for fully independent modules
~ Most logic stays in `perSystem` until option interfaces are defined
~ `flake-utils` kept temporarily for `defaultSystems` constant
+ Full extraction blocked on defining inter-module option contracts
