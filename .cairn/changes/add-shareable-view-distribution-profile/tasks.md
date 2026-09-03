# Tasks: Add Shareable View distribution profile

## Baseline and contract admission

- [ ] [serial] Record current content-store, DAG-sync, transport-session, authority, retention, simulation, and live-action baselines. r[molten.shareable_view_distribution.validation.baseline]
  - Verify: existing failures, blocked adapters, and support limits remain visible.
- [ ] [serial] Admit one exact published Tile Shareable View contract and record schema, fixture, identity, role, bound, license, and non-claim facts. r[molten.shareable_view_distribution.boundary.external]
  - Verify: a sibling path, draft branch, screenshot, pixel stream, or undocumented manifest cannot satisfy the profile.
- [ ] [parallel] Inventory external member roles, Molten ports, authority order, privacy domains, retention classes, live lanes, fork routing, and evidence owners. r[molten.shareable_view_distribution.boundary.ownership]
  - Verify: Molten does not absorb application state, UI, action, presentation, or clone semantics.

## Pure profile and policy

- [ ] [serial] Add typed Nickel policy for external profiles, closure bounds, roles, bytes, privacy domains, issuer domains, reuse, retention, live streams, actions, fork routes, receipts, and non-claims. r[molten.shareable_view_distribution.boundary.request] r[molten.shareable_view_distribution.privacy.admission]
  - Verify: unbounded members, global inventory, cross-domain reuse, authority-after-lookup, executable payloads, hidden locators, and overclaims fail export and Rust validation.
- [ ] [serial] Add pure request, role mapping, closure, privacy, reuse, live, action, fork, outcome, and receipt types. r[molten.shareable_view_distribution.boundary.request] r[molten.shareable_view_distribution.closure.mapping]
  - Verify: external, Molten content, transport, locator, authority, action, and fork identities cannot interchange.
- [ ] [parallel] Add positive and negative profile fixtures for complete, degraded, partial, reused, corrupt, revoked, cross-domain, live, action, fork, and cleanup cases. r[molten.shareable_view_distribution.validation.matrix]
  - Verify: fixtures contain no real secrets, reusable public digests, endpoints, credentials, or paths.

## Closure transfer and private reuse

- [ ] [serial] Map admitted external inventories into existing content-store and DAG-sync plans without duplicating their logic. r[molten.shareable_view_distribution.closure.mapping]
  - Verify: every required member passes exact role, length, chunk, whole-object, request, profile, generation, privacy, and authority checks before exposure.
- [ ] [serial] Enforce authority and privacy admission before local availability, range, resume, reuse, or payload operations. r[molten.shareable_view_distribution.privacy.admission] r[molten.shareable_view_distribution.privacy.no_oracle]
  - Verify: denied callers receive no presence, hit, size, age, peer, locator, path, or timing-class result intended as an API fact.
- [ ] [serial] Add privacy-domain and issuer-partitioned retained-member reuse through the existing content store. r[molten.shareable_view_distribution.reuse.partition]
  - Verify: equal bytes across domains cannot produce a shared observable hit; corrupt, stale, expired, revoked, or incompatible state cannot satisfy reuse.
- [ ] [parallel] Measure real Shareable View duplicate transfer, completion time, disk I/O, storage, CPU, memory, restart, purge, and privacy costs. r[molten.shareable_view_distribution.reuse.measurement]
  - Verify: product reuse remains disabled when benefit is absent, evidence is synthetic-only, or privacy cost exceeds the admitted profile.

## Live state, actions, and fork routing

- [ ] [serial] Add snapshot-first live-session transitions over external object, snapshot, producer, stream, revision, action-catalog, privacy, authority, and transport generations. r[molten.shareable_view_distribution.live.snapshot_first]
  - Verify: updates before baseline, gaps, stale generations, privacy drift, and authority drift require repair or close without guessed state.
- [ ] [serial] Route typed external action requests and outcomes on the reliable control lane. r[molten.shareable_view_distribution.live.actions]
  - Verify: Molten does not interpret action meaning; stale, denied, disconnected, timed-out, duplicate, and uncertain outcomes remain distinct.
- [ ] [parallel] Add mutating-action uncertainty and reconnect negatives. r[molten.shareable_view_distribution.live.uncertain]
  - Verify: uncertain mutations are not replayed automatically and late outcomes cannot cross generations.
- [ ] [serial] Add current-owner fork request routing without clone, activation, or returned-object authority. r[molten.shareable_view_distribution.fork.route]
  - Verify: owner replacement, stale snapshot, missing capability, unsupported profile, and ambiguous delivery produce no fabricated fork.

## Conformance and closeout

- [ ] [parallel] Run deterministic simulation for corruption, truncation, gap, partition, cancellation, revocation, restart, reuse, action uncertainty, fork denial, and cleanup. r[molten.shareable_view_distribution.validation.matrix]
  - Verify: the same pure transitions classify deterministic and live observations.
- [ ] [parallel] Run bounded live Iroh closure, reconnect, live-update, action, close, and cleanup fixtures. r[molten.shareable_view_distribution.validation.live]
  - Verify: transport success never substitutes for authority, content verification, application action success, or fork success.
- [ ] [serial] Add internal and public-redacted receipts plus leak and overclaim tests. r[molten.shareable_view_distribution.validation.receipts]
  - Verify: public output contains no payloads, reusable digests, hit identities, endpoints, credentials, paths, or action inputs.
- [ ] [serial] Run focused tests, formatting, Clippy, Octet, Cairn gates, simulation, and relevant Nix checks. r[molten.shareable_view_distribution.validation.closeout]
  - Verify: accepted requirements sync before archive; global cache, UI semantics, and production claims remain absent.
