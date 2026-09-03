# Shareable View Distribution

## ADDED Requirements

### Requirement: Molten distributes Shareable Views without owning UI semantics [r[molten.shareable_view_distribution.boundary]]

Molten MUST treat Shareable View manifests, application state, action descriptors, presentation profiles, and fork meaning as external typed artifacts. Molten MUST own only its distribution, storage, authority-order, retention, and receipt behavior.

#### Scenario: Exact external contract is admitted [r[molten.shareable_view_distribution.boundary.external]]

- GIVEN a caller requests Shareable View distribution
- WHEN profile activation runs
- THEN Molten MUST require one exact published external contract revision, schema profile, fixture identity, bounds set, and compatibility result
- AND a sibling path, draft branch, screenshot, pixel stream, or undocumented manifest MUST NOT satisfy activation

#### Scenario: Distribution request is complete [r[molten.shareable_view_distribution.boundary.request]]

- GIVEN a request binds external manifest, schema profile, dependency root, privacy domain, issuer domain, mode, policy, authority, resources, and non-claims
- WHEN pure request validation runs
- THEN Molten MUST return one deterministic admitted request or bounded rejection
- AND the request MUST NOT contain renderer handles, callbacks, credentials, sockets, paths, mutable backend objects, or undeclared executable members

#### Scenario: Ownership is reviewed [r[molten.shareable_view_distribution.boundary.ownership]]

- GIVEN the distribution profile changes
- WHEN architecture validation runs
- THEN each field, transition, effect, authority input, retention decision, and receipt MUST have one owner
- AND Molten MUST NOT define application state, semantic action, presentation, accessibility, or clone behavior

### Requirement: Distribution verifies one complete dependency closure [r[molten.shareable_view_distribution.closure]]

Molten MUST map one admitted external member inventory into existing content-store and DAG-sync contracts. It MUST verify all required members before exposure.

#### Scenario: External inventory maps into Molten plans [r[molten.shareable_view_distribution.closure.mapping]]

- GIVEN a complete ordered inventory names external roles, references, lengths, privacy classes, and required status
- WHEN pure closure planning runs
- THEN Molten MUST preserve external and Molten content identity domains and produce bounded existing-port plans
- AND it MUST NOT copy DAG, chunk, content, resume, or verification logic into a UI-specific subsystem

#### Scenario: Required closure completes

- GIVEN every required member arrives under the admitted request and resources
- WHEN verification transitions run
- THEN each member MUST pass role, length, position, chunk, whole-object, request, profile, generation, privacy, and authority checks
- AND the receiver MUST receive a complete closure result only after all required members pass

#### Scenario: Optional presentation member is absent

- GIVEN required state and schema members pass but one declared optional presentation member is unavailable
- WHEN closure classification runs
- THEN Molten MAY return a typed degraded result with the missing optional role
- AND it MUST NOT claim complete presentation, renderer support, or application correctness

### Requirement: Authority precedes availability and reveal [r[molten.shareable_view_distribution.privacy]]

Molten MUST obtain current read and privacy admission before local availability, range, resume, reuse, or payload operations. Subscription, action, and fork operations MUST require separate admission.

#### Scenario: Authorized caller requests a member [r[molten.shareable_view_distribution.privacy.admission]]

- GIVEN current caller, object, issuer, privacy, policy, resource, and revocation facts admit one member read
- WHEN availability and transfer planning runs
- THEN Molten MAY inspect local state and return a bounded content plan
- AND later payload reveal MUST revalidate the member against the same admitted request and current authority

#### Scenario: Denied caller probes local state [r[molten.shareable_view_distribution.privacy.no_oracle]]

- GIVEN current authority or privacy policy denies the member
- WHEN the caller requests status, availability, resume, range, reuse, or reveal
- THEN Molten MUST deny before local member lookup
- AND it MUST NOT expose presence, hit, size, age, peer, locator, storage path, or reusable digest facts

### Requirement: Receiver-local reuse is partitioned and measured [r[molten.shareable_view_distribution.reuse]]

Molten MAY reuse complete verified members only within an admitted receiver namespace. The namespace MUST bind privacy domain, issuer policy, external profile, and retention class.

#### Scenario: Compatible retained member is reused [r[molten.shareable_view_distribution.reuse.partition]]

- GIVEN current authority passes and one retained member matches identity, profile, privacy domain, issuer policy, generation, length, and retention requirements
- WHEN closure planning runs
- THEN Molten MAY omit that member from the transfer plan after complete local verification
- AND no externally visible result may reveal shared state across privacy or issuer domains

#### Scenario: Reuse evidence is evaluated [r[molten.shareable_view_distribution.reuse.measurement]]

