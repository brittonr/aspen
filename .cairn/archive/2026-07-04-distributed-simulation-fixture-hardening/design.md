# Design: distributed simulation fixture hardening

## Scope

Harden the existing deterministic simulation test surface. The core simulator remains a pure function over declared topology, scheduler profile, deterministic seed, fault plan, workflow commands, and evidence refs. The change adds fixtures and profile wiring checks around the existing core rather than introducing a network emulator or VM runner.

## Proof checklist

- **Proof claim**: every supported simulation fault class has a named positive or negative fixture that proves its expected deterministic decision and receipt shape.
- **Positive evidence**: admitted commands under benign delay, drop, reorder, rejoin, crash, restart, and duplicate-delivery cases emit stable pass receipts, stable final-state refs, and explicit diagnostics.
- **Negative evidence**: stale evidence, corrupted receipts, resource pressure, unauthorized transport evidence, undeclared ambient state, and partitioned quorum faults deny before side effects with no committed operation for the denied command.
- **Profile evidence**: distributed CI metadata fixtures bind configured profile ids, command surfaces, expected artifacts, retry policy, unavailable handling, and variance declarations from `default_distributed_ci_profiles` or its repository-owned successor.
- **Out of scope**: QEMU/systemd behavior, live Iroh transport, WAN timing, production soak claims, and any rule that treats simulation pass receipts as authority, policy, provenance, resource, source-gate, retention, deployment, or transport trust.

## Functional core

Keep simulator decisions in the pure `run_distributed_simulation` path. Fixture helpers should construct deterministic in-memory inputs and compare returned values. If profile metadata construction needs new logic, implement it as a pure builder over explicit profile/check inputs and keep filesystem/Nix/output discovery in the shell or test fixture.

## Imperative shell

The shell remains limited to test execution and optional receipt readback. Commands such as `cargo test --lib distributed` and `cargo nextest run --profile deterministic` may provide validation evidence, but the fixture assertions should not depend on live files, clocks, process ids, environment variables, network availability, or retry behavior.

## Validation strategy

Run the smallest relevant checks first:

```sh
cargo test --lib distributed
cargo nextest run --profile deterministic
```

If the implementation touches Nix check wiring or release metadata, also run the relevant Nix check that owns the changed surface.
