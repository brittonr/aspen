# Proposal: Exercise simulation faults causally

## Why

In `DeterministicSimulationPortRouter::route`, an active fault changes the resource count, the output hash, and the emitted event, and then the router still returns a successful `PortEffectOutput`. A delay does not create a pending completion, a drop does not prevent delivery, and a storage fault does not change a modeled durable image. The fault is represented in the record but does not change execution.

The reference runner also always supplies `None` to the choice selector, so the default sorted-first selection runs every time: different seeds cannot explore different schedules. The replay helper compares choice positions and ids only. The shrink predicate checks a workload label instead of rerunning the candidate, so minimization does not prove the failure survives.

FoundationDB's simulation works because faults strike the executing system. The structure for this already exists in `molten-core/src/fabric_simulation/`; the simulated adapters need state and the harness needs real exploration, replay, and minimization.

## What Changes

- Give the simulated transport adapter state: pending transmissions, eligibility times, drops, partitions, and healing.
- Give the simulated storage adapter state: submitted operations, completed operations, and a recoverable durable image; a crash reconstructs from that image, not from pre-crash memory.
- Require an observable causal consequence: a delayed storage completion can change whether the service acknowledges before a crash.
- Add seeded selection among causally eligible events with recorded choices, so different seeds follow different schedules.
- Make replay compare virtual time, generations, and semantic outputs, stopping at the first mismatch.
- Make minimization rerun each candidate and keep it only when it reproduces the same failure fingerprint.
- Connect fault phases to the existing world-fault contract instead of a second taxonomy.

## Impact

- **Files**: `molten-core` simulation adapters, composition runner, replay and shrink helpers, reference world fixtures, and tests.
- **Testing**: delayed-completion-changes-ack acceptance test, drop and partition healing, crash reconstruction from the durable image, seed divergence, replay divergence detection, and shrink reproduction fidelity.
- **Non-goals**: no live-network equivalence claim, no filesystem-emulation claim, and no benchmark claim for Redb, Iroh, or OS behavior.

## Dependencies

- `add-protocol-aware-simulation-oracles` (independent evaluation) — compatible, not required.
- `harden-world-replay-observation-boundary` (replay nondeterminism inventory) — compatible, not required.
- Existing world-fault semantic phase contracts.
