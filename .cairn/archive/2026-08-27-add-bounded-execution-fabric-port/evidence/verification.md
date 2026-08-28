# Verification: bounded execution fabric port

## Baseline

Before implementation, these commands passed:

- `nix develop -c cargo test -p molten-core fabric::`: 16 tests passed.
- `nix develop -c cargo test -p molten system_extension::`: 5 tests passed.

## Implementation evidence

The change adds:

- a pure execution profile, request, authority, resource, lifecycle, linkage, and recovery core;
- one application-owned `ExecutionFabricPort`;
- a live adapter over Bounded Exec revision `29dac88ecded94457572db3fdfaaaab95fa91525`;
- a deterministic simulation adapter over the same request and receipt algebra;
- a system-extension composition root with exact profile selection and no fallback;
- bounded output publication through an application-owned output publisher;
- Cargo, Nix, lock, release-profile, and unit2nix source identity bindings;
- typed Nickel profiles with positive and negative fixtures;
- a strict focused Octet workspace with no disabled lints.

## Checks

These checks passed after implementation:

- `nix develop -c cargo fmt --all -- --check`.
- `nix develop -c cargo test -p molten-core fabric_execution::`: 8 tests passed.
- `nix develop -c cargo test -p molten fabric_execution::`: 9 tests passed.
- `nix develop -c cargo test -p molten --test fabricexecutionboundary`: 2 tests passed.
- `nix develop -c cargo test --workspace`: all workspace, CLI, integration, and documentation tests passed.
- `nix develop -c cargo clippy -p molten-core --all-targets -- -D warnings`.
- `nix develop -c cargo clippy -p molten --all-targets -- -D warnings`.
- Nickel format, positive export, and four negative fixture checks passed.
- `nix build .#checks.x86_64-linux.fabric-execution-profile --no-link -L --option builders ""`.
- `nix build .#checks.x86_64-linux.fabric-execution-octet-deny-all --no-link -L --option builders ""`.
- The strict Octet result was `Status: clean`, with zero findings, warnings, and errors.
- Cairn validation and the proposal, design, and tasks gates passed with the current Cairn default policy.
- Cairn sync created `.cairn/specs/bounded-execution/spec.md` with all 12 requirement IDs.

## Traceability

The current Cairn default Tracey profile reports unrelated inherited gaps across the repository. It reports no uncovered or untested `molten.fabric_execution.*` requirement after this change.

The checked-in Aspen policy is not parseable by the current Cairn CLI because it contains `task_marker_policy.markers`. The current Cairn default policy was used for lifecycle validation.

## Broader check caveats

The repository-wide release dependency Nix check cannot execute its `molten-release-policy` binary from the pinned unit2nix package projection. Direct release validation also finds a stale, missing Octet archive path in the pre-existing release profile.

These failures are outside the bounded execution contract. Focused source identity, profile, tests, Clippy, Nickel, and strict Octet gates pass.

## Claim boundary

The evidence proves only the recorded bounded process and simulation behavior for this source and profile cohort. It does not prove sandboxing, hermeticity, executable trust, child correctness, network isolation, platform equivalence, application success, or release readiness.
