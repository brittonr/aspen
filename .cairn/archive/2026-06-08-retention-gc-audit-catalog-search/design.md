# retention-gc-audit-catalog-search Design

## Overview
Retention GC audit chains are read-only evidence artifacts. Catalog classification should expose their public scope so operators and MCP clients can find plan/apply/execute/audit links without reading private payloads or treating those artifacts as gates.

## Catalog Classifications
The catalog classifies retention GC artifacts with stable text markers:

- `retention-gc:plan`, `retention-gc:apply`, `retention-gc:execute`, or `retention-gc:audit`,
- `retention-gc-stage:<stage>`,
- `retention-gc-decision:<decision>`,
- `retention-gc-subsystem:<subsystem>`,
- `retention-gc-action:<action>`,
- `retention-gc-object:<object-ref>`,
- `retention-gc-class:<retention-class>`,
- chain refs such as `retention-gc-plan:<ref>`, `retention-gc-apply:<ref>`, `retention-gc-execution:<ref>`, `retention-gc-receipt:<ref>`, and `retention-gc-tombstone:<ref>` when present.

The ledger maps `retention-gc-audit-v1` to `retention-gc-audit` so catalog `ledger-kind` filters can find audit artifacts imported into the ledger.

## MCP Search
The read-only MCP allow-list includes `search_retention_gc`. It builds a normal catalog search constrained to `retention-gc:` markers and optional `stage`, `object-ref`, `subsystem`, `decision`, `plan-ref`, `apply-ref`, and `execution-ref` arguments. This is an inspection surface only; it returns the same catalog receipt evidence as other read-only catalog tools.

## Safety Boundaries
Catalog classifications and MCP results are operator discovery evidence. They MUST NOT replace retention plans, apply receipts, execution gates, destructive admission, or imported remote clearance. Destructive commands continue to fail closed unless their normal plan/apply/execution/admission gates pass.
