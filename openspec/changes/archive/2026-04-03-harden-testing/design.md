## Context

Aspen has ~5,700 tests across 650 files, 129 madsim simulations, 78 NixOS VM tests, 25 proptest blocks, 15 fuzz targets, and Verus formal verification across 3 crates. This is a strong foundation, but structural gaps exist:

- **Zero-test crates**: `aspen-secrets-handler` (security-critical, 1,082 lines), `aspen-snix-bridge` (510 lines)
- **Under-tested handlers**: `aspen-forge-handler` (402 lines/test), `aspen-ci-handler` (346 lines/test)
- **No mutation testing**: No way to know if existing tests would catch a real regression
- **Thin proptest**: 25 proptest blocks for 436k lines; model-based tests exist only in `aspen-coordination`
- **Missing madsim scenarios**: No split-brain heal, no snapshot-during-membership-change, no clock-skewed TTL tests
- **No serialization stability tests**: Wire format changes could go undetected

## Goals / Non-Goals

**Goals:**

- Establish mutation testing as a measure of test effectiveness, starting with verified crates
- Eliminate zero-test crates for any crate with >500 lines of production code
- Double proptest model-based coverage in coordination and Raft storage
- Add madsim scenarios for the failure modes most likely to cause production incidents
- Detect accidental serialization/wire format breakage via snapshot tests
- Add fault injection at every I/O boundary in the storage and network layers

**Non-Goals:**

- Achieving 100% line coverage (mutation scores are a better signal)
- Rewriting existing tests or changing test frameworks
- Adding HTTP-based test infrastructure (all testing stays within Iroh/madsim)
- End-to-end UI testing for `aspen-tui` (low ROI relative to complexity)
- Formalizing new Verus specs (this change is about runtime testing, not formal verification)

## Decisions

### D1: cargo-mutants over cargo-fuzz expansion

**Decision**: Use `cargo-mutants` as the primary new testing signal rather than expanding fuzz targets.

**Rationale**: Fuzzing finds crashes in parsing/deserialization paths (already covered by 15 targets). Mutation testing answers the more pressing question: "do our tests catch bugs in business logic?" The verified crates (`aspen-coordination`, `aspen-core`, `aspen-raft`) have the most critical logic and existing tests — mutation testing validates those tests actually work.

**Alternative considered**: Expanding bolero/fuzz. Rejected because the existing fuzz targets cover the parsing surface well; the gap is in behavioral correctness testing.

### D2: Insta for serialization snapshots over custom golden files

**Decision**: Use the `insta` crate for snapshot testing rather than hand-maintained golden files.

**Rationale**: `insta` provides `cargo insta review` for reviewing snapshot changes, inline and file-based snapshots, and redaction support for non-deterministic fields. It's the standard Rust approach and avoids maintaining a custom diffing tool. Already referenced in several crate source files.

### D3: Fault injection via buggify macro, not fail-points crate

**Decision**: Extend the existing `buggify!` macro pattern for new fault injection points rather than adopting `fail` or `failpoints`.

**Rationale**: The codebase already uses buggify-style fault injection in chaos tests. Adding a second fault injection mechanism would split the test surface. The buggify pattern integrates with madsim's deterministic scheduler, which `fail-points` does not.

### D4: Prioritize handler crate tests as integration tests, not unit tests

**Decision**: Test handler crates (`aspen-forge-handler`, `aspen-ci-handler`, etc.) through integration tests using `TestCluster` or patchbay harness, not isolated unit tests.

**Rationale**: Handler code is thin glue between RPC dispatch and Raft/KV operations. Unit tests with mocked stores would test the mock, not the handler. The patchbay harness (`aspen-testing-patchbay`) already provides the infrastructure for this — it's just under-used for handler crates.

### D5: Scope mutation testing to verified crates initially

**Decision**: Run `cargo-mutants` only on `aspen-coordination`, `aspen-core`, and the `verified/` modules of `aspen-raft` in the first iteration.

**Rationale**: These crates contain the highest-value logic (lock semantics, fencing tokens, TTL computation, HLC) and have existing tests to validate. Running mutants across all 80 crates would take hours and produce too much noise. Start focused, establish baselines, then expand.

## Risks / Trade-offs

**[Mutation testing CI time]** → Run mutation testing on a scheduled nightly job, not on every PR. Gate PRs only on quick nextest profile; use mutation scores as a weekly health metric.

**[Insta snapshot churn]** → Snapshots break on any format change, including intentional ones. Mitigate by keeping snapshots at the serialization boundary (RPC messages, not internal structs) and documenting the `cargo insta review` workflow.

**[Buggify overhead in hot paths]** → Buggify macros compile to no-ops in release builds. In test builds, the check is a single atomic load. Acceptable overhead for the fault coverage gained.

**[New madsim scenarios may be flaky]** → Use fixed seeds and deterministic scheduling. All new madsim tests MUST use `#[madsim::test]` with explicit seed configuration. Add to the `ci` nextest profile with extended timeouts.
