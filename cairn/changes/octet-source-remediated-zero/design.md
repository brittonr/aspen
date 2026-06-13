## Context

`docs/octet-tigerstyle-remediation.md` records that current Octet evidence is clean but configuration-clean: `dylint.toml` disables broad/high-noise lint families such as file length, function length, repeated path segments, and module count. The archived `octet-tigerstyle-remediation` change deliberately did not claim source-remediated zero for those families.

`src/main.rs` is the largest imperative shell and is the best first target for incremental, behavior-preserving splits. The Octet command group is self-contained enough to move first because it depends mostly on `molten::octet_gate`, `molten::octet_remediation`, Preserves file IO, and receipt emission helpers.

## Design

### CLI module boundary

Create a binary-local `src/cli_octet.rs` module that owns:

- `OctetCommand` and nested Octet subcommands;
- `run_octet_command` dispatch;
- local Preserves file read/write helpers needed by Octet artifact, source-gate, baseline, review, gate, and remediation commands.

`src/main.rs` remains the top-level Clap shell, but its `TestCommand::Octet` variant references `cli_octet::OctetCommand`, and dispatch delegates to `cli_octet::run_octet_command`.

### Semantic preservation

The split must preserve:

- command paths and flags for `molten test octet ...`;
- receipt labels and stdout/stderr behavior;
- fail-closed denial errors for strict gate, source-gate validation, and baseline checks;
- canonical output values produced by the underlying core helpers.

The module may duplicate tiny shell helpers (`read_preserves_file`, `write_file`, `emit_named_receipt`) instead of depending on private `main.rs` helpers. This keeps the new module a CLI shell boundary and avoids moving unrelated repro/report helpers.

### Evidence and validation

Each slice should run focused Rust validation first, then refresh Octet evidence when the source scope changes materially. Until disabled lint families are removed or narrowed, docs must continue describing the state as configuration-clean rather than source-remediated zero.

## Non-goals

- Do not change Octet gate policy semantics.
- Do not change canonical receipt schemas or refs intentionally.
- Do not remove disabled lint families until enough source splits have landed and Octet can validate the affected families without excessive false positives.
- Do not rewrite unrelated CLI command groups in this first slice.
