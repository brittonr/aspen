## Context

Octet source-gate validation constructs a consumer-specific requirement from the consumer, subject ref, and source scope. It then validates an `octet-gate-receipt-v1` using current config/profile metadata, clean strict checks, and object-corpus/fingerprint check names. The receipt does not embed raw object-corpus source paths, so downstream validation must rely on explicit receipt checks that represent the gate's configured source-scope coverage.

The existing ref validator only checks `blake3:`/`b3:` prefixes with non-empty suffixes. That admits fake refs such as `blake3:test-fingerprint` as pass-shaped source-gate evidence.

## Decisions

### 1. Source-scope coverage is represented by a named receipt check

**Choice:** Add an `object-corpus-source-scope` check to strict Octet gate receipts and require source-gate validation to see it pass. The gate sets that check only when object-corpus coverage includes every configured source-gate source path.

**Rationale:** Source-gate validation receives the gate receipt value, not the raw object corpus. A stable named check preserves the existing receipt schema while making the source-scope coverage invariant explicit and fail-closed for legacy receipts.

### 2. Custom source scopes must be inside the configured inventory

**Choice:** Validate requested source-scope paths against the configured source-gate inventory and deny unknown paths.

**Rationale:** Without raw object-corpus paths in the validation input, arbitrary custom paths cannot be proven covered by the receipt. Denying unsupported paths keeps the trust boundary explicit.

### 3. Ref validation checks exact lowercase BLAKE3 hex grammar

**Choice:** Treat `blake3:<64 lowercase hex>` and `b3:<64 lowercase hex>` as valid content refs. Reject empty, short, long, uppercase, or non-hex suffixes.

**Rationale:** Prefix-only validation lets synthetic strings masquerade as evidence refs. Exact grammar makes malformed ref denial deterministic while preserving the existing `b3:` object-set hash scheme.

## Risks / Trade-offs

- Existing pass-shaped receipts without the new source-scope check will deny until regenerated with the updated gate.
- Custom `--source-scope` paths outside the configured inventory require a future schema extension that binds raw object-corpus source paths or a broader configured inventory.
