## Phase 1: Contract and composition

- [x] [serial] Pin the reviewed Rivet Actors source revision, record its license and selected design concepts, and add the bounded reference to `README.md`. r[molten.addressable_actor.profile]
- [x] [serial] Define the canonical actor key, profile, placement, generation, lifecycle, wake-reason, survival-matrix, and status schemas. r[molten.addressable_actor.profile] r[molten.addressable_actor.survival]
- [x] [depends:coordination-delivery-system-extension] Add pure lifecycle and wake planning over admitted delivery, durable-state, time, placement, resource, supervision, policy, and evidence facts. r[molten.addressable_actor.lifecycle] r[molten.addressable_actor.delivery]
- [x] [depends:molten.addressable_actor.lifecycle] Add the thin system-extension shell for checkpoint, sleep, wake, restore, drain, and status effects with generation rechecks before every effect. r[molten.addressable_actor.lifecycle] r[molten.addressable_actor.authority]

## Phase 2: Failure behavior and evidence

- [x] [parallel] Add positive fixtures for key resolution, admitted message wake, timer wake, checkpoint restore, idle sleep, and bounded drain. r[molten.addressable_actor.verification]
- [x] [parallel] Add negative fixtures for stale keys, stale generations, duplicate wakes, unsupported survival claims, missing authority, resource denial, and unknown external outcomes. r[molten.addressable_actor.verification] r[molten.addressable_actor.survival]
- [x] [depends:molten.addressable_actor.lifecycle] Add deterministic simulation, restart, and multiprocess evidence for the supported lifecycle matrix. r[molten.addressable_actor.verification]
- [x] [serial] Run focused tests, simulation rails, Cairn validation and gates, then record exact non-claims before implementation completion. r[molten.addressable_actor.verification]
