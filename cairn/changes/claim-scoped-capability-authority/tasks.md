## Tasks

- [ ] [serial] r[molten.claim_authority.subject_selectors] Define canonical claim-domain and subject-selector records with hash-agnostic subject kinds, selector bounds, policy refs, resource refs, and broad-selector diagnostics.
- [ ] [serial] r[molten.claim_authority.claim_records] Define `authority-claim-v1` and `authority-claim-admission-v1` records that bind claims to capability admission, UCAN verification, Basalt enforcement, local policy/resource, freshness, revocation, and diagnostics.
- [ ] [serial] r[molten.claim_authority.capability_profile] Add the claim-authority capability profile using existing token fields: holder/session/context/resource, `claim:attest` ability, scoped claim kinds, attenuation, caveats, expiry, and revocation.
- [ ] [parallel] r[molten.claim_authority.no_parallel_trust] Ensure transport observations, peer sessions, handoff bundles, import receipts, registry discovery, and catalog search cannot satisfy claim authority without the capability/UCAN/Basalt path.
- [ ] [parallel] r[molten.claim_authority.downstream_consumption] Add a pure downstream claim-use decision that consumes admitted claims only for matching subject selectors, claim kinds, freshness, and subsystem policy.
- [ ] [parallel] r[molten.claim_authority.registry_readback] Classify claim, selector, and admission artifacts in ledger/catalog/MCP readback without making discovery authoritative.
- [ ] [serial] r[molten.claim_authority.peer_diagnostics] Extend peer/operator diagnostics to report claim authority as a separate gate from bootstrap, session, transport, policy/resource, provenance, and execution.
- [ ] [serial] r[molten.claim_authority.positive_negative_tests] Add positive fixtures for admitted external claims and negative fixtures for missing proof, wrong holder/session/context, wrong selector, wrong claim kind, revoked issuer/delegation, stale proof, over-broad wildcard, transport-only, registry-only, and local-fixture fallback attempts.
- [ ] [serial] r[molten.claim_authority.validation] Run focused capability/claim/peer/catalog tests plus Cairn validation before archiving.
