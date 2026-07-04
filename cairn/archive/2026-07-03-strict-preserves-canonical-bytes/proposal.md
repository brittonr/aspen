## Why

`parse_canonical_bytes` is the entry point many storage, ledger, transport, and execution paths trust when they receive packed Preserves bytes. The helper currently parses packed bytes and returns a value, but it does not prove that the original bytes were the canonical encoding of that value. A trust boundary named canonical should reject alternate encodings instead of silently normalizing them after parse.

## What Changes

- Add strict canonical decode semantics for packed Preserves bytes.
- Reject non-canonical encodings before ledger import, remote ingress, chunk metadata, typed-storage reads, Wasm output, and transport fetch paths accept the value.
- Return deterministic diagnostics or deny receipts when external bytes parse but do not match their re-encoded canonical form.
- Add positive canonical roundtrip tests and negative non-canonical/tampered byte tests.

## Impact

- **Files**: `preserves_rail`, ledger, chunk store, typed storage, remote dataspace, node Iroh ingress, Iroh exchange, Wasm executor, and related tests.
- **Testing**: canonical bytes continue to pass; parseable but non-canonical packed bytes fail closed with explicit diagnostics.
