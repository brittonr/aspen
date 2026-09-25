# Proposal: Octet burn-down, ambient clock and structural-scan recursion

## Why

After the collection-growth slices, the only critical Octet family left in the safety set is `ambient_clock`: 17
findings in 9 distinct sites. It drives the strict gate's `no-critical-findings` failure together with the families
already cleared. `no_recursion` (1 site) is in the same safety slice. Its repair is ready and differentially tested.

## What Changes

- `src/fabric_time/adapters.rs` is the documented live-clock capability boundary. `LiveClockAdapter::new` anchors the
  monotonic origin and `observe_wall` reads the host wall clock. Each gets one item-level
  `#[allow(tigerstyle::ambient_clock, reason = …)]` that names that boundary. These are the only allows.
- `LiveClockAdapter::await_ticks` measures its conformance wait through its own `now_ticks`, using a new
  `TickDeadline` port helper instead of `Instant::now`.
- New `TickDeadline`, generic over `TimerClockAdapter`, and `SupervisionDeadline`, which runs on a `LiveClockAdapter`
  admitted with a named process-supervision live time profile capped at one hour. The six child-process supervision
  timeouts now poll `SupervisionDeadline` instead of reading `Instant::now`: two in
  `cluster_harness/fabric_transport.rs`, one in `cluster_harness/runner.rs`, one in the Raft `live_process`, and two in
  the Sightglass `wasm/performance/runner.rs`.
- `src/preserves/parts/rail/p001/body.rs`: `visit_structural_value` walks with an explicit stack of open containers. The
  stack is bounded through `crate::bounded::push_bounded` by `max_depth`. Children past the remaining node budget are
  not materialized.

## Impact

- **Files**: `src/fabric_time/{adapters.rs,tests.rs}`, the four supervision files, and the Preserves rail part p001.
- **Testing**: the pinned Octet root and lib runs; fmt; clippy `-D warnings`; focused and workspace tests; the harness
  and simulation fixture comparisons; the flake checks for touched surfaces.

## Out of Scope

- `ambient_env`, `unbounded_channel`, and `usize_in_public_api` are left for a later slice. The operator parked the
  `ambient_env` rework and the transcripts/secrets `--scratch-root` CLI change.
- Accepted specifications do not change. Timeouts keep their values and comparisons (`elapsed >= timeout` is exactly
  `remaining == 0`), and the structural scan keeps its preorder, first match, and first bound failure.
