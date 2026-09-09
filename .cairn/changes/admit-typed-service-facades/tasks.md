# Tasks: Admit typed service, state-machine, and task facades

## Facade admission

- [ ] [serial] Add the three facade manifest profiles with their admitted knobs (event/state schema bindings, phase table, postponement bounds, completion schema) and reject illegal tables, unbounded postponement, and unfenced timers at admission. r[molten.system_extension.service_facades]
- [ ] [parallel] Add negative admission fixtures for each rejected shape and positive fixtures for one minimal manifest per profile. r[molten.system_extension.service_facades]

## Lowering

- [ ] [serial] Implement the pure lowering function per profile and the envelope lowering into the existing canonical envelopes, lifecycle transitions, and evidence. r[molten.system_extension.service_facades]
- [ ] [parallel] Add the equivalence fixture proving facade-lowered envelopes match hand-written envelopes for the same turn. r[molten.system_extension.service_facades]

## Semantics

- [ ] [serial] Implement state-machine phase legality, incarnation- and phase-fenced state timeouts, and bounded postponement with dead-letter overflow through the existing delivery path. r[molten.system_extension.service_facades]
- [ ] [parallel] Add the stale-state timer negative case (timer from an earlier phase or incarnation rejected) and the postponed-event-plus-stale-timer combined case. r[molten.system_extension.service_facades]
- [ ] [parallel] Add the serialization fixture proving two parallel events to one logical service cannot interleave inside one state transition under a concurrency limit above one. r[molten.system_extension.service_facades]
- [ ] [parallel] Add the task fixture proving a successfully completed Task transitions to stopped without consuming restart budget. r[molten.system_extension.service_facades]

## Validation and closeout

- [ ] [serial] Run focused facade, system-extension, and addressable-actor tests, formatting, Clippy, Octet, Cairn validation, and the proposal, design, and tasks gates. r[molten.system_extension.service_facades]
- [ ] [serial] Retain the no-new-runtime, opt-in, and no-compatibility non-claims before sync or archive. r[molten.system_extension.service_facades]
