# Deferred decision record

This page records no-go decisions for issues whose own acceptance criteria make
"keep the current design" or "decide later" a valid outcome. A no-go decision is
not a claim that the idea can never work; it names the missing prerequisite and
the trigger for reopening.

## #2 Tree-clock evaluation

**Decision: no tree-clock implementation or dependency.**

The prerequisite causal-turn validation has not produced a named recorded-
causality query, expected scale, time or memory budget, or evidence that an
explicit DAG or vector-clock baseline is insufficient. The inspected runtime
records ordered events without a cause field, so a tree-clock projection would
start from a representation the prerequisite has not validated.

Reopen when the prerequisite lands and one concrete diagnostic query names its
workload, bounds, baseline result, and acceptance threshold. Keep the DAG or
vector-clock baseline authoritative until then.

## #6 Live-frame hashing split

**Decision: keep the current single-step frame path.**

No profile output shows that `frame_ref` is a material fraction of owner-held
time. The current path also creates a thread and Tokio runtime per exchange,
which is a larger suspected cost. Splitting hashing into a `Send` begin step and
an owner-held commit step before measuring that path would add an API and stale-
pending state without evidence.

Reopen after runtime reuse or amortization lands and a profile at typical and
maximum frame sizes names a predeclared threshold. The canonical hash, pending,
acknowledgement, and uncertain-delivery semantics must remain unchanged.

## #10 Encrypted discovery provider

**Decision: no provider implementation.**

The issue is a design consideration, not an accepted requirement. There is no
agreed threat model for address hints, no membership-policy decision about which
already-admitted peers may learn a hint, and no provider or relay profile that
would carry the encrypted payload. Implementing a provider first would decide
those questions implicitly.

Reopen after a threat model, membership disclosure rule, hint lifetime, and
public bootstrap path are agreed. Address hints must remain non-authoritative.

## #12 Root-blocker explanations

**Decision: no explanation engine.**

No bounded world-workflow consumer asks for root-blocker explanations. The
existing operator workflows already report the first blocker and link receipts;
a general explanation engine would add projection, rule and budget surface
without a named query.

Reopen with a concrete workflow question, a bounded input snapshot, and a
comparison against the current first-blocker diagnostics.
