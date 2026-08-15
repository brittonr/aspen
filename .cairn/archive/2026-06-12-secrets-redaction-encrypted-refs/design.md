## Context

Sealed repro export now rejects `<secret>`, `<confidential>`, `<credential>`, `<private>`, and unvalidated `<encrypted-ref>` marker records. Catalogs also redact some known markers. The next step is a canonical confidentiality model that preserves replay and evidence without default plaintext exposure.

## Goals

- Define `secret-ref-v1`, `confidential-label-v1`, `redaction-marker-v1`, `encrypted-ref-v1`, `reveal-receipt-v1`, `decrypt-receipt-v1`, `redaction-transform-receipt-v1`, and `secret-cleanup-receipt-v1`.
- Label secret-bearing fields in envelopes, traces, receipts, snapshots, storage records, transcripts, catalogs, reports, and repro bundles.
- Redact by default in rendered views and unprivileged catalog/MCP calls.
- Gate reveal/decrypt through authority contexts, policy refs, resource refs, effect handles, and audit receipts.
- Allow replay by comparing commitments/hashes when plaintext is unavailable.
- Support encrypted private bundle profiles with explicit transform and reveal receipts.

## Non-Goals

- No custom cryptographic primitive design.
- No plaintext fallback for convenience.
- No authority from possession of ciphertext refs alone.
- No redaction that changes canonical source evidence without a transform receipt.

## Records

```preserves
<secret-ref-v1 "molten.secrets.secret-ref.v1"
  <secret-id <commitment-ref>>
  <scope <authority-or-service-ref>>
  <allowed-use ["decrypt" "sign" ...]>
  <commitment <content-commitment-ref>>
  <encryption <encryption-profile-ref>>
  <redaction-label <label-ref>>
  <expiry <expiry-ref-or-none>>
  <revocation [<revocation-ref> ...]>
  <evidence [<receipt-ref> ...]>
  <checks [<check "no-plaintext-default" "pass"> ...]>>
```

```preserves
<redaction-marker-v1 "molten.secrets.redaction-marker.v1"
  <reason "secret"|"credential"|"private"|"policy">
  <commitment <safe-commitment-ref>>
  <schema <schema-ref>>
  <path <canonical-path-ref>>
  <policy [<policy-ref> ...]>
  <receipt <redaction-receipt-ref>>>
```

```preserves
<reveal-receipt-v1 "molten.secrets.reveal-receipt.v1"
  <decision "pass"|"deny">
  <secret <secret-ref>>
  <requester <authority-context-ref>>
  <purpose "debug"|"replay"|"export"|"adapter-use">
  <plaintext-ref <content-ref-or-none>>
  <commitment <commitment-ref>>
  <diagnostics ["..." ...]>
  <checks [<check "authorized-reveal" "pass"> ...]>>
```

## Rendering Policy

Rendered reports, transcripts, catalogs, diagnostics, and MCP responses replace secret-bearing fields with `redaction-marker-v1` unless the caller supplies a passing reveal receipt. The original canonical evidence remains content-addressed and auditable.

## Replay Policy

If reveal is denied, replay compares secret commitments and redaction marker refs. If an effect truly needs plaintext, replay must use a recorded effect response or an admitted decrypt/reveal effect; otherwise the run is diagnostic-only.
