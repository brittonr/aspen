## Why

Molten's Octet gate currently passes in a configuration-clean state, but the documented caveat remains: several broad lint families are disabled in `dylint.toml` while source-remediated-zero work is still outstanding. The largest visible hotspot is the monolithic CLI shell in `src/main.rs`, where command parsing and dispatch for many domains live in one file.

Strict release evidence should eventually distinguish a true source-shaped zero from a configuration-clean pass. This change starts the source-remediated-zero burn-down by extracting self-contained CLI command groups into focused shell modules, preserving canonical receipt behavior while reducing the main CLI surface.

## What Changes

- Move Octet command enums and dispatch from `src/main.rs` into a focused `src/cli_octet.rs` module.
- Move Retention command enums and dispatch from `src/main.rs` into a focused `src/cli_retention.rs` module as part of the disabled-lint-family burn-down.
- Move Delivery command enums and dispatch from `src/main.rs` into a focused `src/cli_delivery.rs` module as a CLI shell burn-down step.
- Move Provenance command enums and dispatch from `src/main.rs` into a focused `src/cli_provenance.rs` module as a CLI shell burn-down step.
- Move Protocol command enums and dispatch from `src/main.rs` into a focused `src/cli_protocol.rs` module as a CLI shell burn-down step.
- Move Job command enums and dispatch from `src/main.rs` into a focused `src/cli_job.rs` module as the next CLI shell burn-down step.
- Move Secrets and Plugin command enums and dispatch from `src/main.rs` into focused `src/cli_secrets.rs` and `src/cli_plugin.rs` modules as a small low-risk follow-up split.
- Move Node command enums and dispatch from `src/main.rs` into a focused `src/cli_node.rs` module as the next top-level CLI shell hotspot split.
- Move Repro command enums and dispatch from `src/main.rs` into a focused `src/cli_repro.rs` module as the next test CLI shell hotspot split.
- Move Catalog command enums and dispatch from `src/main.rs` into a focused `src/cli_catalog.rs` module as the next catalog/MCP CLI shell hotspot split.
- Move Chunk command enums and dispatch from `src/main.rs` into a focused `src/cli_chunk.rs` module as the next chunk-store CLI shell hotspot split.
- Move Cache command enums and dispatch from `src/main.rs` into a focused `src/cli_cache.rs` module as the next eval-cache CLI shell hotspot split.
- Move Artifact command enums and dispatch from `src/main.rs` into a focused `src/cli_artifact.rs` module as the next artifact-registry CLI shell hotspot split.
- Move Storage command enums and dispatch from `src/main.rs` into a focused `src/cli_storage.rs` module as the next typed-storage CLI shell hotspot split.
- Move Schema command enums and dispatch from `src/main.rs` into a focused `src/cli_schema.rs` module as the next schema-identity CLI shell hotspot split.
- Preserve existing `molten test octet ...`, `molten test retention ...`, `molten test delivery ...`, `molten test provenance ...`, `molten test protocol ...`, `molten test job ...`, `molten test secrets ...`, `molten test plugin ...`, `molten test repro ...`, `molten test catalog ...`, `molten test chunk ...`, `molten test cache ...`, `molten test artifact ...`, `molten test storage ...`, `molten test schema ...`, and `molten node ...` command syntax, receipt output, denial behavior, and canonical Preserves values.
- Track the remaining disabled lint family burn-down as explicit future work rather than claiming the full source-remediated-zero state is complete.
- Require focused validation and refreshed Octet evidence before claiming source-gate improvements.

## Impact

This is a low-risk first vertical slice in the source-remediated-zero roadmap. It reduces the monolithic imperative CLI shell without changing runtime semantics or release evidence contracts. Future slices should continue splitting CLI groups and high-value modules, then remove or narrow the disabled lint family caveats when the source shape supports it.
