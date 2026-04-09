## Context

Aspen already spans several test layers: per-crate unit tests, property tests, madsim scenarios, patchbay topology tests, real-network integration tests, and 79 NixOS VM suites wired into `flake.nix`. The problem is not missing harness surface area. The problem is that suite selection and harness wiring are split across hand-maintained flake blocks, nextest profile regexes, shell wrappers, comments in AGENTS, and duplicated helper code such as `tests/support/real_cluster.rs` alongside `crates/aspen-testing*`.

That duplication makes focused validation expensive and brittle. Adding or reclassifying a suite means touching several files, patchbay remains a special case, and many suites still express readiness with fixed sleeps instead of explicit state-based waits. The result is slower feedback loops and weak visibility into retry-only passes, slowest suites, and missing higher-layer coverage.

## Goals / Non-Goals

**Goals:**

- Make Nickel metadata the source of truth for suite identity, layer, ownership, runtime class, prerequisites, tags, and execution targets.
- Generate Rust- and Nix-consumable inventory outputs from that metadata so test grouping and registration stop drifting.
- Consolidate shared harness APIs under `crates/aspen-testing*` and retire the parallel real-cluster bootstrap path in `tests/support/`.
- Replace fixed sleeps with reusable wait helpers in the most failure-prone Rust and VM flows.
- Emit machine-readable reports that expose retries, skips, runtime hotspots, and coverage-by-layer gaps.

**Non-Goals:**

- Rewriting every existing test in one change.
- Replacing nextest, NixOS VM tests, patchbay, or madsim with a new runner.
- Adding line-coverage tooling or changing production behavior.
- Forcing every suite into one execution binary; layer-specific runners remain valid.

## Decisions

### 1. Authoritative suite metadata lives in Nickel

**Choice:** Store suite manifests in Nickel, validated against a shared schema, and treat those manifests as the only hand-authored test inventory.

**Rationale:** The repo already uses Nickel for Aspen CI configuration, so Nickel fits the existing toolchain and keeps manifests expressive without introducing a new format. The user also asked for metadata to be written in Nickel.

**Alternative:** TOML or YAML manifests would be simpler to parse but would add another configuration language and would not reuse existing Nickel conventions already present in the repo.

**Implementation:** Add a schema for suite manifests and a directory of per-suite Nickel files. Each manifest declares stable suite metadata and references the execution target for its layer.

### 2. Generated outputs bridge Nickel into Rust and Nix entry points

**Choice:** Generate machine-consumable outputs from the Nickel manifests for consumers that should not parse Nickel directly at runtime or flake-eval time.

**Rationale:** `flake.nix`, nextest helpers, and reporting tooling need fast, deterministic inputs. Direct Nickel evaluation inside every consumer would spread parser and export logic around the repo and complicate flake evaluation.

**Alternative:** Teach each consumer to parse Nickel directly. Rejected because it would multiply failure modes and duplicate validation logic.

**Implementation:** One generator exports a normalized inventory that Rust tooling, scripts, and Nix check registration can consume. The generated inventory becomes the translation layer between authored Nickel metadata and harness execution.

### 3. `aspen-testing` becomes the shared harness facade

**Choice:** Move common real-cluster bootstrap and wait functionality behind the `crates/aspen-testing*` facade and reduce `tests/support/real_cluster.rs` to either a thin compatibility wrapper or remove it entirely.

**Rationale:** Today the repo has both a reusable testing crate family and a second, root-only real-cluster bootstrap path. That split guarantees drift. A shared facade makes fixes to timeouts, bootstrap sequencing, and helper ergonomics flow to all callers.

**Alternative:** Leave root integration tests on a separate support module. Rejected because it preserves exactly the drift we want to remove.

**Implementation:** Define common harness traits or façade types for cluster setup, client access, and layer-specific capabilities. Real-network tests migrate first, while patchbay and madsim adapters retain their specialized internals behind the shared API.

### 4. Wait helpers replace fixed sleeps in prioritized paths

**Choice:** Add reusable wait helpers for Rust and NixOS VM suites, then migrate the high-value suites that are currently using fixed sleeps to await leader election, replication, service readiness, and job completion.

**Rationale:** The current harness has many unconditional sleeps that are really hidden readiness checks. That increases runtime on fast machines and creates flakes on slow ones.

**Alternative:** Keep fixed sleeps and rely on longer timeouts or retries. Rejected because it treats symptoms instead of making readiness explicit.

**Implementation:** Add bounded wait helpers with named conditions, common poll intervals, and clear failure messages. Start with cluster, dogfood, federation, and VM readiness flows where sleeps are most prevalent.

### 5. Reporting is derived from the same inventory and run outputs

**Choice:** Build machine-readable harness reports by joining run results with suite metadata.

**Rationale:** Retry-only passes, skipped suites, and missing higher-layer coverage are only actionable if they can be tied back to owners, layers, and subsystem tags.

**Alternative:** Report only raw nextest or Nix output. Rejected because raw logs do not answer which subsystem is under-covered or which layer is routinely unstable.

**Implementation:** Persist a run report format that records outcomes, retry counts, skips, durations, and coverage-by-layer summaries keyed by suite identifier.

## Risks / Trade-offs

- **Generator drift** -> Keep Nickel manifests authoritative and add validation that regenerated outputs are current.
- **Over-modeling suites** -> Start with a small required field set and add optional fields only when a consumer uses them.
- **Migration churn** -> Migrate a narrow first slice of suites and provide compatibility shims while root tests move onto shared harness APIs.
- **Flake evaluation complexity** -> Consume generated outputs from Nix rather than parsing Nickel inside `flake.nix`.
- **Partial sleep removal** -> Prioritize high-value suites first and leave purely timing-based simulation sleeps alone when they are part of the scenario instead of hidden readiness checks.

## Migration Plan

1. Add the Nickel schema, manifest directory, and generator.
2. Register an initial set of suites spanning Rust integration, patchbay, and VM layers.
3. Switch flake registration, helper scripts, and report generation to the generated inventory.
4. Move real-cluster integration tests onto the shared `aspen-testing` façade.
5. Introduce wait helpers and migrate the most sleep-heavy readiness flows.
6. Turn on report generation in local and CI-oriented validation commands.

Rollback is straightforward: generated outputs can continue to be consumed while manifest adoption is incomplete, and existing suite entry points remain callable until the last migration step is finished.

## Open Questions

- Should generated inventory artifacts be committed to the repo or produced during validation commands and checked for freshness?
- Which command should own metadata export and freshness checks: a script under `scripts/`, a small Rust tool, or a Nix app?
- How much change-aware selection should be in the first pass versus a follow-up change once metadata and reports exist?
