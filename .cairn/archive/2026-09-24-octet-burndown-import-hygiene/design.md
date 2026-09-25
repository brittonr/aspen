# Design: Octet burn-down, import hygiene

## Context

`non_trait_imports` rejects a private `use` of a concrete item, because the import hides the owner path.
`explicit_defaults` rejects `T::default()` for an ADT owned by another crate, and it also fires on `#[serde(default)]`.
The repository's supported repairs are owner-path qualification (`docs/octet-tigerstyle-remediation.md` import-hygiene
slices) and explicit owner constructors with `Default` delegating to them (the `exercise-simulation-faults-causally`
precedent).

## Decisions

### Decision: Hand-apply the codemod's repair where the codemod refuses

**Choice:** Run `scripts/octet-qualify-imports.rs --self-test` and `--dry-run` first. The dry run skipped all 8 files,
as recorded in `evidence/codemod-dry-run.txt`, so apply the same rewrite by hand and let compilation and tests
decide acceptance.

**Rationale:** The refusals come from scope constructs the tool cannot rewrite safely, not from repairs that are
unsound. The compiler catches missed or wrong qualifications.

### Decision: Module path instead of re-export for CLI commands

**Choice:** `pub(crate) mod command` plus `crate::cli_fabric_*::command::*Command` at the one consumer.

**Rationale:** The re-export existed only to shorten a single path, and the owner module path is explicit.

### Decision: Explicit constructors, `Default` delegates

**Choice:** Each flagged ADT gains a constructor that lists its fields literally, and `Default` calls it.

**Rationale:** Values stay identical to the derived defaults, and in-crate `Default` users keep working.

### Decision: Named serde defaults

**Choice:** `#[serde(default = "…")]` with `const fn` helpers that return `None` or `Vec::new()`.

**Rationale:** Deserialization of a missing field stays the same. The lint accepts named defaults; the file already
uses one for `algorithm_profile`.

## No-spec classification

Accepted requirement text does not change. Semantic review inputs: the diff of the flagged files, the new
constructors, and the Octet, clippy, and test evidence.

## Failure behavior

Any missed qualification fails compilation. Any value drift in a constructor fails the existing tests that compare
canonical receipts and state.

## Risks / Trade-offs

- Import-qualification rewrites have broken a flake literal scan before (`18a59c2b0`), so the source-scan checks are
  rerun here.
