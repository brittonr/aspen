# Design: distributed simulation fault plans

## Scope

Add a deterministic simulation harness for distributed behavior. The simulation is not a network emulator; it is a pure model over declared peers, queues, logical clocks, actor/node state summaries, and explicit fault events.

## Proof checklist

- **Proof claim**: for a given topology, seed, scheduler policy, and fault plan, the simulator emits stable canonical decisions and receipts for distributed workflow invariants.
- **Out of scope**: real kernel networking, QEMU/systemd behavior, WAN correctness, adversarial transport security, and treating simulation pass evidence as authority or deployment approval.
- **Trusted assumptions**: modeled handlers accurately encode the intended workflow boundary; BLAKE3 refs identify canonical model inputs and outputs.
- **Positive evidence**: same-input/same-seed simulations produce identical run refs; valid workflows pass under bounded delay/reorder/duplication where invariants permit.
- **Negative evidence**: stale authority, duplicate operation misuse, undeclared ambient state, unauthorized transport evidence, partitioned quorum assumptions, and missing replay data deny before side effects.
- **Canonical refs**: simulated topology ref, fault-plan ref, seed ref, scheduler profile ref, model input refs, child workflow refs, distributed run receipt ref, and first-divergence ref.
- **Regeneration command**: a focused simulation command or Nix check that runs the positive and negative distributed simulation fixtures.

## Functional core

The core should be a pure transformation:

`SimulationInput -> SimulationOutcome`

Inputs include topology, deterministic seed, scheduler profile, fault plan, initial model state, workflow commands, and declared evidence refs. Outputs include ordered model events, resulting state refs, pass/deny decisions, diagnostics, and canonical receipt values. The core must not read clocks, files, network, process state, random sources, or environment variables.

## Imperative shell

The shell may load fixture files, parse CLI arguments, write receipts, and invoke the core. It should remain thin enough that core behavior is tested without spawning nodes or VMs.

## Evidence boundary

Simulation receipts are review and regression evidence. They do not grant authority, policy, provenance, resource, source-gate, retention, transport, destructive-operation, or deployment trust. VM and live pilot evidence remain required for claims about platform and transport behavior.
