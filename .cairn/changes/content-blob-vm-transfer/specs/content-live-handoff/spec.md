## ADDED Requirements

### Requirement: Explicit read grants precede serving
r[molten.content_live.read_grant] Molten MUST admit finite operator-owned read grants that bind one canonical manifest and explicit reader keys. Protected serving MUST compare the authenticated peer with that grant before exposing blobs. Pinning and transport identity alone MUST NOT grant read access.

#### Scenario: Unlisted reader
- GIVEN a manifest-specific grant to one reader
- WHEN another authenticated reader connects
- THEN the server denies before the blob handler exposes content.

### Requirement: Public handoff is bounded and identity preserving
r[molten.content_live.handoff] Molten MUST separate public locator metadata from private live router state. Handoff admission MUST validate bounds, exact expected manifest, ordered locator membership, and the selected provider before client networking. The existing chunk owner and transition core MUST verify received bytes.

#### Scenario: Changed locator membership
- GIVEN an expected canonical manifest and handoff with a missing, extra, or reordered locator
- WHEN the client admits the handoff
- THEN admission fails before networking.

### Requirement: VM fixture uses independent processes
r[molten.content_live.vm] The fixture MUST run storage and client roles in separate VMs using the real Iroh adapter, persistent canonical storage, and disposable credentials. Receipts MUST distinguish verified transfer, failure, and retention observations without claiming package provenance or production readiness.

#### Scenario: Verified bytes after restart
- GIVEN a pinned object on a persistent guest disk
- WHEN the storage VM restarts and a fresh client fetches it
- THEN reconstruction preserves the original content and manifest identities.
