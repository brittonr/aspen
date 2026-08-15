# Local Catalog Mcp Readonly Specification

## Purpose

Defines the `local-catalog-mcp-readonly` capability.

## Requirements

### Requirement: System MUST Define canonical `catalog-mcp-request-v1` records with tool name, Preserves args, and read-only/no-path-identity checks
r[molten.catalog_mcp.request_dto] The system MUST Define canonical `catalog-mcp-request-v1` records with tool name, Preserves args, and read-only/no-path-identity checks.

### Requirement: System MUST Define canonical `catalog-mcp-response-v1` records with decision, request ref, result ref, payload, catalog receipt ref, diagnostics, and checks
r[molten.catalog_mcp.response_dto] The system MUST Define canonical `catalog-mcp-response-v1` records with decision, request ref, result ref, payload, catalog receipt ref, diagnostics, and checks.

### Requirement: System MUST Define canonical `catalog-mcp-receipt-v1` records binding request, response, catalog-core receipt, refs, diagnostics, and read-only checks
r[molten.catalog_mcp.receipt_dto] The system MUST Define canonical `catalog-mcp-receipt-v1` records binding request, response, catalog-core receipt, refs, diagnostics, and read-only checks.

### Requirement: System MUST Keep registry/ledger filesystem paths outside canonical request identity
r[molten.catalog_mcp.no_path_identity] The system MUST Keep registry/ledger filesystem paths outside canonical request identity.

### Requirement: System MUST Implement a read-only allow-list for `catalog.list`, `catalog.view`, `catalog.search`, `catalog.deps`, `catalog.dependents`, and `catalog.short_id`
r[molten.catalog_mcp.allowlist] The system MUST Implement a read-only allow-list for `catalog.list`, `catalog.view`, `catalog.search`, `catalog.deps`, `catalog.dependents`, and `catalog.short_id`.

### Requirement: System MUST Route read-only calls through `src/catalog/mod.rs` and embed/bind the resulting catalog receipt
r[molten.catalog_mcp.catalog_core_binding] The system MUST Route read-only calls through `src/catalog/mod.rs` and embed/bind the resulting catalog receipt.

### Requirement: System MUST Return canonical deny responses for unknown or mutating tool names
r[molten.catalog_mcp.fail_closed_mutation] The system MUST Return canonical deny responses for unknown or mutating tool names.

### Requirement: System MUST Deny missing or malformed tool arguments as canonical diagnostics
r[molten.catalog_mcp.argument_validation] The system MUST Deny missing or malformed tool arguments as canonical diagnostics.

### Requirement: System MUST Preserve hidden-ref visibility filtering for every read-only tool
r[molten.catalog_mcp.hidden_refs] The system MUST Preserve hidden-ref visibility filtering for every read-only tool.

### Requirement: System MUST Redact view payloads by default unless an explicit local arg disables redaction
r[molten.catalog_mcp.redacted_default] The system MUST Redact view payloads by default unless an explicit local arg disables redaction.

### Requirement: System MUST Resolve short ids through catalog ambiguity/min-length checks before downstream operations
r[molten.catalog_mcp.short_id_expansion] The system MUST Resolve short ids through catalog ambiguity/min-length checks before downstream operations.

### Requirement: System MUST Accept policy/capability/redaction refs in request args and pass them to catalog visibility inputs
r[molten.catalog_mcp.policy_args] The system MUST Accept policy/capability/redaction refs in request args and pass them to catalog visibility inputs.

### Requirement: System MUST Add `molten test catalog mcp-call <request.preserves>` with response and receipt outputs
r[molten.catalog_mcp.cli_call] The system MUST Add `molten test catalog mcp-call <request.preserves>` with response and receipt outputs.

### Requirement: System MUST Add tests that read-only calls match catalog core results and bind catalog receipts
r[molten.catalog_mcp.readonly_tests] The system MUST Add tests that read-only calls match catalog core results and bind catalog receipts.

### Requirement: System MUST Add tests for hidden refs, ambiguous short ids, and mutating tool denial
r[molten.catalog_mcp.denial_tests] The system MUST Add tests for hidden refs, ambiguous short ids, and mutating tool denial.

### Requirement: System MUST Add Hegel properties for deterministic read-only request/response refs and mutation denial
r[molten.catalog_mcp.property_tests] The system MUST Add Hegel properties for deterministic read-only request/response refs and mutation denial.
