# Design: Octet burn-down, safety `no_unwrap`

## Context

`no_unwrap` flags `.unwrap()` and `.expect()` on `Option`/`Result` outside test context. Its test-context check reads
`#[test]` and `#[cfg(test)]` attributes, but inside an integration-test binary the `#[test]` attribute is already expanded,
so every `.expect()` in `tests/*.rs` is reported. The repository's existing repair is a `Result`-returning test with a
boxed error (`tests/moltennodehostfacade.rs`, `CliResult` in `tests/parts/cliharness/p000`, `BoundaryResult` in
`tests/fabric_simulation_boundary.rs`).

## Decisions

### Decision: Result-returning integration tests

**Choice:** Each touched test binary declares one local alias `type TestResult<T> = std::result::Result<T,
Box<dyn std::error::Error>>` (cliharness keeps `CliResult`). Each test and helper returns it and uses `?`. Several fixture
error types (`NativeServiceError`, `NativeHostIssue` vectors, `content_replication::Issue`) do not implement
`std::error::Error`, so a small local `OrFail` trait converts a failed `Result` or an absent `Option` into a boxed error
that keeps the former `expect` label and the failure's `Debug` form.

**Rationale:** It is the pattern the repository already uses, and a failure still fails the test with the same message.

### Decision: Propagate errors in library helpers

**Choice:** `fixture_profile` and the lifecycle summary builder return `Result` and their callers use `?`. The summary
builder reports a node without a config ref as the existing invalid-harness error.

**Rationale:** Callers already return `Result`, so no signature crosses a public API boundary.

### Decision: Shift instead of checked division in `integer_sqrt`

**Choice:** `(high - low) >> 1` replaces `checked_div(BINARY_SEARCH_DIVISOR).expect(...)`.

**Rationale:** Halving an unsigned value by shift is total and exactly equal to division by two, so the search and its
result do not change.

## No-spec classification

Accepted requirement text does not change. Semantic review inputs: the diff of the flagged files and the Octet, clippy,
and test evidence.

## Failure behavior

A test that used to panic now returns `Err`, which the harness reports as a failure. A library helper that used to panic
now returns an error on the same condition.

## Risks / Trade-offs

- A `?` that converts an error type loses nothing, because the boxed error keeps the source's `Display`.
