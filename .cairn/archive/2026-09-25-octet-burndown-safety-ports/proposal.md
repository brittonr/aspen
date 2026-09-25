# Proposal: Octet burn-down, bounded channels and fixed-width public integers

## Why

After the critical safety families were cleared, two non-critical safety families remain in the C4 set.
`unbounded_channel` has 9 sites, all in the live Raft ports and their tests. `usize_in_public_api` has 32 findings
at 16 public functions. This base is `integration/stack-20260925` (`9602f924c`, 3250 workspace findings). Both
families must reach zero by repair.

## What Changes

- Channels:
  - The live Raft replica event inbox and the supervision control channel become bounded `tokio::sync::mpsc`
    channels.
  - The inbox capacity is the admitted time profile's `max_scheduler_queue_depth`, and a zero depth is denied. The
    control capacity is a named `LIVE_CONTROL_OBSERVATION_CAPACITY` whose rationale is recorded next to it.
  - Timer tasks `send(..).await`: a full inbox delays a timer event and never drops it.
  - The synchronous control port uses `try_send`. A full queue returns an observable error ("live Raft supervision
    queue is full") and publishes nothing. A closed receiver keeps its existing error.
- `usize_in_public_api`:
  - Public functions take and return `u64`/`u32`: `call_count`, `request_count`, `canonical_content_command`,
    `bounded_content_status`, `DeterministicSimulationSink::new`, `detect_flapping_probes`,
    `MaterializationPath::parse`, `MaterializationPolicy::with_bounds`, `simple_record_fields`,
    `record_content_ref_sequence`/`_strings`, `parse_checks_record`, `BoundarySchemaSpec::arity`,
    `AdmissionPhase::index`, and `canonical_world_sync_receipt`.
  - Conversions go through two checked helpers in `crate::bounded` (`usize_from_u64`, `u64_from_usize`). Each
    returns a labelled error when a value does not fit, and nothing uses `as`.
  - Call sites that already hold the host `usize` policy bound use the new crate-private
    `MaterializationPath::parse_within`.
  - Record-arity constants used only as arities become `u64`.

## Impact

- **Files**: 38 files in `src/`.
- **Testing**:
  - Pinned Octet root and lib runs.
  - fmt, and clippy `-D warnings`.
  - Full `cargo test --workspace`.
  - Harness, fabric-simulation, and `fabricboundarycompat` byte identity against the base binary.

## Out of Scope

- The public signatures change their integer types, but no public item is renamed. `detect_flapping_probes` now
  returns `Result`, so a probe count that does not fit `u64` surfaces as an error.
- Encoded values do not change: `u64_value` replaces the equivalent `usize_value` conversion.
- `ambient_env` stays parked.