- GIVEN representative Shareable View workloads cover first transfer, repeat, reconnect, partial progress, corruption, revocation, purge, and route change
- WHEN cost and privacy measurements run
- THEN evidence MUST report transferred bytes, completion time, disk I/O, storage, CPU, memory, recovery, and privacy outcomes
- AND product reuse MUST remain disabled when benefit is absent, evidence is synthetic-only, or policy rejects the privacy cost

### Requirement: Live distribution is snapshot-first and action-safe [r[molten.shareable_view_distribution.live]]

Molten MUST accept one complete external snapshot before live deltas or semantic action traffic for a producer generation. Reliable control facts MUST remain ordered under pressure.

#### Scenario: Live session starts [r[molten.shareable_view_distribution.live.snapshot_first]]

- GIVEN current subscription authority and one complete admitted snapshot closure
- WHEN the live session starts
- THEN Molten MUST bind object, snapshot, producer, stream, state revision, action catalog, privacy, authority, and transport generations
- AND deltas MUST require exact predecessor sequence and revision facts

#### Scenario: Semantic action crosses the route [r[molten.shareable_view_distribution.live.actions]]

- GIVEN a current external action request passes separate transport and authority admission
- WHEN Molten routes the request and observes its outcome
- THEN Molten MUST preserve the typed external request and terminal outcome without interpreting application action meaning
- AND replaceable state or presentation traffic MUST NOT evict ordered action admission or outcome facts

#### Scenario: Mutating action becomes uncertain [r[molten.shareable_view_distribution.live.uncertain]]

- GIVEN disconnect or producer loss prevents a definite mutating-action result
- WHEN session recovery runs
- THEN Molten MUST preserve an uncertain outcome and MUST NOT replay the request automatically
- AND a later request MUST use a new explicit operation after current admission

### Requirement: Fork requests route without clone authority [r[molten.shareable_view_distribution.fork]]

Molten MAY route an authorized fork request to the current external owner. It MUST NOT create application state, select clone policy, activate results, or grant returned-object authority.

#### Scenario: Current owner receives fork request [r[molten.shareable_view_distribution.fork.route]]

- GIVEN a request binds exact external object, snapshot, owner generation, fork profile, caller capability, and operation identity
- WHEN current-owner resolution and route admission pass
- THEN Molten MAY deliver one typed request and return the external owner outcome
- AND delivery or response success MUST NOT grant read, mutation, subscription, activation, or retention authority for a returned object

#### Scenario: Owner or snapshot is stale

- GIVEN owner generation, snapshot, fork profile, capability, or route facts are stale or ambiguous
- WHEN fork routing runs
- THEN Molten MUST deny or return an explicit unavailable or uncertain result
- AND it MUST NOT fabricate a fork, choose another owner, or replay a mutating request

### Requirement: Distribution has simulation and live evidence [r[molten.shareable_view_distribution.validation]]

Molten MUST test positive and negative closure, authority-order, privacy, reuse, live, action, fork, retention, receipt, and cleanup behavior through shared pure transitions.

#### Scenario: Baseline is recorded [r[molten.shareable_view_distribution.validation.baseline]]

- GIVEN implementation is about to change distribution behavior
- WHEN focused baseline validation runs
- THEN current content-store, DAG-sync, transport, authority, retention, simulation, and live-action results MUST be recorded
- AND existing failures and blocked adapters MUST remain visible

#### Scenario: Deterministic matrix runs [r[molten.shareable_view_distribution.validation.matrix]]

- GIVEN fixtures cover complete, degraded, partial, reused, corrupt, revoked, cross-domain, live, action, fork, and cleanup outcomes
- WHEN deterministic simulation runs
- THEN valid cases MUST reach their expected bounded outcomes
- AND every prohibited lookup, reveal, replay, cross-domain correlation, fabricated fork, or overclaim MUST fail at its declared boundary

#### Scenario: Live profile runs [r[molten.shareable_view_distribution.validation.live]]

- GIVEN bounded live Iroh endpoints and admitted fixture authority
- WHEN closure, reconnect, live update, action, close, and cleanup workflows run
- THEN live observations MUST enter the same pure transitions used by simulation
- AND transport success MUST NOT replace authority, content verification, application action success, or fork success

#### Scenario: Receipts remain bounded [r[molten.shareable_view_distribution.validation.receipts]]

- GIVEN internal or public receipt projection runs
- WHEN receipt validation completes
- THEN internal receipts MUST bind exact profile and outcome facts and public receipts MUST use aggregate redacted facts
- AND public output MUST contain no payload, reusable digest, hit identity, endpoint, credential, path, peer secret, or action input

#### Scenario: Change is ready to archive [r[molten.shareable_view_distribution.validation.closeout]]

- GIVEN maintainers intend to accept the distribution profile
- WHEN closeout validation runs
- THEN focused tests, formatting, Clippy, Octet, Cairn, simulation, and relevant Nix checks MUST pass
- AND accepted specifications MUST retain UI semantics, global cache, exactly-once effects, and production readiness as non-claims
