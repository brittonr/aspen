## Why

Molten already treats runtime communication, storage, and evidence as canonical Preserves values, but artifact identity still needs a stricter operational rail: every executable or declarative artifact should be addressable by a stable content ref derived from canonical bytes, not filenames, package names, raw source text, or rendered diagnostics.

Unison is useful prior art because it shows the leverage of immutable definition identity. Molten should adapt that principle for Wasm components, Steel predicates, Nickel contracts, Preserves schemas, Trellis projections, migration recipes, transcripts, and native descriptors while preserving Molten's own BLAKE3, Preserves, policy, capability, provenance, and receipt boundaries.

## What Changes

- Add canonical artifact identity receipts that bind artifact kind, domain separator, canonical payload ref, schema refs, dependency summary refs, policy refs, provenance refs, and identity checks.
- Require artifact installation paths to normalize supported payloads before hashing whenever a canonical IR, component, schema, or manifest representation exists.
- Deny install/use attempts that present only mutable names, raw source text, rendered logs, or unreviewed hash algorithms as identity.
- Add positive and negative fixtures for stable canonical ids, domain separation, canonicalizer drift, unsupported artifact kinds, and source-text-only identity.

## Impact

- **Files**: artifact registry core, install receipts, artifact show/search diagnostics, fixtures, docs.
- **Testing**: positive fixtures for stable ids across repeated normalization; negative fixtures for wrong domain, raw-source hash, missing canonicalizer, and tampered canonical bytes.
- **Security**: content addressing remains identity only. Artifact safety still requires capability, policy, provenance, source-gate, resource, effect, and execution admission.