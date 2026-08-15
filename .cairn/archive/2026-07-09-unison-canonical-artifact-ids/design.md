## Context

The accepted artifact registry already treats Unison/UCM as non-normative prior art. This change narrows the next hardening slice: make canonical artifact id derivation observable and auditable for every install/use path.

Molten-owned identity uses BLAKE3 over canonical Preserves or reviewed canonical bytes. Names, aliases, file paths, package versions, transport URLs, and rendered output are lookup metadata only.

## Design

### Identity flow

```text
input payload or content ref
  -> artifact-kind-specific canonicalizer
  -> canonical payload bytes/ref
  -> domain-separated BLAKE3 artifact ref
  -> artifact-identity-receipt-v1
  -> install/use admission gates
```

The pure core validates in-memory identity inputs: artifact kind, canonical payload ref, declared domain separator, schema refs, dependency summary refs, and supported hash algorithm. The imperative shell reads files or blobs, invokes canonicalizers, persists receipts, and calls policy/provenance/capability/resource/source gates.

### Canonical payload boundary

Artifact kinds may have different canonical payload forms:

- Wasm components use component bytes plus inspected import/export metadata.
- Preserves schemas use normalized schema artifacts.
- Nickel contracts use deterministic checked/exported forms.
- Steel predicates use reviewed normalized source plus callable metadata.
- Trellis artifacts use canonical choreography/projection records.
- Transcripts and docs use parsed canonical stanza/comment records, not rendered markdown alone.

When a kind has no reviewed canonicalizer, install must deny or classify the input as opaque content that cannot satisfy executable or policy-bearing artifact roles.

### Domain separation

Each artifact kind has an explicit identity domain. Identical bytes in two domains produce distinct artifact refs and cannot be substituted across kind boundaries without a compatibility receipt.

### Non-goals

- Do not adopt Unison hash formats, syntax, UCM codebase semantics, or Unison typechecking.
- Do not treat content identity as safety, authority, provenance, or execution admission.
- Do not hash raw source text when a canonical representation exists.
- Do not add filesystem, network, clock, or environment reads to pure identity cores.