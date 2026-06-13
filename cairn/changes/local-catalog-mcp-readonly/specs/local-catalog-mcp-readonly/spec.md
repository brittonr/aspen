# local catalog mcp readonly Delta Spec

## ADDED Requirements

### Requirement: Define canonical `catalog-mcp-request-v1` records with tool name, Preserves args, and read-only/no-path-identity checks
r[molten.catalog_mcp.request_dto] Define canonical `catalog-mcp-request-v1` records with tool name, Preserves args, and read-only/no-path-identity checks.

### Requirement: Define canonical `catalog-mcp-response-v1` records with decision, request ref, result ref, payload, catalog receipt ref, diagnostics, and checks
r[molten.catalog_mcp.response_dto] Define canonical `catalog-mcp-response-v1` records with decision, request ref, result ref, payload, catalog receipt ref, diagnostics, and checks.

### Requirement: Define canonical `catalog-mcp-receipt-v1` records binding request, response, catalog-core receipt, refs, diagnostics, and read-only checks
r[molten.catalog_mcp.receipt_dto] Define canonical `catalog-mcp-receipt-v1` records binding request, response, catalog-core receipt, refs, diagnostics, and read-only checks.

### Requirement: Keep registry/ledger filesystem paths outside canonical request identity
r[molten.catalog_mcp.no_path_identity] Keep registry/ledger filesystem paths outside canonical request identity.

### Requirement: Implement a read-only allow-list for `catalog.list`, `catalog.view`, `catalog.search`, `catalog.deps`, `catalog.dependents`, and `catalog.short_id`
r[molten.catalog_mcp.allowlist] Implement a read-only allow-list for `catalog.list`, `catalog.view`, `catalog.search`, `catalog.deps`, `catalog.dependents`, and `catalog.short_id`.

### Requirement: Route read-only calls through `src/catalog.rs` and embed/bind the resulting catalog receipt
r[molten.catalog_mcp.catalog_core_binding] Route read-only calls through `src/catalog.rs` and embed/bind the resulting catalog receipt.

### Requirement: Return canonical deny responses for unknown or mutating tool names
r[molten.catalog_mcp.fail_closed_mutation] Return canonical deny responses for unknown or mutating tool names.

### Requirement: Deny missing or malformed tool arguments as canonical diagnostics
r[molten.catalog_mcp.argument_validation] Deny missing or malformed tool arguments as canonical diagnostics.

### Requirement: Preserve hidden-ref visibility filtering for every read-only tool
r[molten.catalog_mcp.hidden_refs] Preserve hidden-ref visibility filtering for every read-only tool.

### Requirement: Redact view payloads by default unless an explicit local arg disables redaction
r[molten.catalog_mcp.redacted_default] Redact view payloads by default unless an explicit local arg disables redaction.

### Requirement: Resolve short ids through catalog ambiguity/min-length checks before downstream operations
r[molten.catalog_mcp.short_id_expansion] Resolve short ids through catalog ambiguity/min-length checks before downstream operations.

### Requirement: Accept policy/capability/redaction refs in request args and pass them to catalog visibility inputs
r[molten.catalog_mcp.policy_args] Accept policy/capability/redaction refs in request args and pass them to catalog visibility inputs.

### Requirement: Add `molten test catalog mcp-call <request.preserves>` with response and receipt outputs
r[molten.catalog_mcp.cli_call] Add `molten test catalog mcp-call <request.preserves>` with response and receipt outputs.

### Requirement: Add tests that read-only calls match catalog core results and bind catalog receipts
r[molten.catalog_mcp.readonly_tests] Add tests that read-only calls match catalog core results and bind catalog receipts.

### Requirement: Add tests for hidden refs, ambiguous short ids, and mutating tool denial
r[molten.catalog_mcp.denial_tests] Add tests for hidden refs, ambiguous short ids, and mutating tool denial.

### Requirement: Add Hegel properties for deterministic read-only request/response refs and mutation denial
r[molten.catalog_mcp.property_tests] Add Hegel properties for deterministic read-only request/response refs and mutation denial.

