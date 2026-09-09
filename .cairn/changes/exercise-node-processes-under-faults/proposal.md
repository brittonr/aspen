# Exercise node processes under faults

## Why

Deterministic simulation tests modeled adapters. Real-process testing challenges the assumptions those adapters make. The existing fault work covers this only for consensus: `add-chaoscontrol-consensus-conformance` runs bounded KVM campaigns over the production-shaped consensus path, and `add-live-consensus-reliability-adapter` runs live reliability profiles for consensus clusters. The non-consensus node paths — promotion, content replication, coordination delivery, and the public entry points that drive them — have no tracked change that runs real node processes over real Iroh transport and real Redb stores, kills or pauses processes at controlled boundaries, restarts with the same storage, and checks externally observed histories.

The TigerBeetle Vortex review (2026-09) identified this outside-in track as a complement to deterministic simulation, not a substitute. Neither covers the other's failure classes. This proposal is a design review finding; no whole-process fault campaign was executed.

## What Changes

- Add a bounded outside-in test track that drives ordinary release-shaped node processes through public entry points over actual Iroh transport and actual local Redb stores. r[molten.outside_in.real_stack]
- Add controlled process kill, pause, and restart profiles at stated boundaries, with restart on the same storage and no repaired or reset state. r[molten.outside_in.fault_profiles]
- Check externally observed histories through public interfaces, alongside available protocol projections where the simulation oracles supply them, without comparing physical timing. r[molten.outside_in.observation]
- Record unsupported behaviors and incomplete observations as typed results; retain the simulation and live non-claims from the conformance work. r[molten.outside_in.validation]

## Impact

- **Files**: a bounded test harness surface under the existing testing infrastructure, node entry-point fixtures, transport and storage configuration for the harness, docs.
- **Testing**: no-fault baseline, kill/restart, pause/resume, lost-response, and partition-and-heal profiles; externally observed history agreement; malformed and boundary negatives.
- **Non-goals**: no production admission claim, no replacement of deterministic simulation, no consensus-path duplication of the ChaosControl campaigns, no performance or benchmark claim.

## Dependencies

- `add-chaoscontrol-consensus-conformance` and `add-live-consensus-reliability-adapter` own the consensus-path campaigns; this change excludes the consensus path to avoid scope overlap.
- `exercise-simulation-faults-causally` and `add-protocol-aware-simulation-oracles` supply the simulation side and projections this track complements.
