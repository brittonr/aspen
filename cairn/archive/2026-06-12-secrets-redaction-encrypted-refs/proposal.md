## Why

Molten currently protects portable pass evidence by rejecting sensitive markers. That is safe but too restrictive for real operation: nodes need to store, replay, exchange, and debug secret-bearing workflows without leaking plaintext. Molten needs canonical secret refs, redaction markers, encrypted refs, reveal receipts, and commitment-based replay.

## What Changes

- Add canonical secret refs, confidential field labels, redaction markers, encrypted refs, reveal/decrypt/redact receipts, and secret cleanup receipts.
- Apply redacted views to catalog, MCP, transcripts, reports, remote dataspace diagnostics, and repro bundles.
- Gate decryption/reveal through explicit authority, policy, resource, and effect handles.
- Support replay by comparing commitments when plaintext reveal is not admitted.
- Add encrypted/private repro bundle profile that remains gate-preserving only when reveal/redaction receipts validate.

## Impact

This turns confidentiality from coarse fail-closed rejection into a usable evidence-preserving rail. It is needed for production logs, operator workflows, remote exchange, and dogfood runs involving credentials or private data.
