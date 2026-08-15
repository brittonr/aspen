## Phase 1: Manifest and lifecycle core

- [x] [serial] Add canonical system-extension manifests, service descriptors, callback declarations, port requirements, execution profiles, state compatibility, resources, and non-claims. r[molten.system_extension.manifest]
- [x] [serial] Implement pure lifecycle and generation state machines for install, admit, initialize, start, run, checkpoint, recover, drain, fail, restart, upgrade, rollback, and shutdown. r[molten.system_extension.lifecycle]
- [x] [parallel] Add positive manifest/transition fixtures and negative unknown-callback, incompatible-port, stale-generation, illegal-transition, and over-authority fixtures. r[molten.system_extension.manifest] r[molten.system_extension.lifecycle]

## Phase 2: Executable callback host

- [x] [serial] Add the service-instance dispatcher and invoke initialize, start, request, message, stream, timer, health, checkpoint, recover, drain, and shutdown callbacks. r[molten.system_extension.callbacks]
- [x] [parallel] Route callback effects only through generation-bound admitted fabric ports and reject ambient or undeclared effect requests. r[molten.system_extension.typed_effects]
- [x] [parallel] Add an executable fixture proving callback invocation and a negative fixture proving modeled receipts alone do not satisfy executable-host conformance. r[molten.system_extension.callbacks]

## Phase 3: Supervision, bounds, and recovery

- [x] [serial] Integrate system-extension instances with node supervision, restart policy, health reporting, drain, shutdown, and final cleanup. r[molten.system_extension.supervision]
- [x] [parallel] Enforce bounded callback concurrency, queues, bytes, streams, timers, deadlines, cancellation, and shutdown grace from admitted resource envelopes. r[molten.system_extension.backpressure]
- [x] [parallel] Add crash, timeout, cancellation, overload, failed checkpoint, failed recovery, stale callback, drain, and cleanup tests. r[molten.system_extension.supervision] r[molten.system_extension.backpressure]

## Phase 4: Profiles and evidence

- [x] [serial] Add separately admitted execution profiles without changing the typed callback and effect contract. r[molten.system_extension.execution_profiles]
- [x] [parallel] Emit canonical admission, activation, generation, checkpoint, recovery, failure, drain, and shutdown evidence with explicit non-claims. r[molten.system_extension.evidence]
- [x] [parallel] Add CLI and operator readback for active instance, generation, execution profile, bound ports, resources, health, and last lifecycle evidence. r[molten.system_extension.operator_readback]

## Phase 5: Validation

- [x] [serial] Run focused unit, integration, executable-fixture, negative-authority, resource-bound, restart, recovery, and cleanup tests. r[molten.system_extension.final_validation]
- [x] [serial] Run Cairn validation and proposal, design, and tasks gates before sync and archive. r[molten.system_extension.final_validation]
