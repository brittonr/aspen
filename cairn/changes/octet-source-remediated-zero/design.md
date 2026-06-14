## Context

`docs/octet-tigerstyle-remediation.md` records that current Octet evidence is clean but configuration-clean: `dylint.toml` disables broad/high-noise lint families such as file length, function length, repeated path segments, and module count. The archived `octet-tigerstyle-remediation` change deliberately did not claim source-remediated zero for those families.

`src/main.rs` is the largest imperative shell and is the best first target for incremental, behavior-preserving splits. The Octet command group was self-contained enough to move first because it depends mostly on `molten::octet_gate`, `molten::octet_remediation`, Preserves file IO, and receipt emission helpers. The Retention command group followed because it is operator-facing, self-contained around `molten::retention`, and large enough to materially reduce the monolithic CLI file while preserving receipt behavior. The Delivery command group is a smaller split around `molten::delivery_idempotency`; the Provenance command group follows the same low-risk pattern around `molten::provenance`.

## Design

### CLI module boundary

Create binary-local CLI modules that own self-contained command groups:

- `src/cli_octet.rs` owns `OctetCommand`, nested Octet subcommands, `run_octet_command`, and local Preserves file read/write helpers needed by Octet artifact, source-gate, baseline, review, gate, and remediation commands.
- `src/cli_retention.rs` owns `RetentionCommand`, retention dispatch, and local Preserves file read/write helpers needed by retention class, pin, clearance, bundle, plan, audit, check, fixture, and show commands.
- `src/cli_delivery.rs` owns `DeliveryCommand`, delivery dispatch, and local Preserves file read/write helpers needed by delivery scope, operation-id, idempotency check, receipt-show, and show commands.
- `src/cli_provenance.rs` owns `ProvenanceCommand`, provenance dispatch, bounded CLI evidence parsing, and local Preserves file read/write helpers needed by build-record, verify-build, record, fixture, evaluate, and show commands.

`src/main.rs` remains the top-level Clap shell, but its `TestCommand::Octet`, `TestCommand::Delivery`, `TestCommand::Provenance`, and `TestCommand::Retention` variants reference module-local command enums, and dispatch delegates to module-local runners.

### Semantic preservation

The split must preserve:

- command paths and flags for `molten test octet ...`, `molten test delivery ...`, `molten test provenance ...`, and `molten test retention ...`;
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
