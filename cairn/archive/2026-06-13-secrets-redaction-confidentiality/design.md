## Context

Molten intentionally records canonical traces and receipts. Those records can include capabilities, UCANs, keys, tokens, payloads, storage keys, peer identities, or operational metadata. A confidentiality layer ensures evidence is useful without revealing protected data to every reader.

## Goals

- Represent secrets as references or encrypted content, not ambient strings.
- Label confidential fields in envelopes, traces, receipts, snapshots, and catalog views.
- Redact display and export while preserving hash/evidence structure.
- Gate decryption and reveal through explicit capabilities and receipts.
- Prevent transcript/replay/test artifacts from accidentally persisting secrets.
- Support deterministic replay with recorded secret hashes or authorized secret fixtures.

## Non-Goals

- Do not make redaction a substitute for access control.
- Do not log plaintext secrets by default.
- Do not allow catalog or MCP tools to reveal protected content without policy admission.
- Do not pretend redacted evidence can prove plaintext semantics unless appropriate commitments are present.

## Secret refs

A secret ref should include:

- secret id/content commitment,
- owner/scope,
- allowed use/effect scope,
- encryption/key refs if stored,
- redaction label,
- expiry/revocation refs,
- reveal policy refs,
- evidence refs.

Secret-bearing effects pass secret refs or encrypted content refs where possible.

## Redaction model

Redaction replaces protected content with a canonical marker containing:

- redaction reason/class,
- original content hash or commitment if safe,
- schema/path of redacted field,
- authority/policy that performed redaction,
- receipt ref.

Rendered docs, catalog views, transcript outputs, and diagnostics use redacted views unless the caller has reveal authority.

## Encryption

Encrypted blobs/storage records should bind ciphertext, encryption metadata, schema refs, policy refs, and content commitments. Decryption is an effect that requires authority and emits receipts. Deterministic replay can inject recorded decrypted values only within an authorized replay scope or compare commitments without reveal.

## Trace and snapshot safety

Trace records and snapshots must be classified before export. Handler profiles declare whether effect responses may contain secrets and whether record/replay logs store plaintext, encrypted payloads, or commitments only.

## Open Questions

- Which field-labeling mechanism should be used first: schema annotations, policy overlays, or both?
- How should deterministic replay run when secret plaintext is unavailable but commitments match?
- Which encryption envelope format should Molten standardize on first?
