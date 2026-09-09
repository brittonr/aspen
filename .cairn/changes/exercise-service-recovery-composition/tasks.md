# Tasks: Exercise a service recovery composition end to end

## Composition definition

- [ ] [serial] Define the three-service composition (durable stateful service, dependent worker, independent sibling) once through the typed facades and recovery-group declaration, consumable by both harness tracks. r[molten.testing.service_recovery_composition]

## Deterministic matrix

- [ ] [serial] Add the deterministic scenario matrix asserting the five properties (unaffected services continue; stale instances lose influence; durable work is not silently lost; uncertain effects are not blindly repeated; storms terminate within the window) across member failure, delayed events, readmission, saturation, overload, storm, and upgrade scenarios. r[molten.testing.service_recovery_composition]
- [ ] [parallel] Add the deliberately broken variants (fencing removed, group budget reset) and prove the harness fails each while the correct composition passes. r[molten.testing.service_recovery_composition]

## Live twin

- [ ] [serial] Add the real-process twin covering crash-restart fencing, durable readmission, storm termination, and the generation upgrade, with observations mapped to the deterministic assertion vocabulary and timing checks on admitted observations. r[molten.testing.service_recovery_composition]

## Validation and closeout

- [ ] [serial] Run the composition suite in both tracks, formatting, Clippy, Octet, Cairn validation, and the proposal, design, and tasks gates. r[molten.testing.service_recovery_composition]
- [ ] [serial] Retain the no-whole-system-correctness and no-mechanism-patching non-claims before sync or archive. r[molten.testing.service_recovery_composition]
