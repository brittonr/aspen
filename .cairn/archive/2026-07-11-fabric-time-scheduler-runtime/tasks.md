## Phase 1: Canonical time and timer core

- [x] [serial] Add distinct canonical wall, monotonic, logical, and virtual time values plus checked durations, uncertainty, profile descriptors, and non-claims. r[molten.fabric_time.time_domains]
- [x] [serial] Implement pure one-shot and periodic timer state machines with generation fencing, ordering, cancellation, lateness, coalescing, skipped-period, overload, and terminal outcomes. r[molten.fabric_time.timers]
- [x] [parallel] Add positive timer fixtures and negative mixed-domain, overflow, stale-generation, duplicate-fire, fire-after-cancel, and over-limit fixtures. r[molten.fabric_time.time_domains] r[molten.fabric_time.timers]

## Phase 2: Scheduler and entropy ports

- [x] [serial] Add the bounded runnable-scheduler command, event, queue, choice, cancellation, and replay-position contract. r[molten.fabric_time.scheduler]
- [x] [parallel] Add bounded deterministic and production entropy profiles with explicit stream ids, purpose, positions, secrecy rules, and replay semantics. r[molten.fabric_time.entropy]
- [x] [parallel] Integrate timer, scheduler, and entropy events with active system-extension generations and resource accounting. r[molten.fabric_time.scheduler] r[molten.fabric_time.entropy]

## Phase 3: Live and simulation adapters

- [x] [serial] Implement live clock, timer, scheduler-wakeup, and production-entropy shells behind the canonical ports. r[molten.fabric_time.live_sim_parity]
- [x] [parallel] Implement virtual time, deterministic runnable selection, and deterministic entropy behind the same observable contracts. r[molten.fabric_time.live_sim_parity]
- [x] [parallel] Add explicit clock-jump, scheduler-overload, delayed-fire, partition-coupled deadline, and cancellation fault injection. r[molten.fabric_time.live_sim_parity]

## Phase 4: Deadlines, leases, and evidence

- [x] [serial] Add deadline and lease decision helpers that require declared time domains, uncertainty, fencing, and consistency assumptions. r[molten.fabric_time.deadline_lease]
- [x] [parallel] Add bounded profile, anomaly, semantic-deadline, timer-leak, deterministic-run, and scheduler-replay evidence plus operator readback. r[molten.fabric_time.evidence]

## Phase 5: Validation

- [x] [serial] Run shared adapter conformance, timer ordering, cancellation, coalescing, virtual-time, deterministic replay, entropy, clock anomaly, overload, deadline, lease, stale-generation, and cleanup tests. r[molten.fabric_time.final_validation]
- [x] [serial] Run Cairn validation and proposal, design, and tasks gates before sync and archive. r[molten.fabric_time.final_validation]
