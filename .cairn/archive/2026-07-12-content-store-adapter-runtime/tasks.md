## Phase 1: Content adapter primitives and ports

- [x] [serial] Define canonical content-store and content-exchange descriptors, commands, events, handles, completion classes, optional capabilities, resource bounds, and non-claims. r[molten.content_store_adapter.port_contract] r[molten.content_store_adapter.partial_state]
- [x] [serial] Extract or preserve pure manifest identity, chunk/range planning, missing-set calculation, streaming verification, partial-state, and protection/retention decisions without adapter imports. r[molten.content_store_adapter.identity_boundary] r[molten.content_store_adapter.verify_before_available] r[molten.content_store_adapter.retention_boundary]
- [x] [parallel] Add positive and negative primitive tests for stable identity, ranges, missing chunks, corruption, unsupported transforms, partial state, uncertain outcomes, and retention boundaries. r[molten.content_store_adapter.identity_boundary] r[molten.content_store_adapter.verify_before_available] r[molten.content_store_adapter.partial_state]

## Phase 2: Live and deterministic adapters

- [x] [serial] Convert capability-rooted local content and Redb index operations to the canonical adapter contract with bounded streaming and cancellation. r[molten.content_store_adapter.port_contract] r[molten.content_store_adapter.streaming_bounds]
- [x] [serial] Add a live Iroh content adapter that streams manifests and chunks, preserves Molten identity, and exposes backend tickets, hashes, tags, and providers only as hints. r[molten.content_store_adapter.identity_boundary] r[molten.content_store_adapter.verify_before_available]
- [x] [parallel] Add a deterministic adapter that models partial transfer, cancellation, latency, corruption, capacity, restart, and uncertain outcomes through the same command/event contract. r[molten.content_store_adapter.partial_state] r[molten.content_store_adapter.live_sim_conformance]

## Phase 3: Integration and safety

- [x] [serial] Wire artifact, typed-storage, replay, operator range-read, DAG, and future replication callers through content ports rather than concrete backend APIs. r[molten.content_store_adapter.port_contract]
- [x] [parallel] Map backend protection handles to canonical retention operations without treating tags, pins, or unprotect as deletion or read authority. r[molten.content_store_adapter.retention_boundary]
- [x] [parallel] Add bounded operator readback for canonical availability, verified partial state, adapter profile, active operations, resources, failures, and redacted backend hints. r[molten.content_store_adapter.partial_state]

## Phase 4: Conformance and validation

- [x] [serial] Run shared live/local/simulation conformance with positive store, stream, range, resume, restart, and cancellation cases and negative corruption, truncation, reordering, unexpected chunk, stale ticket, transform, root escape, overload, retention, and secret-leak cases. r[molten.content_store_adapter.live_sim_conformance] r[molten.content_store_adapter.final_validation]
- [x] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and the smallest relevant Nix checks before sync and archive. r[molten.content_store_adapter.final_validation]
