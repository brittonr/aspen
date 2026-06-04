## Context

`local-artifact-catalog` provides the read-only query core. This slice adds a deterministic MCP-shaped local surface over that core. It is not a network server and does not implement the full external MCP transport; it defines canonical request/response/receipt artifacts and a CLI harness that future MCP adapters can reuse.

## Goals

- Expose catalog inspection as a small allow-list of read-only tool names.
- Keep registry/ledger filesystem paths outside request identity; CLI supplies them as local IO handles.
- Reuse the catalog core for all semantics and receipts.
- Redact payload views by default.
- Preserve hidden-ref visibility filtering.
- Expand short ids before downstream operations.
- Return canonical denial responses for missing args, ambiguous short ids, unknown tools, or mutating tool names.

## Non-Goals

- No network listener or JSON-RPC server in this slice.
- No mutating MCP tools.
- No ambient access to registry, ledger, filesystem, environment, clock, or network from request records.
- No bypass of catalog visibility/redaction policy.

## Request model

```preserves
<catalog-mcp-request-v1 "molten.catalog.mcp-request.v1"
  <tool "catalog.list" | "catalog.view" | "catalog.search" | "catalog.deps" | "catalog.dependents" | "catalog.short_id" | ...>
  <args [<kind "doc"> <reference <ref-or-short>> <hidden-ref <ref>> ...]>
  <checks [<check "read-only-surface" "pass"> ...]>>
```

Arguments are tool-specific Preserves records. Registry and ledger paths are not arguments because paths are local IO handles, not canonical catalog identity.

## Response and receipt model

```preserves
<catalog-mcp-response-v1 "molten.catalog.mcp-response.v1"
  <tool ...>
  <decision "pass"|"deny">
  <request <request-ref>>
  <result <none>|<some <catalog-result-or-short-id-ref>>>
  <payload <catalog-result-v1 ...>|<short-id-resolution-v1 ...>|<none>>
  <catalog-receipt <none>|<some <catalog-receipt-ref>>>
  <diagnostics ["..."]>
  <checks [<check "read-only-tool" "pass"> ...]>>
```

```preserves
<catalog-mcp-receipt-v1 "molten.catalog.mcp-receipt.v1"
  <tool ...>
  <decision "pass"|"deny">
  <request <request-ref>>
  <response <response-ref>>
  <catalog-receipt <none>|<some <catalog-receipt-ref>>>
  <refs [<request-ref> <response-ref> <catalog-receipt-ref> ...]>
  <diagnostics ["..."]>
  <checks [<check "canonical-receipt" "pass"> <check "mutating-tools-denied" "pass"> ...]>>
```

## Tool mapping

- `catalog.list`: optional `<kind "...">`.
- `catalog.view`: required `<reference "...">`; optional `<payload #t|#f>` and `<redacted #t|#f>`, default redacted true.
- `catalog.search`: filter args matching catalog filters, root args, include-dependency/dependent booleans.
- `catalog.deps`: required reference, optional transitive boolean.
- `catalog.dependents`: required reference, optional transitive boolean.
- `catalog.short_id`: required prefix, optional min-length.

Every tool also accepts visibility args: repeated `<hidden-ref <ref>>`, `<policy-ref <ref>>`, `<capability-ref <ref>>`, and optional `<redaction-profile-ref <ref>>`.

## Denial behavior

Deny as canonical data, not side effects:

- Missing required args return a deny response.
- Unknown or mutating tools return a deny response with `read-only-tool` fail and `mutating-tools-denied` pass.
- Ambiguous or too-short short ids return the catalog core's deny result.
- Hidden refs are filtered before candidate/result rendering.
