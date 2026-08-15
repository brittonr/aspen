## Why

Molten traces, snapshots, receipts, catalogs, transcripts, effect logs, and replay data may contain secrets, capabilities, payloads, or sensitive metadata. Deterministic playback and rich evidence must not become a secret exfiltration path.

## What Changes

- Define secret references, confidential payload labels, redaction policies, and safe rendering rules.
- Require traces, receipts, snapshots, catalogs, transcripts, and replay logs to distinguish public hashes/metadata from protected content.
- Support encrypted blobs/storage records and capability-gated decryption.
- Add redaction markers that preserve audit structure without revealing secret bytes.
- Require handler profiles and transcript runners to declare whether they may record secret-bearing effects.
- Integrate visibility with catalog/MCP, retention/GC, authority/revocation, typed storage, remote sync, and deterministic replay.

## Impact

This lets Molten keep strong evidence while protecting secrets. The first milestone can mark selected fields as confidential, redact rendered traces/catalog output, and deny transcript/replay export without appropriate authority.
