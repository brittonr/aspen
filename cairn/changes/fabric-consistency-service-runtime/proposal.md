## Why

Aspen already specifies pluggable consensus profiles, normalized engine interfaces, pure state machines, membership admission, snapshots, and evidence. The implementation still identifies an `in-process-raft-control-registry-v1` model as production-admitted, while no live multi-process engine currently exchanges consensus traffic through node transport, persists its own durable log through a fabric port, drives elections from admitted timers, or demonstrates quorum behavior under process and network failures.

System extensions also need to consume optional consistency groups without moving their database, log, scheduler, or workflow semantics into Aspen core.

## What Changes

- Broaden explicit consistency-group scope from control-plane applications alone to admitted system extensions while preserving opt-in use and existing non-claims.
- Run consensus engines as supervised live services over admitted transport, durable-state, time, membership, placement, fencing, and resource ports.
- Add an extension-facing consistency port for group creation or attachment, proposals, declared read modes, snapshots, recovery, membership/config transitions, and bounded status.
- Implement and validate the first live Raft service profile using distinct node processes and real transport/durable adapters.
- Reclassify the current in-process control-registry engine as model or simulation only; production admission remains denied until live quorum, durability, recovery, placement, and operational evidence passes.
- Keep application state machines and hot-path protocol traffic free from heavyweight per-message receipts.

## Impact

- **Files**: consensus profile descriptors and registry, engine runtime shell, transport/durable/time/membership bindings, system-extension consistency effects, Raft service implementation, operator readback, cluster fixtures, and `cairn/specs/consensus/spec.md`.
- **Testing**: shared engine conformance, multi-process election/commit/read, partition and quorum loss, crash/restart, durable recovery, stale leader fencing, membership changes, snapshot catch-up, model-profile denial, and extension isolation.
- **Safety**: a live profile is production-admitted only for its reviewed environment and failure model; consensus-group success does not prove extension semantics, distributed transactions across groups, Byzantine tolerance, or global ordering.
