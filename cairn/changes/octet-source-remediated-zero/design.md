## Context

`docs/octet-tigerstyle-remediation.md` records that current Octet evidence is clean but configuration-clean: `dylint.toml` disables broad/high-noise lint families such as file length, function length, repeated path segments, and module count. The archived `octet-tigerstyle-remediation` change deliberately did not claim source-remediated zero for those families.

`src/main.rs` is the largest imperative shell and is the best first target for incremental, behavior-preserving splits. The Octet command group was self-contained enough to move first because it depends mostly on `molten::octet_gate`, `molten::octet_remediation`, Preserves file IO, and receipt emission helpers. The Retention command group followed because it is operator-facing, self-contained around `molten::retention`, and large enough to materially reduce the monolithic CLI file while preserving receipt behavior. The Delivery command group is a smaller split around `molten::delivery_idempotency`; the Provenance command group follows the same low-risk pattern around `molten::provenance`. The Protocol command group is self-contained around `molten::protocol_session` and has existing CLI coverage. The Job command group is a hotspot split because its dispatch is large but remains a CLI shell over `molten::job_dag`, coordination scheduling evidence, local-gossip worker fixtures, and receipt IO. The Secrets and Plugin command groups are small low-risk follow-up splits over `molten::secrets` and `molten::plugin_host` fixture/install/show surfaces. The Node command group is the next top-level hotspot split over `molten::node_daemon`, `molten::node_runtime`, and provenance fixture helpers, including live workflow bundle diagnostics and node-control receipt materialization. The Repro command group is another test CLI hotspot split over harness repro export/verify/unpack and local Iroh repro exchange receipt materialization. The Catalog command group is a catalog/MCP hotspot split over `molten::catalog` and `molten::catalog_mcp` query, view, graph, short-id, chunk-store, and read-only MCP receipt surfaces. The Chunk command group is another hotspot split over `molten::chunk_store` put/verify/read/range/sync/local-Iroh/pin/index/receipt/lineage/GC receipt materialization. The Cache command group is a focused split over `molten::eval_cache` put/get/status/list/show/invalidate/index receipt materialization. The Artifact command group is the next focused split over `molten::artifacts` install/list/view/name/dependency/closure/impact/index receipt materialization.

## Design

### CLI module boundary

Create binary-local CLI modules that own self-contained command groups:

- `src/cli_octet.rs` owns `OctetCommand`, nested Octet subcommands, `run_octet_command`, and local Preserves file read/write helpers needed by Octet artifact, source-gate, baseline, review, gate, and remediation commands.
- `src/cli_retention.rs` owns `RetentionCommand`, retention dispatch, and local Preserves file read/write helpers needed by retention class, pin, clearance, bundle, plan, audit, check, fixture, and show commands.
- `src/cli_delivery.rs` owns `DeliveryCommand`, delivery dispatch, and local Preserves file read/write helpers needed by delivery scope, operation-id, idempotency check, receipt-show, and show commands.
- `src/cli_provenance.rs` owns `ProvenanceCommand`, provenance dispatch, bounded CLI evidence parsing, and local Preserves file read/write helpers needed by build-record, verify-build, record, fixture, evaluate, and show commands.
- `src/cli_protocol.rs` owns `ProtocolCommand`, protocol dispatch, lifecycle/index helpers, and local Preserves file read/write helpers needed by install, run-request-response, gate-lifecycle, and show commands.
- `src/cli_job.rs` owns `JobCommand`, job dispatch, worker scheduling helpers, local-gossip worker fixture helpers, and local Preserves file read/write helpers needed by install, run, plan/profile/fusion, sync, admission, execution, worker, ref-submit/ref-execute, status, and receipt-show commands.
- `src/cli_secrets.rs` owns `SecretsCommand`, secrets fixture/show dispatch, and local Preserves file read/write helpers for redaction, reveal, decrypt, cleanup, private-bundle, and evidence outputs.
- `src/cli_plugin.rs` owns `PluginCommand`, plugin install/fixture/show dispatch, and local Preserves file read/write helpers for plugin manifest, lifecycle, health, removal, and evidence outputs.
- `src/cli_node.rs` owns `NodeCommand`, node-control dispatch, live workflow next-step diagnostics, adapter receipt parsing, and local Preserves file read/write helpers for node init/run/status/stop, control ingress, live tickets, workflow bundles, protocol gates, shutdown, and health receipts.
- `src/cli_repro.rs` owns `ReproCommand`, repro export/verify/unpack/publish/fetch dispatch, reveal receipt validation, and local Preserves file read/write helpers for sealed repro bundles, failure repro bundles, repro verify receipts, and local Iroh repro exchange receipts.
- `src/cli_catalog.rs` owns `CatalogCommand`, catalog dispatch, catalog filter construction, visibility construction, and local Preserves file read/write helpers for list, view, search, dependency graph, short-id, chunk-store, show, and read-only MCP call receipts.
- `src/cli_chunk.rs` owns `ChunkCommand`, chunk-store dispatch, and local Preserves file write/receipt helpers for put, verify, read, range, sync, local Iroh publish/fetch, pin, index, receipt, lineage, and GC commands.
- `src/cli_cache.rs` owns `CacheCommand`, eval-cache dispatch, synthetic cache-ref construction, and local Preserves file read/write/receipt helpers for put, get, status, list, show, invalidate, and index rebuild commands.
- `src/cli_artifact.rs` owns `ArtifactCommand`, artifact-registry dispatch, synthetic artifact-ref construction, and local Preserves file read/write/receipt helpers for install, list, view, name-set, name-show, deps, closure, impact, and index rebuild commands.

`src/main.rs` remains the top-level Clap shell, but its `TestCommand::Octet`, `TestCommand::Delivery`, `TestCommand::Protocol`, `TestCommand::Provenance`, `TestCommand::Retention`, `TestCommand::Job`, `TestCommand::Secrets`, `TestCommand::Plugin`, `TestCommand::Repro`, `TestCommand::Catalog`, `TestCommand::Chunk`, `TestCommand::Cache`, and `TestCommand::Artifact` variants reference module-local command enums, and dispatch delegates to module-local runners.

### Semantic preservation

The split must preserve:

- command paths and flags for `molten test octet ...`, `molten test delivery ...`, `molten test provenance ...`, `molten test protocol ...`, `molten test retention ...`, `molten test job ...`, `molten test secrets ...`, `molten test plugin ...`, `molten test repro ...`, `molten test catalog ...`, `molten test chunk ...`, `molten test cache ...`, `molten test artifact ...`, and `molten node ...`;
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
