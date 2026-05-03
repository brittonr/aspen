## Context

Dogfood receipt generation is intentionally simple: a run writes canonical JSON to a sibling receipts directory derived from `--cluster-dir` (`<cluster-dir>-receipts/<run-id>.json`) and logs the path. This is enough for durable evidence, but not enough for an operator workflow.

## Goals / Non-Goals

**Goals:**

- Provide read-only local receipt discovery and display through the dogfood CLI.
- Keep default output concise enough for terminals and logs.
- Preserve machine-readable output through `--json` without changing the v1 receipt schema.
- Treat corrupt or wrong-schema receipt files as explicit operator-visible errors for `show`, while allowing `list` to skip invalid files with a warning.

**Non-Goals:**

- Upload receipts to Forge, CI, or object storage.
- Query live cluster state from receipt inspection commands.
- Add a new receipt schema version.

## Decisions

### 1. Nested receipts command

**Choice:** Add a `receipts` subcommand with `list` and `show` children.

**Rationale:** Receipt inspection is conceptually separate from run orchestration and leaves space for future `verify`/`export` commands.

**Alternative:** Add top-level `list-receipts` / `show-receipt`; rejected because it flattens an evidence namespace that will likely grow.

### 2. Sibling receipts directory remains the default source

**Choice:** Use `RunConfig::receipt_dir_path()` as the default list/show run-id lookup directory.

**Rationale:** The existing writer already stores receipts in this sibling directory; operators should not have to memorize that convention.

**Alternative:** Store receipts under the cluster directory; rejected because receipts should survive cluster directory cleanup patterns and already have a separate durable location.

### 3. Summary output is derived from validated receipts

**Choice:** Parse and validate each receipt before producing summary rows. Summary output includes run id, created timestamp, command, stage counts, final status, and path.

**Rationale:** Listing invalid JSON as if it were evidence undermines operator trust. The summary must be derived from the receipt model, not filename guesses.

**Alternative:** Glob filenames only; rejected because it cannot report mode, command, stage status, or validation failures.

## Risks / Trade-offs

**Invalid historical files hidden by list** → `list` should warn when skipping invalid `.json` files so operators know the directory contains non-evidence.

**Raw JSON stability** → `show --json` should re-emit the parsed receipt using the canonical serializer so schema validation happens before output.
