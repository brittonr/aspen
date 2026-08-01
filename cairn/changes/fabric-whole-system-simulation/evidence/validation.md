# Validation evidence

## Scope

This change adds deterministic whole-system simulation for ordinary Molten system extensions.
It does not claim live transport, live disk, operating-system timing, production scale, or production readiness.
It does not claim FoundationDB, Kafka, or external scheduler compatibility.

## Prerequisite review

All declared prerequisites are archived in this repository:

- `cairn/archive/2026-07-11-fabric-time-scheduler-runtime/`
- `cairn/archive/2026-07-11-system-extension-service-runtime/`
- `cairn/archive/2026-07-12-fabric-durable-state-ports/`
- `cairn/archive/2026-07-12-fabric-membership-placement-runtime/`
- `cairn/archive/2026-07-12-fabric-transport-session-runtime/`
- `cairn/archive/2026-07-12-receipt-first-cluster-harness/`
- `cairn/archive/2026-07-24-fabric-consistency-service-runtime/`

The prior blocker is resolved. Deterministic and live consistency profiles remain separate claims.

## Baseline

Pueue task `7298` ran `nix develop -c cargo test --workspace` before implementation.
The workspace test suite passed.

Pueue task `7288` ran repository validation and the proposal, design, and tasks gates.
All commands passed before implementation.

- Proposal receipt: `979e6e52e44605d916c46dbd85a02642421a9dd6acb09f43fa4358da5f5d40e8`
- Design receipt: `de388f19b156b4a469534dbd9f48848fd686d58405ed0cad542f31821275534b`
- Tasks receipt: `512a88d133c703ff7d3bbf56303f817b15439303af4253d22a833bdaeaea9b8e`

An earlier direct `cargo` attempt lacked the development-shell toolchain.
It made no repository change and supplied no baseline claim.

## Focused implementation validation

Pueue task `7323` ran formatting, focused core and composition tests, CLI-shell tests, CLI integration tests, and focused Clippy.
All commands passed.

The focused suites include:

- 13 pure-core simulation tests
- 6 canonical composition tests
- 3 CLI shell-planning tests
- 2 positive and negative CLI integration tests

Coverage includes world admission, same-core identity, all named fault kinds, scheduler replay, first divergence, invariants, claim promotion, and shrinking.
It also includes all three reference service cores and tampered-report denial.

Pueue task `7314` exported the positive Nickel profile.
It also proved that every negative Nickel fixture fails export.

Pueue task `7315` ran the positive and negative CLI integration tests.
Result: `2 passed; 0 failed`.

Pueue task `7323` ran Clippy for `molten-core` and `molten` across all targets after the final claim-evidence code change.
The command exited successfully.

The implementation commit hook in task `7332` ran `cargo fmt --check` and `cargo octet check` inside the Nix development shell.
The commit completed only after both hooks passed.

## Executable witness

Pueue task `7309` ran the final reference fixture through the CLI.
It then replayed and inspected the emitted report.

Observed identities:

- World: `blake3:1c6a2e7e214f8416ec0677a385f0af1958e98714d84c8329f4aca07593a5f0c4`
- Run: `blake3:0933689046412a0d11edb05ac009e436ab41fc2971bf3085f425f4119a64a558`
- Bundle: `blake3:e838d6af3303e8234cadcde447d9e373261e4b1a0f81b3325bbdb82d942a7bb1`

The report recorded six choices, six observations, twelve invariant results, three final states, and no divergence.
The decision was `pass` under the `deterministic-whole-system` profile.

The run emitted 78 canonical port events.
Each of the six requests crossed all thirteen deterministic fabric port classes.

The three service slices used the ordinary `SystemExtensionHost` request callback and typed-effect routing path.
No node-core database, log, or scheduler branch was added.

## Claim-ladder decision

The differential artifact compares shared command and event contracts only.
It records distinct simulation and reviewed-live profile refs.
It does not claim live execution equivalence.

Promotion from deterministic simulation to multi-process live fails without profile-specific environment, lifecycle, and operator evidence.
Host-chaos promotion also requires fault evidence from that profile.

## Broad validation

Pueue task `7316` ran `nix develop -c cargo test --workspace` after implementation.
The workspace test suite passed.

Pueue task `7317` ran `nix develop -c cargo clippy --workspace --all-targets -- -D warnings`.
The workspace Clippy rail passed.

Pueue task `7318` ran `nix flake check --no-build --no-write-lock-file`.
All flake outputs and checks evaluated successfully.

Pueue task `7325` ran the full `nix flake check -L --no-write-lock-file` rail.
All checks passed, including the dogfood release-evidence path.

Pueue task `7319` ran repository validation, proposal gate, design gate, tasks gate, and pre-sync Tracey coverage.

- Repository validation returned `valid: true` with no issues.
- Proposal receipt: `f33249db320fe448b57d27cfaa5c6cd6ced6682ba7c0c1230e308aa332f40f96`.
- Design receipt: `831b4ae5718acb135ef295e13def3087e4c09437055a6f8cd34315ea9020f45e`.
- Tasks receipt with 16 completed tasks: `ef3158f312bf6a236c42b87e04a4c069229e5f3c2a63530c11a5ee41b8ce3e6f`.
- Pre-sync Tracey receipt: `9db68a9507b8018b3634e723dbcc9acd9152b87301d2cc090adaee59120ecfdb`.

Tracey reports `molten.fabric_simulation.claim_ladder` as dangling before sync.
That requirement remains in the active delta spec and is not accepted yet.
Repository-wide Tracey coverage also fails for unrelated existing requirements.

Pueue task `7337` ran the positive and negative source-boundary tests.
All three tests passed.
The tests bound their source scan, reject reference-service branches in node-core source, and reject ambient I/O dependencies in the pure simulation core.

Pueue task `7375` ran final formatting, workspace tests, and workspace Clippy after the source-boundary validation was added.
All commands passed.

Pueue task `7379` ran the final pre-sync Cairn validation and gates.

- Repository validation returned `valid: true` with no issues.
- Proposal receipt: `46506d00d34323f7699261b5a1f2cb723364acd8c2a9b8424ebf8f12b3f6da3b`.
- Design receipt: `dae7a5aae7338037f9833439ae15843fbfe52da004ceb2180a2f78bb8eec161c`.
- Tasks receipt: `8394f88de0a3a4dd17dfdc77db0edad10b7d267cfba944006a16d26270b882d4`.
- Task state: 18 completed; 0 incomplete.

Sync, post-sync coverage, archive, and post-archive validation remain pending.
