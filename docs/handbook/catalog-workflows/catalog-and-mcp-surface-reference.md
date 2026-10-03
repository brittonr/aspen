# Catalog and MCP surface reference

Mode: Reference

This is a source-checked map for selecting a discovery surface and interpreting its evidence. It is not a server deployment guide or an execution-authority specification. No commands or tests were executed for this page. The [technical companion](../../technical/foundations/evidence-and-authority-separation.md) explains why discovery and receipts cannot confer authority.

## Entry points and ownership

| Surface | Owner | Boundary |
| --- | --- | --- |
| Catalog library | [Catalog entry file](../../../src/catalog/mod.rs) and its included parts | Reads supplied registry/ledger roots and constructs canonical query artifacts |
| MCP-style library | [MCP entry file](../../../src/catalog/mcp.rs) and included dispatch | Parses Preserves requests, checks a tool allow-list, wraps catalog results |
| CLI | [Catalog arguments](../../../src/cli/core/catalog/command.rs) and [handlers](../../../src/cli/core/catalog/ops.rs) | Reads files, passes roots, prints values, optionally writes response/receipt files |
| Export routing | [Library](../../../src/lib.rs) and [main](../../../src/main.rs) | Physical catalog files are also reached through inventory-named module aliases |
| Root spelling | [Included root declarations](../../../src/main/root/parts/command/p000/body.rs) | Catalog belongs under `molten test catalog` |

“MCP-style” here means the local Preserves request/response surface inspected in these files. The catalog CLI's `mcp-call` reads a request file and invokes it directly. These sources do not establish an HTTP listener, JSON-RPC transport, remote authentication composition, or compatibility with an external MCP client.

## CLI selection table

These are subcommand names, not runnable examples. All are declared in the catalog argument file linked above.

| Subcommand | Required inputs | Useful optional inputs | Output meaning |
| --- | --- | --- | --- |
| `list` | `--registry` | `--ledger`, `--kind`, `--hide-ref` | Visible summaries |
| `search` | `--registry` | `--root`, typed filters, `--ledger` | Intersection of filters within scope |
| `view` | Positional reference, `--registry` | `--payload`, `--ledger`, `--hide-ref` | Summary and rendered content slot |
| `deps`, `dependents` | Positional reference, `--registry` | `--transitive`, `--ledger` | Declared dependency relationships |
| `short-id` | Positional prefix, `--registry` | `--min-length`, `--ledger` | Resolution decision and visible candidates |
| `chunks` | `--chunks` | `--hide-ref` | Local chunk manifest/availability/pin discovery |
| `mcp-call` | Request file, `--registry` | `--ledger`, `--chunks`, `--out` | MCP response envelope |
| `show` | Artifact file | None | Supported catalog/MCP artifact summary |

Query commands provide `--receipt-out`; `show` does not. Output files are shell effects, not registry mutation. The writer does not reserve a fresh path automatically. Standard output may contain multiple values or a receipt-written notice, so it is not uniformly one serialized result.

## MCP tool families

The exact allow-list is owned by [MCP dispatch](../../../src/catalog/parts/mcp/p000/body.rs). These groups retain the declared names rather than presenting invented aliases.

| Family | Allowed names | Main arguments or routing |
| --- | --- | --- |
| General discovery | `catalog.list`, `list_artifacts`; `catalog.search`, `search_artifacts`, `explain_evidence` | Kind or search filters; explanation routes to search |
| Views | `catalog.view`, `view_artifact`, `view_transcript` | `reference`; `payload` defaults false, `redacted` defaults true |
| Graph | `catalog.deps`, `list_dependencies`; `catalog.dependents`, `list_dependents`; `impact_query` | `reference`, `transitive` |
| Receipt discovery | `view_receipts`, `show_receipt`, `search_receipts` | Graph input with required `reference`, not arbitrary receipt-search arguments |
| Specialized search | `search_by_schema`, `search_by_effect` | Required `schema-ref` or `effect-ref` |
| Lifecycle views | `search_transcripts`, `list_upgrade_sessions`, `show_release_snapshot` | Transcript/upgrade filters; snapshot optionally routes to view |
| Evidence search | `list_provenance`, `search_provenance`, `search_retention_gc`, `search_replay_evidence` | Family-specific text classifications plus general filters |
| Chunk discovery | `catalog.chunk_store`, `search_chunk_store` | Caller must separately supply chunk root |
| Short identifiers | `catalog.short_id`, `short_id_resolve` | `prefix`, optional `min-length` |

[Search adapters](../../../src/catalog/parts/mcp/p001/body.rs) implement the specialized mappings. Without explicit filters, transcript search selects status `pass`, while upgrade-session search selects `planned`. Those are query defaults, not a complete history. Search roots use repeated `root` argument records. Visibility uses `hidden-ref`, whereas the CLI flag is spelled `--hide-ref`.

## Canonical artifacts and fields

| Record | Important fields | Owner |
| --- | --- | --- |
| `catalog-query-v1` | `operation`, `scope`, `filters`, `visibility`, `render`, `checks` | [Query builder](../../../src/catalog/parts/mod/p007/body.rs) |
| `catalog-summary-v1` | `artifact` triple, `names`, `schemas`, `dependencies`, `dependents`, `effects`, `policy`, `evidence`, `classifications`, `visibility` | [Summary builder](../../../src/catalog/parts/mod/p006/body.rs) |
| `catalog-result-v1` | `query`, `decision`, `results`, `diagnostics`, `checks` | Query builder |
| `catalog-receipt-v1` | `operation`, `decision`, `query`, optional `result`, `refs`, `diagnostics`, `checks` | Query builder |
| `catalog-mcp-request-v1` | `tool`, `args`, `checks` | MCP dispatch |
| `catalog-mcp-response-v1` | `tool`, `decision`, `request`, optional `result`, `payload`, optional `catalog-receipt`, `diagnostics`, `checks` | [MCP envelope builder](../../../src/catalog/parts/mcp/p001/body.rs) |
| `catalog-mcp-receipt-v1` | `tool`, `decision`, `request`, `response`, optional `catalog-receipt`, `refs`, `diagnostics`, `checks` | MCP envelope builder |

Each record also carries its schema identifier. Canonical Preserves plus BLAKE3 define identity; these tables do not define identity through Rust layout or rendered text.

## Worked interpretation and limits

Suppose an allowed view fails to resolve its subject after request parsing. Dispatch wraps the error as a deny response without a catalog-result binding. An unlisted mutation name is also denied, but at the allow-list boundary. Malformed requests can fail earlier instead of producing a wrapped denial. These three cases require different diagnostics even though none authorizes execution.

The catalog declares 100,000-item and 4,096-reference bounds; MCP declares 512 arguments and filters, 4,096 references, and 128 checks. They are separate limits, not a guarantee that every query reaches the largest item bound: receipt construction also accumulates item references. Visibility reference validation checks shape, not current policy admission. Raw rendering is representable through MCP; default redaction is not an immutable confidentiality boundary.

## Sources

- [Handbook](../README.md)
- [Architecture](../../architecture.md)
- [Technical companion: evidence and authority separation](../../technical/foundations/evidence-and-authority-separation.md)
- [Catalog types and limits](../../../src/catalog/parts/mod/p000/body.rs)
- [MCP allow-list, parsing, and limits](../../../src/catalog/parts/mcp/p000/body.rs)
- [MCP argument readers](../../../src/catalog/parts/mcp/p002/body.rs)
- [MCP filter mapping](../../../src/catalog/parts/mcp/p004/body.rs)
