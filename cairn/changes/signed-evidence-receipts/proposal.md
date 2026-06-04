# Change: signed-evidence-receipts

## Why

Molten receipts and sealed bundles are canonical and content-addressed, but they are not yet attributable. Operators need to know which harness/runtime identity produced a receipt, whether the signing key was authorized for that evidence class, and whether receipts form a verifiable chain across export, verify, unpack, gate, and future distributed exchange.

## What

- Add signed receipt envelopes for gate receipts, repro verify receipts, redaction/reveal receipts, and future runtime receipts.
- Bind signatures to canonical Preserves bytes, schema ids, signer identity refs, key purpose, timestamp/clock evidence, and optional parent receipt refs.
- Use fail-closed verification before a signed receipt can satisfy a policy or pass gate.
- Separate content refs from trust: Blake3 refs identify bytes; signatures attribute and authorize evidence.
- Prepare for transparency-log or ledger anchoring without requiring a network service in the first slice.

## Impact

Unsigned local receipts can remain useful diagnostics during development. Evidence-bearing pass gates can be configured to require signatures once key fixtures and trust roots are available.
