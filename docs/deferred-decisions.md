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

## #3 bbtree transient-index evaluation

**Decision: keep the current BTreeMap index; no bbtree dependency.**

The published `bbtree` versions inspected (0.1.5 through 0.2.1) pin
`vstd = "=0.0.0-2026-08-30-0159"`. The workspace's `verified-logic` dependency
pins `vstd = "=0.0.0-2026-01-25-0057"`. Cargo rejects the two exact pre-release
versions in one graph, so a benchmark cannot compare bbtree against the current
planner without a separate workspace, a different Verus cohort, or a patch that
would change the evaluated source.

Reopen with a bbtree release or pinned revision that is compatible with the
workspace's verified-logic cohort, or with an isolated benchmark workspace that
retains the exact source/toolchain evidence. Until then, the canonical edit
path, profile identity, roots, and storage behavior stay unchanged.

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

## #9 TPM2-sealed node identity key

**Decision: no `Tpm2Sealed` class or loader yet.**

`ManagedSecret` does not cover this gap today. It appears in the profile's
eligible classes and in a redaction fixture, but no loader fronts
systemd-creds, a TPM, or another secret manager; the only implemented live
loader is the capability-file adapter. Adding the class without a loader would
overstate what is available.

The remaining work needs a swtpm-backed PCR-policy seal/unseal test, a typed
fail-closed denial on an altered PCR state, and rotation between
`CapabilityFile` and the new class through the existing rotation plan. Reopen
when that hardware or swtpm rail and the exact backend ref are available.

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
