# Bounded execution fabric port

r[impl molten.fabric_execution.component_pin] r[impl molten.fabric_execution.port_contract] r[impl molten.fabric_execution.authority] r[impl molten.fabric_execution.request] r[impl molten.fabric_execution.environment] r[impl molten.fabric_execution.lifecycle] r[impl molten.fabric_execution.output] r[impl molten.fabric_execution.generation] r[impl molten.fabric_execution.uncertainty] r[impl molten.fabric_execution.simulation] r[impl molten.fabric_execution.validation] r[impl molten.fabric_execution.nonclaims]

Molten exposes bounded process execution as one application-owned fabric port. It does not treat process execution as transport, scheduling, or supervision.

## Component cohort

The live adapter uses `bounded-exec` revision `29dac88ecded94457572db3fdfaaaab95fa91525`.

Cargo and Nix use the canonical read-only source:

```text
https://git.onix.computer/z2CpqLFpdP36fZXYUK5ZNWxMibpCo.git
```

The source uses `AGPL-3.0-or-later`. Molten records the repository, revision, package, license, platform, and non-claims in one canonical source cohort.

A mutable sibling path is not release evidence. The Nix gate rejects a path dependency for `bounded-exec`.

## Authority and admission

A request must bind these facts before process start:

- executable artifact and measured identity;
- executable, process, workspace, and effect authority;
- provenance and policy evidence;
- extension, service, callback, effect, operation, generation, and idempotency identities;
- explicit arguments and environment entries;
- capability-rooted workspace and input references;
- timeout, input, output, polling, teardown, concurrency, and queue bounds;
- termination scope and exit observation policy.

Artifact bytes or a host path do not grant execution authority. The pure core rejects incomplete or substituted facts before path resolution.

## Environment and platform

The live profile clears the inherited environment. It adds only the admitted public entries.

The profile rejects environment inheritance, shell-mode requests, path search, implicit current directories, duplicate keys, and secret environment values.

The first live profile uses Unix process-group teardown. Other platforms must select a different reviewed profile. Direct-child teardown does not prove descendant teardown.

## Output handling

`bounded-exec` retains a bounded prefix for standard output and standard error. Each stream records observed bytes, retained bytes, and truncation.

A selected output publisher stores each retained prefix through a content-store boundary. The execution receipt stores content and publication receipt references. It does not store raw output bytes.

If publication fails, the port returns a typed publication failure. The failure retains the completed process observation and canonical receipt.

## Lifecycle and recovery

The lifecycle distinguishes admitted, queued, started, exited, timed-out, cancelled, failed-before-start, failed-after-start, teardown-incomplete, and unknown states.

A spawn failure is a definite pre-start failure. A failure after process start remains unknown without definitive terminal and teardown evidence.

Unknown work does not retry automatically. Reconciliation requires the exact operation and generation.

Completion admission compares the complete identity tuple. A stale generation or substituted executable cannot reach extension state.

## Simulation

The deterministic adapter consumes the same canonical request and output publication contract. It does not spawn a process.

Scripts can model exit, timeout, cancellation, start refusal, truncation, and unknown completion. Equal profiles, requests, scripts, and publishers produce equal canonical receipts.

## Operator status

A terminal receipt describes one bounded process observation. An accepted exit code means only that the request policy accepted the observed exit.

The receipt does not claim:

- sandboxing or hermeticity;
- executable trust or child correctness;
- network isolation;
- platform equivalence;
- application success;
- release readiness.
