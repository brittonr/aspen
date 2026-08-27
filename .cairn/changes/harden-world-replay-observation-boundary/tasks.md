## Phase 1: Inventory and observation core

- [ ] [depends:add-world-commit-replay-capsules] Record baseline replay-capsule, hostcall, effect-log, scheduler, time, entropy, and first-divergence outcomes. r[molten.world_replay_boundary.verification]
- [ ] [serial] Define closed source inventory and handling classes for deterministic, simulated, recorded-observation, and unsupported inputs. r[molten.world_replay_boundary.inventory]
- [ ] [serial] Define canonical observation, logical-position, request-binding, protection, completeness, blocker, and receipt DTOs. r[molten.world_replay_boundary.observations]
- [ ] [depends:world-replay-observation-dtos] Implement pure inventory closure, count, order, request, profile, cohort, and protection validation. r[molten.world_replay_boundary.completeness]

## Phase 2: Runtime and effect adapters

- [ ] [depends:world-replay-observation-core] Bind existing clock, entropy, scheduler, asynchronous delivery, hostcall, external-read, and effect-log paths to declared source IDs. r[molten.world_replay_boundary.inventory]
- [ ] [serial] Implement replay adapters that return sealed recorded observations and deny original external effects. r[molten.world_replay_boundary.effect_sealing]
- [ ] [parallel] Bind schedule and asynchronous delivery decisions to explicit logical order and reject wall-clock arrival as hidden ordering authority. r[molten.world_replay_boundary.ordering]
- [ ] [parallel] Add content-addressed trace members and validate identity before any reflink or deduplication optimization. r[molten.world_replay_boundary.trace_identity]
- [ ] [depends:add-native-process-record-replay-profile] Add detached ChaosControl native-process descriptors as optional opaque diagnostics only. r[molten.world_replay_boundary.opaque_native]

## Phase 3: Verification and documentation

- [ ] [parallel] Add positive deterministic, simulated, recorded external-read, asynchronous-order, sealed-effect, stable-replay, and opaque-diagnostic fixtures. r[molten.world_replay_boundary.verification]
- [ ] [parallel] Add negative unknown source, omitted row, missing result, wrong request, wrong order, duplicate event, extra observation, unsupported source, profile drift, adapter drift, secret, repeated effect, tamper, and overclaim fixtures. r[molten.world_replay_boundary.verification]
- [ ] [serial] Document the semantic host boundary, source classes, ordering, effect sealing, opaque diagnostics, storage optimization, and non-claims. r[molten.world_replay_boundary.claims]
- [ ] [depends:world-replay-boundary-verification] Run focused replay tests, Octet, Clippy with warnings denied, Cairn validation and gates, and relevant Nix checks. r[molten.world_replay_boundary.verification]
