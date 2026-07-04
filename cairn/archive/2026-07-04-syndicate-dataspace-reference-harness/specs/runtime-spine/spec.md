## ADDED Requirements

### Requirement: Syndicate reference harness is gated behind Molten boundaries
r[molten.syndicate_dataspace.reference_harness] Molten SHOULD provide a Syndicate-backed local dataspace reference harness for adopted local assertion, retraction, Observe, message, and cleanup semantics. The harness MUST consume canonical Molten runtime steps and values, and MUST emit Molten evidence rather than treating Syndicate internals as the public boundary.

#### Scenario: Reference harness consumes canonical input
- GIVEN a canonical Molten runtime step sequence with actor ids, Preserves values, and explicit evidence refs
- WHEN the Syndicate reference harness evaluates the sequence
- THEN the harness consumes those explicit inputs
- AND emits Molten route, assertion, observer, and trace evidence refs.

#### Scenario: Syndicate internals do not bypass admission
- GIVEN a Syndicate-backed harness object can publish an assertion internally
- WHEN the corresponding Molten capability, policy, resource, or authority admission is missing
- THEN the Molten boundary denies before the assertion becomes committed Molten runtime state.

### Requirement: Syndicate parity receipts compare normalized outcomes
r[molten.syndicate_dataspace.parity_receipts] Molten MUST compare adopted current-dataspace and Syndicate-harness outcomes through normalized canonical Preserves events, assertion refs, observer refs, owner refs, route refs, and diagnostics. Differences MUST produce deterministic parity receipts.

#### Scenario: Matching outcomes pass parity
- GIVEN the existing Molten dataspace and the Syndicate reference harness process the same adopted equality Observe fixture
- WHEN their normalized outcomes contain the same assertion, observer, and delivery refs
- THEN Molten emits a parity receipt with decision `pass`.

#### Scenario: Divergent outcomes deny parity
- GIVEN the two interpreters produce different retraction delivery refs for the same input
- WHEN parity comparison runs
- THEN Molten emits a parity receipt with decision `deny`
- AND diagnostics identify the first divergent normalized ref.

### Requirement: Syndicate parity fixtures cover pass and deny paths
r[molten.syndicate_dataspace.fixture_parity] Molten SHOULD maintain positive and negative fixtures for Syndicate reference parity covering assertion, retraction, Observe initial delivery, Observe future delivery, owner cleanup, missing authority, and unsupported pattern cases.

#### Scenario: Positive fixture covers Observe lifecycle
- GIVEN a positive parity fixture with an existing matching assertion and a future matching assertion
- WHEN both dataspace interpreters run the fixture
- THEN parity evidence confirms both initial and future deliveries.

#### Scenario: Negative fixture catches missing authority
- GIVEN a negative parity fixture whose actor attempts a privileged assertion without admitted authority
- WHEN the Syndicate harness could represent the assertion internally
- THEN Molten parity evidence records that the assertion is denied before committed state.

### Requirement: Facet cleanup retracts owner-scoped assertions
r[molten.syndicate_dataspace.facet_cleanup] Molten SHOULD model Syndicate facet or conversation lifetime in the reference harness as Molten actor, session, facet, or live-reference ownership. Cleanup MUST retract owner-scoped assertions and observers deterministically before they remain visible as live state.

#### Scenario: Facet stop retracts assertions
- GIVEN a Syndicate-backed facet owns a local service readiness assertion
- WHEN the facet stops or loses admitted authority
- THEN Molten emits deterministic retraction evidence for the owned assertion
- AND observers no longer see it as live state.

#### Scenario: Shared assertion remains for surviving owner
- GIVEN two owners maintain the same canonical assertion value
- WHEN one owner facet is cleaned up
- THEN the assertion remains visible while the surviving owner remains live
- AND cleanup evidence names only the removed owner ref.

### Requirement: Capability attenuation remains Molten-admitted
r[molten.syndicate_dataspace.cap_attenuation] Molten MAY map admitted Molten capability, authority, and policy decisions onto Syndicate capability or rewrite-style attenuation in the reference harness. Such mapping MUST be derived from explicit Molten admission evidence and MUST NOT make Syndicate cap possession alone sufficient authority.

#### Scenario: Admitted capability narrows publication scope
- GIVEN Molten admission grants an actor assertion authority for one dataspace topic or pattern scope
- WHEN the Syndicate harness publishes through an attenuated cap derived from that evidence
- THEN matching assertions may commit
- AND out-of-scope assertions deny before visibility.

#### Scenario: Syndicate cap alone is not authority
- GIVEN a harness object holds a Syndicate cap but lacks Molten authority, policy, or capability evidence
- WHEN it attempts a privileged assertion or Observe subscription
- THEN Molten denies before committed runtime state
- AND diagnostics state that Syndicate cap evidence is not authority by itself.

### Requirement: Syndicate compatibility is not claimed
r[molten.syndicate_dataspace.no_wire_compat] Molten MUST document that Syndicate crate usage is an implementation aid or reference semantics layer. Molten MUST NOT claim Syndicate wire protocol, relay, sturdyref, capability, trace, service, or authority compatibility unless a future compatibility change explicitly scopes and proves that surface.

#### Scenario: Documentation names Molten boundaries
- GIVEN user-facing documentation describes Syndicate-backed dataspace behavior
- WHEN it explains the adopted runtime pattern
- THEN it names Molten envelopes, Preserves refs, policy gates, authority gates, resource gates, and receipts as normative
- AND it does not claim Syndicate wire, relay, sturdyref, or authority compatibility.
