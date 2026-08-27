## Why

World replay capsules bind expected successor commits, but a transition can still consume undeclared time, entropy, scheduling, hostcall, or external-service observations.

The RR record-and-replay design provides a useful boundary rule: capture every nondeterministic value that crosses from the environment into deterministic execution. Molten has a stronger semantic boundary than a native Linux process. Its boundary is the closed Wasm hostcall and typed effect-port surface.

Molten must also separate replayed observations from repeated effects. A replay that sends a message, writes an external store, or calls a remote service again can create new behavior while appearing deterministic.

## What Changes

- Add a closed nondeterminism inventory for every replay-relevant hostcall, effect port, scheduler decision, asynchronous event, time source, entropy source, and external observation.
- Classify each source as deterministic, simulated, recorded-observation, or unsupported under one exact replay profile.
- Bind ordered observations, source identity, request identity, result, error class, logical position, protection profile, and runtime cohort into transition traces.
- Require replay adapters to return recorded observations without repeating external effects.
- Bind asynchronous delivery and schedule decisions to explicit logical order rather than wall-clock arrival.
- Fail closed on unknown hostcalls, missing observations, reordered or duplicate events, unsupported sources, profile drift, and attempts to execute sealed effects.
- Keep native process `rr` traces as optional ChaosControl opaque diagnostics. They do not define Molten semantic replay.
- Use content-addressed immutable trace members. Do not treat paths, hard links, or reflinks as identity.

## Dependencies

- `add-world-commit-replay-capsules`.
- `bind-world-promotion-to-effect-release`.
- `add-world-execution-snapshot-profiles`.
- Existing Molten hostcall, effect-log, deterministic scheduler, virtual-time, and entropy contracts.
- ChaosControl `add-native-process-record-replay-profile` for optional opaque diagnostics.

## Non-Goals

- Reimplementing Linux `ptrace`, syscall interception, or hardware-counter replay inside Molten.
- Replacing ChaosControl VM replay or claiming semantic equivalence with native process replay.
- Repeating external effects during replay.
- Universal determinism across arbitrary hosts, kernels, runtimes, drivers, or concurrent programs.
- Treating a trace, capsule, or replay result as authority, effect completion, or release eligibility.

## Impact

- **Core**: source inventory, handling classes, observation envelopes, ordering, completeness, and replay effect-denial plans.
- **Shell**: record adapters, sealed observation adapters, profile checks, and detached opaque-diagnostic import.
- **Schemas**: canonical nondeterminism inventory, observation ledger, completeness report, and replay receipt fields.
- **Testing**: positive deterministic, simulated, and recorded cases plus negative unknown source, missing result, wrong order, duplicate event, unsealed effect, cohort drift, tamper, and overclaim cases.
