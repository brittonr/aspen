# Verification: native system-extension host

## Baseline

Before native-host changes, these commands passed:

- `nix develop -c cargo test -p molten system_extension::`: 5 tests passed.
- `nix develop -c cargo test -p molten-core system_extension::`: 16 tests passed.

## Implementation evidence

The change adds:

- pure native profile, executable, ingress, operation, recovery, completion, and removal decisions;
- canonical bounded callback envelope and outcome codecs;
- `NativeProcessSystemExtensionExecutor` over the accepted execution fabric port;
- callback and effect intent persistence before external effects;
- canonical instance records with memory and Redb durability-port journals;
- a versioned acknowledged ingress port and local pilot client;
- install, start, request, status, checkpoint, restart, recover, drain, stop, and remove operations;
- generation-fenced effect completion callbacks;
- a workload-neutral node instance registry;
- an offline executable, callback, state, effect, checkpoint, lifecycle, and parent-child artifact index;
- an independent native callback executable at `molten-native-extension-fixture`;
- typed Nickel profiles with positive and negative fixtures;
- a strict focused Octet workspace with no disabled lints.

## Checks

These checks passed after implementation:

- `nix develop -c cargo fmt --all -- --check`.
- `nix develop -c cargo test -p molten-core native_host::`: 9 pure tests passed.
- `nix develop -c cargo test -p molten native_host::`: callback wire and journal tests passed.
- `nix develop -c cargo test -p molten --test nativesystemextension`: 3 separate-process and negative-path tests passed.
- `nix develop -c cargo test -p molten nativehostnode::`: 2 node-registry tests passed.
- `nix develop -c cargo test --workspace`: all workspace, CLI, integration, and documentation tests passed.
- `nix develop -c cargo clippy -p molten-core --all-targets -- -D warnings`.
- `nix develop -c cargo clippy -p molten --all-targets -- -D warnings`.
- Nickel format, positive export, and four negative profile fixtures passed.
- `nix build .#checks.x86_64-linux.native-system-extension-host-profile --no-link -L --option builders ""`.
- `nix build .#checks.x86_64-linux.native-system-extension-octet-deny-all --no-link -L --option builders ""`.
- The strict Octet result was `Status: clean`, with zero findings, warnings, and errors.
- Cairn validation and the proposal, design, and tasks gates passed with the current Cairn default policy.
- Cairn sync merged all 14 native-host requirements into `.cairn/specs/system-extension-runtime/spec.md`.

## Failure coverage

The native executor tests reject malformed canonical output, output floods, timeouts, cancellation, nonzero exits, missing executables, stale ingress, wrong ALPN, and unavailable profiles.

Pure tests reject incomplete executable evidence, stale generation, duplicate completion, incompatible recovery state, unresolved removal, missing authority, and missing non-claims.

The separate-process fixture proves parent-observed callbacks, effect intent and routing, completion callbacks, checkpoint, restart recovery, drain, stop, removal, offline verification, and tamper denial.

## Traceability

The current Cairn default Tracey profile reports unrelated inherited gaps across the repository. It reports no uncovered or untested `molten.system_extension.native_host.*` requirement after this change.

The checked-in Aspen policy is not parseable by the current Cairn CLI because it contains `task_marker_policy.markers`. The current Cairn default policy was used for lifecycle validation.

## Broader check caveats

The repository-wide release dependency Nix check cannot execute its `molten-release-policy` binary from the pinned unit2nix package projection. Direct release validation also finds a stale, missing Octet archive path in the pre-existing release profile.

These failures are outside the native-host contract. Focused tests, Clippy, Nickel, architecture, profile, strict Octet, and lifecycle gates pass.

## Claim boundary

The evidence proves a bounded local separate-process pilot for the exact source and profile cohort. It does not prove sandboxing, hermeticity, executable trust, callback correctness, effect success, transport delivery, distributed availability, or production readiness.
