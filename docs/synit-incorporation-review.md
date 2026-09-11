# Synit manual review for Molten

Status: review notes, not a decision. Nothing here is implemented.

Source: *The Synit Manual* (<https://synit.org/book/>), licensed CC BY 4.0,
copyright © 2021–2023 Tony Garnock-Jones. Local saved copy:
`~/.local/share/mantle-references/synit-book/` (`print.html`, 53 per-page
markdown files, `synit-book.md`, `README.md` with BLAKE3 digests).

Molten already treats Synit and the Syndicated Actor Model as non-normative
references (`.cairn/specs/runtime-spine/spec.md`), and the Syndicate crate is a
reference-semantics harness only (`docs/syndicate-reference-harness.md`). These
notes keep that boundary. Every candidate below names a Molten surface, the
evidence that would prove it, and what it does not prove.

Citations use `SYNIT-FILE → Heading`, where `SYNIT-FILE` is a page under
`synit-book/pages/`.

## 1. What Molten already has

Verified by reading the cited sources.

| Mechanism | Molten surface | Evidence |
|---|---|---|
| Atomic turns with staged actions and rollback | `src/runtime/dataspace/state.rs` (`begin_turn`, `commit_turn_with_predicate_receipt`, `rollback_turn`) | Turn actions stay invisible until commit; Trellis predicate receipt gates the commit |
| Assertions and retractions | `src/runtime/turn/mod.rs` `RuntimeAssertion` | Keyed by `(actor, value)`; a `value-ref` is bound to each record |
| Owner-scoped cleanup | `RuntimeState::cleanup_actor_scope`, `ReferenceHarness::cleanup_actor_scope` | Cleanup returns sorted assertion, observer, and message refs |
| `Observe` subscriptions | `LocalAdapter::observe_pattern`, `stage_step` (`Step::Observe`) | An observer receives current matches and later matches |
| Bounded pattern subset | `src/runtime/predicates/parts/mod/p000/body.rs` `RuntimePattern` | `Exact` and `Wildcard { binding }` only; unknown AST forms deny |
| Capability-scoped admission | `src/runtime/admission/mod.rs`, `src/runtime/spine/parts/p001/body.rs` | Basalt/UCAN authority input must bind holder, session, context, request, and verified grants |
| Gatekeeper resolver | `src/authority/parts/mod/p001/body.rs` `gatekeeper_resolve_live_ref` | Credential resolves to a scoped live ref with expiry and evidence refs |
| Revocation cleanup | `cleanup_for_revocation` | Assertions bound to a revoked authority ref are removed and receipted |
| Service lifecycle FSM | `src/lifecycle/parts/mod/p000/body.rs` `State` | `declared`, `spawning`, `starting`, `ready`, `degraded`, `stopping`, `stopped`, `failed`, `restarting`, `cleaned` |
| Service dependency evidence | `src/runtime/predicates/parts/mod/p004/body.rs` | Demanded refs, force-run refs, readiness refs, and a dependency-subset check |
| Coordination state as assertions | `src/coordination/parts/mod/p000/body.rs`, `.cairn/specs/coordination/spec.md` | Status assertions follow committed coordination results |
| Coordination retention records | `.cairn/specs/dataspace-access-cache/spec.md` | Bounded advisory cache with no authority change |

Two facts matter for the rest of this document.

1. The product runtime keeps assertions in an ordered set of `(actor, value)`
   pairs (`src/runtime/dataspace/state.rs`), while the reference harness keeps
   them in a `syndicate::bag::BTreeBag` (`src/runtime/dataspace/syndicate.rs`).
   The two disagree on duplicate values.
2. The trace vocabulary has no causality field. `RuntimeEvent`
   (`src/runtime/turn/chronicle.rs`) records what happened, not why the turn ran.

## 2. Synit rules with a real Molten gap

Value first, risk second. Each candidate names a consumer surface, so a
candidate without a consumer stays open instead of becoming infrastructure.

### C1. Equal assertions coalesce for observers

**Synit rule.** "assertion of a value is idempotent: multiple assertions of the
same value appear to observers indistinguishable from a single assertion"
(`07-syndicated-actor-model.md → Dataspaces`). The dataspace pseudocode notifies
subscribers only on the transition into and out of "one or more owners"
(`07-syndicated-actor-model.md → Dataspaces`, add/retract pseudocode).

**Molten gap.** `RuntimeState::stage_step` emits `Event::AssertionObserved` once
per asserting owner for every match, and `RuntimeAssertion` includes `actor` in
its identity. Two actors that assert an equal value therefore produce two
observer notifications. The reference harness uses bag counting and does not.
No test covers duplicate assertions (`src/runtime/dataspace/tests.rs`).

**Why it matters.** A subscriber that treats each notification as a state change
sees phantom churn, and a retraction by one owner can look like a full
withdrawal. The parity harness cannot detect this, because only the harness has
the bag rule.

**Evidence to prove it.** Positive: two owners assert the same value, an
observer sees one notification; one owner retracts, the value stays visible with
no second notification; the last owner retracts, one retraction arrives.
Negative: distinct values still notify separately; a retraction of a value that
no owner holds denies or is ignored before commit.

**Non-claims.** Coalescing changes observation only. It does not change
ownership, retention, or authority.

### C2. Facets as nested owner scopes

**Synit rule.** A facet is a conversation frame. It owns entities and
assertions, which "share its fate". A stopped facet never runs again. Stopping a
facet stops subfacets, retracts its assertions, and then runs stop handlers in
order, child before parent. Crash skips stop handlers
(`05-glossary.md → Facet`).

**Molten gap.** `docs/architecture.md` states that the public runtime model
includes facets, but no facet type exists in `src/`. The only occurrences are a
traceability comment in the Syndicate harness. Owner scopes are flat actor
strings with one `cleanup_actor_scope` step, so stop ordering, stop permanence,
and separate crash behavior are absent.

**Why it matters.** System extensions, plugin hosts, and vat objects already
need nested scopes. A flat scope means a child failure can retract a parent's
assertions, and an orderly stop is indistinguishable from a crash.

**Evidence to prove it.** Positive: stopping a child facet retracts only the
child's assertions, in child-before-parent order. Negative: a stopped facet
never restarts; a crash path performs no stop handler; a handler that asserts
during stop is denied or retracted before commit.

**Non-claims.** Facets are an ownership and lifetime structure. They do not
change turn scheduling or grant authority.

### C3. Introduction before reference at message boundaries

**Synit rule.** "messages MUST NOT embed any reference not previously known to
the peer (a 'transient reference')"; a reference is introduced by an assertion.
A relay that receives such a message must terminate the session with an error
(`08-protocol.md → Membranes`).

**Molten gap.** `docs/syndicate-reference-harness.md` disclaims wire
compatibility, and the remote dataspace path validates declared content refs
before delivery (`molten.iroh_sam_dataspace.content_ref_validation`). No rule
states that a message may not introduce an unknown ref, and no negative test
exists for an unknown ref in a message body.

**Evidence to prove it.** Positive: a message that references a ref established
by an earlier assertion is admitted. Negative: a message that introduces an
unknown ref denies before delivery; the denial names the unknown ref; no
partial state commits.

**Non-claims.** This is a protocol-shape rule. It proves nothing about delivery,
completeness, or crash consistency.

### C4. Attenuation caveats beyond allow and deny

**Synit rule.** Attenuation is a caveat chain applied in reverse order, newest
caveat last. A caveat yields a rewritten value or no value. Rejection is silent.
An unknown caveat rejects all inputs. Rewrite validity requires every template
ref to be bound by the matching pattern (`08-protocol.md → Attenuation of
authority`, `08-protocol.md → Validity of Caveats`).

**Molten gap.** `.cairn/specs/runtime-spine/spec.md`
(`molten.runtime_spine.capability_attenuation`) admits scoped allow and deny
authority only, and states that "rewrite/filter transforms require explicit
future rule evidence". Plugin grants attenuate by resource refs and evaluation
turn (`src/plugin/parts/host/p001/body.rs`), which is not a caveat chain.

**Prerequisite.** UCAN and Basalt own authority in this stack. Check whether the
UCAN caveat model already expresses a bounded filter over Preserves payloads. If
it does, Molten consumes it. If it does not, the decision to add a second
authority format needs its own review.

**Evidence to prove it.** Positive: a rewrite rule that matches rewrites the
payload and a rule that does not match rejects. Negative: an unknown caveat
rejects everything; an unbound template ref fails validation; caveat order is
deterministic.

**Non-claims.** Attenuation proves a restriction over the supplied rule
language. It does not prove that the delivered work was correct.

### C5. A bounded record pattern with minimum-match semantics

**Synit rule.** Group patterns check the type of the input, then match declared
key positions in increasing order. A candidate with more structure than the
pattern requires still matches, so a newer producer can add fields; a candidate
with less structure fails (`38-protocols__syndicate__dataspacePatterns.md →
Group`, `→ Example`). Bindings are numbered, and the visit order is
deterministic because Preserves dictionaries sort their keys
(`38-protocols__syndicate__dataspacePatterns.md`, note 2).

**Molten gap.** `RuntimePattern` supports exact values and one wildcard binding
only. The spec explicitly defers "record, array, dictionary, conjunction,
negation, or extensible compound matching" to a future admitted extension.

**Why it matters, with a caution.** Minimum-match is forward compatible, and it
is dangerous near identity. A pattern that ignores extra fields must never feed
an identity computation.

**Evidence to prove it.** Positive: a record with an extra field matches a
declared record pattern and the used bindings are recorded. Negative: a missing
or mistyped declared field denies; a pattern outside the admitted subset denies
before it controls routing; pattern routing never changes a canonical value ref.

**Non-claims.** Pattern admission proves shape acceptance only, not semantic
compatibility with the newer producer.

### C6. Turn causality in the trace vocabulary

**Synit rule.** Each trace entry records one activation, and each turn records a
`cause`: a prior turn, `cleanup`, `linkedTaskRelease`, `periodicActivation`,
`delay`, or `external` (`42-protocols__syndicate__trace.md → Turn causes`). The
action taxonomy includes `spawn`, `link`, `facetStart`, `facetStop`,
`enqueue`, and `dequeue` (`42-protocols__syndicate__trace.md → Turn action
descriptions`).

**Molten gap.** `RuntimeEvent` has no cause field and no spawn, link, or facet
action kinds. `TraceEvidence` in the reference harness binds event refs and a
replayability status, but not causality.

**Why it matters.** "Why did this turn run?" and "what did this failure
release?" need an edge, not a log line. Molten already records the committed
facts; the missing part is the reason.

**Evidence to prove it.** Positive: a dependency-driven turn names the turn that
released it; a cleanup turn names `cleanup`. Negative: an action without a valid
cause fails validation; a truncated trace fails closed; an unknown action kind
is rejected.

**Non-claims.** A trace proves the recorded causal order only. It does not prove
that the trace is complete or that any action succeeded.

### C7. Service state as a union of assertions

**Synit rule.** `State = =started / =ready / =failed / =complete / @userDefined
any`. "The overall state of the service is the union of asserted `state`s"
(`12-operation__service.md → Convey the current state of a service`). `ready`
implies `started`; `complete` reports a one-shot program that finished; the
config layer derives an `up` alias from `ready` or `complete`
(`19-operation__synit-config.md → Synthesis of service state "up"`).

**Molten gap.** `src/lifecycle/parts/mod/p000/body.rs` models one state value
from a ten-variant enum. System extensions use a separate phase vocabulary
(`starting`, `ready`, `draining`, `failed`, `stopped`), and production readiness
uses another. There is no `complete` state for a one-shot service, and no
derived `up` alias.

**Why it matters.** Three vocabularies describe the same question. A dependent
service that waits for `ready` cannot express "finished successfully", and a
single-valued state cannot hold "started and ready" at once.

**Evidence to prove it.** Positive: a one-shot service reports `complete` after
a normal exit; a dependent acts on `ready` and on `complete` through `up`.
Negative: a process that exits before readiness reports `failed`, not `ready`;
an unknown state value is user-defined and cannot satisfy a `ready` dependency.

**Non-claims.** Declared state describes the component's own observation. It
does not prove output quality.

### C8. Demand and force-run as assertions

**Synit rule.** `require-service` starts a service after its dependencies are
satisfied and asserts `run-service` for it. `run-service` starts immediately. A
service shuts down when the last `run-service` assertion is withdrawn
(`12-operation__service.md → Details`). `restart-service` is a message, not an
assertion.

**Molten gap.** `evaluate_service_dependencies`
(`src/runtime/predicates/parts/mod/p004/body.rs`) already distinguishes demanded
refs from force-run refs and checks dependency subsets. Demand is an input list
to a predicate, not a retractable assertion, so demand has no observable
lifetime.

**Why it matters.** A withdrawable demand record makes capacity and shutdown
requests visible, and retraction becomes the shutdown signal instead of a
control message.

**Evidence to prove it.** Positive: withdrawing the last demand assertion allows
shutdown; a dependent starts only when its dependency is ready. Negative: a
force-run request bypasses dependency order by explicit declaration only; an
unknown or unverified dependency ref never satisfies demand.

**Non-claims.** Demand states a requirement. It does not prove that a provider
exists or that placement is optimal.

### C9. Digest-named records for durable declared state

**Synit rule.** Each user setting is one file named after the digest of the
canonical form of its assertion, and a rewriting config watcher wraps each
assertion into the config dataspace (`19-operation__synit-config.md → User
settings`).

**Molten gap.** Molten stores durable declared state in aggregate files and
databases rather than one record per declared fact. The access cache and the
retention records are the closest surfaces; the pattern is not used.

**Why it matters.** One record per fact makes removal a single-file deletion,
keeps unrelated writers intact, and makes content identity equal to file
identity. Use BLAKE3 for the digest, not SHA-1.

**Evidence to prove it.** Positive: two writers add records concurrently and
both survive; a removed record retracts exactly one fact. Negative: a malformed
record denies; a record whose name does not match its content denies; duplicate
records deduplicate.

**Non-claims.** A record proves declared interest. It does not prove that the
tool acted on it.

### C10. Ownership of remote assertions

**Synit rule.** Assertions never outlive their asserting actor, and the
dataspace forwards the withdrawal to subscribers
(`07-syndicated-actor-model.md → Dataspaces`, `05-glossary.md → Retraction`).

**Molten gap, stated as a question.** The remote dataspace path applies a
delivered envelope through the local turn boundary
(`docs/architecture.md`, `molten.iroh_sam_dataspace.*`). No source states which
owner holds a remote assertion, and no cleanup path exists on peer disconnect
(`src/remote/parts/dataspace/` has no disconnect or session-close cleanup).

**Why it matters.** Either the local receiver owns the assertion, in which case
disconnect is not a retraction and the fact is stale until explicit cleanup, or
the session owns it, in which case the disconnect path must retract it. Both are
defensible. Leaving it unnamed is not.

**Evidence to prove it.** Positive: whichever rule the review picks, a
disconnect test shows the documented outcome. Negative: a stale session never
keeps a live assertion after cleanup runs.

**Non-claims.** This rule shapes protocol behavior. It does not prove remote
delivery.

## 3. Rules Molten already follows

Keep these, and cite them rather than re-litigating them.

- **No protocol-level failure feedback.** Synit discards on failure because a
  live peer cannot distinguish slow from failed
  (`07-syndicated-actor-model.md`, note 6). Molten already denies before side
  effects and publishes non-claims instead of delivery promises
  (`docs/syndicate-reference-harness.md`, `.cairn/specs/delivery/spec.md`
  "No exact-once network claim"). Keep delivery receipts out of authority.
- **Retraction as the failure signal.** A crashed owner's assertions must
  disappear, and that disappearance is the signal. `cleanup_actor_scope` and the
  revocation cleanup path implement this.
- **Transport and receipts are not authority.** Iroh identity, tickets, and
  receipts grant nothing by possession
  (`docs/architecture.md`, `.cairn/specs/runtime-spine/spec.md`). This matches
  the Synit rule that a bearer capability is a secret, not a permission.
- **Turn atomicity.** `molten.runtime_spine.turn_semantics` already forbids
  visible pre-commit side effects.
- **Bounded, deterministic matching.** `molten.runtime_spine.preserves_patterns`
  already denies unsupported pattern forms before they control routing.

## 4. What not to take

- **Do not adopt the Syndicate wire protocol, relays, or OID membranes as a
  compatibility surface.** Molten has its own envelope, receipt, and transport
  boundaries. The membrane *rule* is worth taking (C3); the wire format is not.
- **Do not replace entity-level addressing.** Synit addresses entities; Molten
  addresses actors and services with explicit string targets. Moving to
  entity-level addressing is a large change with no named consumer. Take facet
  scoping (C2) without the addressing rewrite.
- **Do not add sturdyref HMAC construction.** UCAN and Basalt own authority and
  delegation. Molten keeps a resolver-shaped gatekeeper over its own verified
  inputs.
- **Do not move the batch or evidence paths onto reactive state.** Molten's
  receipts, signed content refs, and replay logs stay immutable and canonical.
  Reactive facts belong to the coordination plane, exactly as ADR 0080 decided
  for the sibling Mantle repository.
- **Do not adopt the Synit configuration scripting language.** Molten keeps
  typed Nickel contracts and versioned Preserves records.

## 5. Open questions

1. Which subsystem needs facet-scoped ownership first: system extensions,
   plugin hosts, or the vat? C2 needs one.
2. Who owns a remote assertion, and what ends its life (C10)?
3. Does the UCAN caveat model cover bounded payload filters? If yes, C4 becomes
   an adoption task; if no, C4 needs its own authority review.
4. Which consumer breaks on exact-only patterns today (C5)? Without one, C5 is
   speculative.
5. Should the trace vocabulary take the full Synit action taxonomy, or only the
   cause field (C6)?

## 6. Candidate summary

| Candidate | Molten surface | Type | Blocked on |
|---|---|---|---|
| C1 equal-assertion coalescing | `src/runtime/dataspace/state.rs` | Behavior fix | None |
| C2 facet owner scopes | `src/runtime`, `src/lifecycle` | Design | Consumer choice |
| C3 introduction-before-reference | `src/remote/parts/dataspace` | Rule and tests | None |
| C4 caveat attenuation | `src/authority`, plugin grants | Design | UCAN review |
| C5 bounded record patterns | `RuntimePattern` | Extension | Named consumer |
| C6 turn causality | `src/runtime/turn/chronicle.rs` | Extension | Scope choice |
| C7 service-state union | `src/lifecycle`, `src/system_extension` | Design | Vocabulary owner |
| C8 demand assertions | service dependency predicate | Design | None |
| C9 digest-named records | node state, retention records | Design | Consumer |
| C10 remote assertion ownership | `src/remote/parts/dataspace` | Decision | Review |

The tracey baseline already classifies these runtime-spine requirements as
`accepted-implementation-unestablished`
(`evidence/tracey/inherited-debt-classification.tsv`). C1 maps to
`molten.runtime_spine.assertion_lifetimes` and `.observe_patterns`; C3 and C10
map to `.reference_lifetimes`; C6 maps to `.interaction_tracing`. Those
candidates close accepted requirements instead of opening new ones.

## 7. Change packages

Reviewing this document produced one Cairn change package per candidate under
`.cairn/changes/`. All tasks are open. Creating a package is not acceptance, and
no package implements anything.

| Candidate | Change package | Delta spec |
|---|---|---|
| C1 | `coalesce-assertion-notifications` | `runtime-spine` |
| C2 | `admit-facet-owner-scopes` | `runtime-spine` |
| C3 | `enforce-introduction-before-reference` | `runtime-spine` |
| C4 | `review-caveat-attenuation-boundary` | `runtime-spine` |
| C5 | `admit-record-pattern-subset` | `runtime-spine` |
| C6 | `record-turn-causality-in-traces` | `runtime-spine` |
| C7 | `unify-service-state-assertions` | `runtime-spine` |
| C8 | `publish-withdrawable-service-demand` | `runtime-spine` |
| C9 | `store-declared-state-as-digest-named-records` | `durable-state-ports` |
| C10 | `own-remote-assertions-per-session` | `runtime-spine` |

C2, C5, and C9 carry a consumer decision as their first task, because each one
needs a named user surface before implementation. C4 is review-only and records
a rule instead of a filter implementation.
