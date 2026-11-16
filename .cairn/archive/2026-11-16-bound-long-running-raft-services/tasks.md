## Phase 1: Implementation

- [x] [serial] Classify reliability and collaboration Raft services as long-running and document their bounds and stop conditions. r[molten.consensus.long_running_service_bounds]
- [x] [serial] Add the validated snapshot threshold and host-issued threshold snapshots for reliability and collaboration, surfacing snapshot failures. r[molten.consensus.snapshot_threshold]
- [x] [serial] Bound `completed_requests` to the deterministic applied-position window; carry, restore and validate it through snapshots. r[molten.consensus.bounded_request_dedup]
- [x] [serial] Add authority-gated snapshot retirement and covered truncation to the durable core and adapters; compact the Raft durable log in crash-safe order and accept compacted prefixes in recovery. r[molten.consensus.durable_log_compaction] r[molten.fabric_durability.snapshot_compaction]
- [x] [serial] Replace the lifetime ingress limit with a windowed budget, drop and count undecodable frames, and make host connections a concurrency bound. r[molten.consensus.long_running_service_bounds]

## Phase 2: Verification

- [x] [serial] Run focused tests, formatting, Clippy and strict Cairn validation before review, sync and archive. r[molten.consensus.long_running_service_bounds] r[molten.consensus.snapshot_threshold] r[molten.consensus.bounded_request_dedup] r[molten.consensus.durable_log_compaction] r[molten.fabric_durability.snapshot_compaction]

The [final ordered source checks](../../../evidence/integrated-cutover-2026-11-16/checks.json) passed both configured Octet producers, current object-corpus/import/strict gate, complete workspace library tests, Clippy, formatting and strict provider-policy Cairn validation. The [earlier complete workspace execution](../../../evidence/integrated-cutover-2026-11-16/earlier-checks.json) remains separate, including its subsequently repaired Clippy failure. No physical-host-loss or cross-host qualification follows from these local controls.
