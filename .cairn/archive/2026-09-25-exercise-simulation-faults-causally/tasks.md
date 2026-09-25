# Tasks: Exercise simulation faults causally

## Stateful simulated adapters

- [x] [serial] Record the current router behavior, pending-event modeling options, storage contract semantics including `OutcomeUnknown`, and the reference fixture baseline. r[molten.fabric_simulation.stateful_transport] r[molten.fabric_simulation.stateful_storage]
- [x] [serial] Add transport adapter state with pending transmissions, eligibility times, drops, and partitions with healing, delivering events through the live-shell scheduling boundary. r[molten.fabric_simulation.stateful_transport]
- [x] [parallel] Add transport fixtures for delayed eligibility, dropped delivery, and partition block-and-heal, plus negative unbounded-pending and partition-overflow cases. r[molten.fabric_simulation.stateful_transport]
- [x] [serial] Add storage adapter state separating submitted operations, completed operations, and the recoverable durable image, with crash reconstruction from the image only. r[molten.fabric_simulation.stateful_storage]
- [x] [parallel] Add the delayed-completion acceptance test where acknowledgment outcome changes across fault presence, plus crash-recovery and boundary fixtures. r[molten.fabric_simulation.stateful_storage]

## Exploration, replay, minimization

- [x] [serial] Replace the always-`None` runner selection with recorded seeded choice selection among causally eligible events. r[molten.fabric_simulation.causal_exploration]
- [x] [serial] Extend replay comparison to virtual time, generations, and semantic outputs with first-mismatch reporting. r[molten.fabric_simulation.causal_exploration]
- [x] [serial] Replace the label-only shrink predicate with rerun-and-fingerprint retention and remove the label-only fixture. r[molten.fabric_simulation.causal_exploration]
- [x] [parallel] Add seed-divergence, replay-divergence, and non-reproducing-candidate fixtures plus overflow and bound-exhaustion negatives. r[molten.fabric_simulation.causal_exploration]

## Integration and closeout

- [x] [serial] Connect fault phases to the existing world-fault contract phases and regenerate reference evidence refs. r[molten.fabric_simulation.stateful_transport] r[molten.fabric_simulation.stateful_storage]
- [x] [serial] Run focused simulation tests, formatting, Clippy, Octet, Cairn validation, and the proposal, design, and tasks gates. r[molten.fabric_simulation.causal_exploration]
- [x] [serial] Triage any invariant failure that seeded exploration surfaces and retain the no-live-equivalence and no-benchmark non-claims before sync or archive. r[molten.fabric_simulation.causal_exploration]
