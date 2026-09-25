# Design: Octet burn-down, functions within the 70-line limit

## Context

`function_length` flags any function body longer than 70 lines, including `#[test]` functions, which count in the
workspace run. The 127 base sites range from +1 to +222 lines over the limit. The largest were:
- `execute_cluster_harness`: +222
- the plugin capability-grant test: +167
- `execute_live_iroh_stream_get`: +92

## Decisions

### Decision: Extract helpers that keep order and name one responsibility

**Choice:** Each long function becomes a short driver over private helpers. Each helper has a doc comment and owns one
step: plan, execute phases, record evidence, or finish. Helpers take the values the step reads and return the values
the step produces. Mutable state passes as `VecSink` or owned structs, not as new public types.

**Rationale:** Reviewers need to be able to check that the order is preserved. Where order is observable, the helper
boundaries follow the original statement order, and values that must be computed late stay late. For example:
- `executor_manifest("minimal-v2")` still runs after the removal in `run_fixture_trace`.
- `next_sequence` still runs after the chunk lookup in `simulated_chunk_observation`.
- In `execute_cluster_harness`, the phase gating, skip diagnostics, artifact push order, cleanup ordering, and the
  failure bundle condition are unchanged.

### Decision: Use `clap::Args` structs for long CLI variants

**Choice:** CLI variants with long inline field lists hold a `#[derive(Debug, clap::Args)]` struct instead.

**Rationale:** The dispatch functions were long because they destructured many fields inline. Moving the fields into
an args struct leaves every flag's name, help text, default, and conflict unchanged, and each handler receives one
value.

### Decision: Move tests and helpers into siblings instead of crossing the file limit

**Choice:**
- The concrete-port assembly test and its fixture builders move to `raft/iroh/bundle.rs`, following the existing
  `#[path = "…/tests.rs"]` convention in `raft/mod.rs`.
- `codec_checked_manifest_ref` moves to the next include body of the same chunk-store module.
- The optimization-cap comparison moves to `wasm::performance::optimization`.

**Rationale:** Extracting helpers adds signature lines. Without these moves, three files would cross the 300-line
`excessive_file_length` limit and grow that family.

### Decision: Name new helpers without repeating ancestor segments

**Choice:** New private items do not reuse words from their module path. For example, `cluster_harness::runner` uses
`PlannedRun`, `RunEvidence`, and `plan_run`, not `HarnessPlan`.

**Rationale:** New names must not add `path_segment_repetition` findings.

### Decision: Leave five unmasked public names to C2

**Choice:** Five public names are not renamed and are not re-exempted:
- `content_store_port_descriptors`
- `fabric_membership_port_descriptors`
- `fabric_observability_port_descriptors`
- `fabric_time_port_descriptors`
- `run_executable_system_extension_fixture`

`path_segment_repetition` now reports them because their bodies no longer contain `ExternalEffect` or
`compatible_state_schemas`.

**Rationale:** Octet exempts any item whose source snippet contains `compat`, `external`, `mandated`, or `api`. The
check is `has_compatibility_documentation` in `src/naming/path_segment_repetition.rs:128` at the pinned
`fc38f593`. These findings existed before this change and a keyword match hid them. Renames belong to the C2 naming
decision. Adding keywords to re-exempt them would be suppression.
