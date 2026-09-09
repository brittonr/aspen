# Tasks: Exercise node processes under faults

All tasks are proposed work. This package grants no implementation permission.

- [ ] [serial] Record the available node entry points, transport and storage configuration surfaces, existing harness tiers, and the consensus-path exclusion boundary. r[molten.outside_in.real_stack]
- [ ] [serial] Add the harness scenario shape: setup on real processes, declared fault phase, bounded budgets, typed results, and owned teardown. r[molten.outside_in.real_stack]
- [ ] [serial] Add the no-fault baseline scenario over real Iroh transport and real Redb stores through public entry points. r[molten.outside_in.real_stack]
- [ ] [serial] Add the kill-and-restart, pause-and-resume, and lost-response fault profiles with declared boundaries and same-storage restart. r[molten.outside_in.fault_profiles]
- [ ] [serial] Add the partition-and-heal profile on the real transport with bounded heal timing. r[molten.outside_in.fault_profiles]
- [ ] [parallel] Add externally observed history collection through public interfaces and agreement checking, with protocol projections collected where available. r[molten.outside_in.observation]
- [ ] [parallel] Add negative and boundary fixtures: startup failure, budget exhaustion, unsupported behavior recording, incomplete observation, and scratch-path teardown failures. r[molten.outside_in.validation]
- [ ] [serial] Update testing docs with the outside-in tier, the simulation complement non-claim, and the consensus-path exclusion. r[molten.outside_in.observation]
- [ ] [serial] Run the baseline and each fault profile as bounded harness scenarios, then focused Octet, Clippy, workspace, Nix, and required Cairn gates on repository code. r[molten.outside_in.validation]
