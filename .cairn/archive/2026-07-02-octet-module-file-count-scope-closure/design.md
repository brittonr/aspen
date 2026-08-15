## Context

`dylint.toml` currently disables only `module_file_count`. README evidence says the final no-disabled probe had source-scope warnings that were generated/remapped dependency source rather than Molten-owned source. The accepted source-scope requirement already demands deterministic classification before narrowing gate scope.

## Design

### Source-scope closure core

The closure decision is deterministic data over Octet artifacts and the checked-in source inventory:

- current no-disabled finding counts by lint;
- source-scope rows reported as `module_file_count`;
- classification for each row: Molten-owned, integration-test, generated/remapped dependency, registry/rustlib, or unknown;
- pass condition: zero Molten-owned and zero unknown `module_file_count` rows before removing the broad disable.

Filesystem reads, `cargo octet` execution, artifact import, and documentation updates stay in the imperative shell.

### Configuration boundary

Removing `module_file_count` from `dylint.toml` is allowed only when validation shows the configured strict gate does not hide Molten-owned source debt. If residual rows remain due to external source maps, they must stay documented as external-scope residue until Octet upstream/source-map support can report them outside Molten source.

### Validation

Validation should include:

- a fresh no-disabled probe;
- focused Rust tests for Octet remediation/source-scope classification when code changes touch that path;
- `cargo fmt --check` and focused clippy/test checks when source changes occur;
- `cargo octet check` with the final checked-in `dylint.toml` proving the configured workspace gate is not warning-only from Molten-owned source.

### Closure evidence

The closure evidence is complete when the checked-in `dylint.toml` has `disabled_lints = []`, configured workspace and lib Octet summaries are clean, the strict gate receipt binds structured findings plus object-corpus/fingerprint evidence, and the object-corpus coverage set includes shell modules selected by the replay command even when those modules emit no standalone function objects.

## Non-goals

- Do not weaken strict Octet denial for warning-only status.
- Do not add suppressions that silently hide Molten-owned source rows.
- Do not claim upstream Octet source-map issues are fixed unless the artifacts prove it.
