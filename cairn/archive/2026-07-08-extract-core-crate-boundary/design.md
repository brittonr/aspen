## Context

The intended layer map starts with a pure core that owns envelope identity, content refs, capability-neutral data types, and deterministic validation. The repository currently declares only the root package as a workspace member, so compile-time dependency direction is not enforced.

## Design

### Core crate scope

The new core crate should contain only deterministic logic and stable data definitions that can be tested without adapters. Candidate surfaces include:

- shared error/result types or a minimal core error family;
- bounded collection helpers;
- content-ref and stable-id newtypes;
- envelope DTOs and pure validation results;
- canonical identity inputs that do not require live codec execution;
- small pure helpers used across policy, runtime, evidence, and adapters.

### Dependency rules

The core crate must not depend on CLI parsing, Iroh, Redb, Wasmtime, Steel execution, Nickel runtime evaluation, filesystem traversal, environment reads, process execution, clocks, or tracing side effects. If canonical Preserves encoding is too heavy for the first slice, the core crate can own typed inputs while a codec crate owns conversion.

### Migration pattern

The root crate should continue to re-export moved items under the existing public paths while internal callers move toward the core crate. That keeps user-facing behavior stable and allows incremental proof and test refresh.

### Validation

The first slice should include positive tests showing moved types/validators still accept current valid fixtures and negative tests showing malformed refs, missing fields, or invalid bounds still fail before adapters run.

## Non-goals

- Do not extract all runtime domains in one step.
- Do not remove compatibility re-exports in the first core extraction.
- Do not introduce runtime Nickel evaluation or new trust claims.
