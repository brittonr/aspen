## Context

The Aspen codebase relies heavily on the "Functional Core, Imperative Shell" (FCIS) pattern, distributed consensus via Raft, and local storage via `redb`. To maintain high reliability and the liveness guarantees expected of a distributed orchestration layer, the system cannot tolerate unexpected panics caused by unhandled error states.

A recent multi-lens audit found over 5,000 instances of `.unwrap()`, `.expect()`, and `panic!()` in production source code (`src/` directories), violating the project's Tiger Style guidelines.

## Goals / Non-Goals

**Goals:**

- Eliminate all `.unwrap()`, `.expect()`, `todo!()`, and `panic!()` usages in production code (`crates/**/src/*.rs`).
- Gracefully propagate errors using `Result` and the `snafu` crate.
- Provide actionable, descriptive error contexts using `snafu::context()`.
- Ensure nodes handle partial failures (e.g., I/O issues, network timeouts) without crashing.

**Non-Goals:**

- Removing `.unwrap()`, `.expect()`, or `panic!()` from test code (`tests/` directories or `#[test]` modules).
- Refactoring the underlying logic of complex systems; the focus is strictly on error propagation and safe handling.
- Completely preventing panics from third-party dependencies (though we should handle their returned `Result`s safely).

## Decisions

### 1. Phased Rollout by Crate

**Choice:** The remediation will be executed crate by crate, prioritized by criticality (Core & Transport first, followed by Coordination & Raft, and finally Services & Jobs).
**Rationale:** A codebase-wide purge in a single pass is too disruptive and likely to introduce logical regressions or cause widespread merge conflicts. Phased rollouts allow for incremental testing and verification.
**Alternative:** A massive global search-and-replace using an AST tool. Rejected because error handling often requires semantic understanding of the specific failure mode to define proper `snafu` error variants.

### 2. Error Definition Strategy

**Choice:** We will use `snafu` in library crates (`aspen-core`, `aspen-raft`, etc.) to define domain-specific error enums with `#[snafu(context)]`. We will use `anyhow` in application/binary crates (`aspen-node`, `aspen-cli`) for rapid error reporting.
**Rationale:** This aligns with existing Aspen conventions. Libraries need structured, matchable errors for programmatic recovery. Applications need descriptive, chained errors for human debugging.

**Implementation:**
When encountering an `.unwrap()`, we will:

1. Identify the failure condition.
2. If an appropriate error variant doesn't exist in the crate's error enum, create one (e.g., `IoError { source: std::io::Error }`).
3. Replace `.unwrap()` with `.context(IoSnafu)?` or `.map_err(|e| ...)?`.
4. Update the enclosing function signature to return `Result<T, CrateError>`.
5. Propagate the change up the call stack.

## Risks / Trade-offs

**Call Stack Churn** → Changing a deeply nested function to return a `Result` forces all its callers to handle the `Result`. This can cause cascading API changes.
*Mitigation:* We will start from the bottom (leaf functions) and work our way up to handlers, managing the churn incrementally within each crate.

**Performance Overhead** → Boxing or allocating complex error contexts in tight loops could theoretically impact performance.
*Mitigation:* Use `snafu` which is highly efficient. Avoid allocating strings in the error context on the hot path (e.g., inside the redb transaction loop) unless a failure actually occurs.
