# Domain Capability Audit Evidence

Generated: 2026-05-05T01:55:21Z

## Summary

Phase 3 has converted the audited internal-domain request families from generic data-prefix authorization to domain-specific capabilities in focused verified slices. The latest slice converted KV vault/index metadata requests to `KvMetadataRead` / `KvMetadataWrite` and SQL queries to `SqlRead`.

## Completed focused slices

- **Secrets/trust**
- **SNIX store**
- **Net service mesh**
- **CI pipeline**
- **Jobs/workers**
- **Blob/docs/hooks**
- **Coordination primitives and leases**
- **Observability**
- **Nix binary cache**
- **Automerge documents and sync**
- **KV metadata and SQL**

## KV metadata and SQL slice

- Converted vault key listing and secondary-index list/scan/create/drop requests away from generic KV `_sys:` or raw vault-name `Read`/`Write` scopes.
- Converted SQL execution away from generic `_sql:` `Read` scope.
- Added `Capability::KvMetadataRead`, `Capability::KvMetadataWrite`, and `Capability::SqlRead` plus matching `Operation` variants with prefix containment for delegation.
- Updated root token generation to include broad KV metadata and SQL read capabilities.
- Added regressions `kv_metadata_requests_use_domain_specific_capabilities`, `generic_kv_metadata_prefixes_do_not_authorize_metadata_requests`, `sql_queries_use_domain_specific_capability`, and `generic_sql_prefix_does_not_authorize_sql_queries`.

## Remaining generic internal-domain mappings

Remaining count: **0**

No generic internal-domain mappings remain in audited `to_operation/*_ops.rs` files.
