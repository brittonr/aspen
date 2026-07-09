## Tasks

- [ ] [serial] r[molten.artifacts.canonical_id_receipts] Define `artifact-identity-receipt-v1` records that bind artifact kind, domain separator, canonical payload ref, schema refs, dependency summary refs, policy refs, provenance refs, and identity checks.
- [ ] [serial] r[molten.artifacts.normalized_payload_boundary] Route supported artifact install paths through reviewed canonicalizers before identity hashing and classify unsupported opaque content conservatively.
- [ ] [parallel] r[molten.artifacts.domain_separated_identity] Enforce artifact-kind domain separation so identical bytes cannot satisfy another artifact kind without explicit compatibility evidence.
- [ ] [parallel] r[molten.artifacts.install_rejects_noncanonical] Deny install/use attempts that rely on mutable names, raw source text, rendered logs, unsupported algorithms, or missing canonical payload refs as identity.
- [ ] [serial] r[molten.artifacts.canonical_identity_validation] Add positive and negative fixtures for stable ids, repeated normalization, wrong domains, canonicalizer drift, unsupported kinds, and tampered canonical bytes.