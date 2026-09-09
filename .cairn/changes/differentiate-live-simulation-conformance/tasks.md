# Tasks: Differentiate live and simulation conformance

## Shared workload execution

- [ ] [serial] Define the bounded shared workload type with per-request declared permitted outcome classes derived from the port contract schema sets. r[molten.fabric_simulation.live_differential]
- [ ] [serial] Execute the workload against the simulated and live adapters through one host abstraction and normalize outcomes to classes. r[molten.fabric_simulation.live_differential]
- [ ] [parallel] Add agreement and unpermitted-class fixtures plus malformed-workload and class-normalization negatives. r[molten.fabric_simulation.live_differential]

## Result taxonomy and evidence

- [ ] [serial] Add the agreement, permitted-divergence, and incomplete differential results and keep incomplete results from counting as agreement. r[molten.fabric_simulation.differential_results]
- [ ] [parallel] Add unsupported-behavior, unavailable-adapter, and permitted-divergence fixtures. r[molten.fabric_simulation.differential_results]
- [ ] [serial] Keep the structural differential as a separate artifact and bind both sides, workload, decisions, incompleteness, and non-claims into evidence. r[molten.fabric_simulation.differential_evidence]
- [ ] [parallel] Add evidence identity and binding fixtures, including a negative where the two artifacts share one identity. r[molten.fabric_simulation.differential_evidence]

## Validation and closeout

- [ ] [serial] Run focused differential tests, formatting, Clippy, Octet, Cairn validation, and the proposal, design, and tasks gates. r[molten.fabric_simulation.live_differential] r[molten.fabric_simulation.differential_results] r[molten.fabric_simulation.differential_evidence]
- [ ] [serial] Retain the no-live-equivalence, no-benchmark, and no-release-eligibility non-claims before sync or archive. r[molten.fabric_simulation.differential_evidence]
