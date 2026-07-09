## Why

Molten has a large accepted testing-harness specification and a substantial Rust/CLI test suite, but requirement coverage is still easiest to reason about from ad hoc command arguments and reviewer memory. That makes it hard to tell which changed requirements have positive evidence, negative evidence, property coverage, CLI coverage, or explicit exemptions.

A checked-in evidence matrix makes coverage part of the source tree. Reviewers can inspect one canonical artifact before implementation, CI can fail closed on missing coverage, and future test hardening work can improve the matrix incrementally without changing every test at once.

## What Changes

- Add a checked-in requirement-to-test evidence matrix for testing-harness requirements.
- Support positive, negative, property, CLI, integration, and exemption entries with canonical artifact refs or deterministic commands.
- Extend traceability scanning or add an adjacent gate so changed evidence-bearing requirements require both positive and negative coverage unless explicitly exempted.
- Emit a canonical traceability receipt that summarizes covered, missing, stale, unsupported, and exempt entries.

## Impact

This turns coverage review into a deterministic gate instead of a manual checklist. It should be the first test-suite hardening slice because it makes every later gap visible and auditable.
