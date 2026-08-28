# Proposal: Add a bounded execution fabric port

## Why

Molten system extensions can request transport, durable state, time, scheduling, and other typed effects. They cannot request a bounded native child process through a fabric port.

This gap blocks services that must call reviewed external tools without receiving ambient process authority. Kiln is the first planned consumer, but the missing mechanism is product-neutral.

The OnixResearch `bounded-exec` component already owns explicit argv and environment handling, bounded input and output, deadlines, cancellation, and owned teardown. Molten can adapt that mechanism without copying its source or transferring application policy.

## What Changes

- Add an application-owned bounded execution port with versioned canonical request, event, outcome, status, and receipt schemas.
- Add an `Execution` fabric-port class and explicit process-execution authority.
- Pin the reviewed `bounded-exec` source revision through immutable Nix and Cargo inputs.
- Keep executable authorization, input materialization, sandbox policy, output meaning, retry policy, and release meaning outside the adapter.
- Add a pure request and outcome admission core.
- Add a thin live adapter over the published Bounded Exec shell.
- Add a deterministic simulation adapter over the same command and event contract.
- Route execution only through exact system-extension bindings and active service generations.
- Store bounded standard output and standard error through content references instead of callback-inline logs.
- Preserve timeout, cancellation, truncation, teardown, infrastructure failure, and unknown completion as distinct observations.

## Impact

- **Core**: new pure execution request, transition, bound, and outcome types.
- **Application**: new narrow execution port and typed failure contract.
- **Adapters**: live Bounded Exec and deterministic simulation implementations.
- **Fabric registry**: new execution class, descriptor, binding, and conformance profile.
- **Configuration**: typed Nickel profiles for artifacts, environments, limits, authority, resources, and non-claims.
- **Testing**: positive and negative admission, execution, cancellation, timeout, flood, teardown, generation, simulation, and receipt tests.

## Non-goals

- Prove sandboxing, hermeticity, child correctness, executable trust, network isolation, or release eligibility.
- Authorize arbitrary host paths, inherited environments, credentials, devices, or network access.
- Define Kiln, Mantle, Cairn, Nix, or another consumer's process policy.
- Turn process completion into application success.
- Add automatic retries after uncertain or side-effecting execution.
