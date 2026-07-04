# Design: Syndicate dataspace reference harness

## Scope

This change introduces Syndicate as an executable reference/harness layer for Molten's local dataspace semantics. It does not replace Molten's envelope spine, authority gates, receipt model, or deterministic replay identity. The initial target is parity and evidence, not broad production routing replacement.

## Proof checklist

- **Proof claim**: the Syndicate-backed reference harness produces deterministic Molten evidence for adopted assertion, retraction, Observe, cleanup, attenuation, flow-control, and trace scenarios from canonical Preserves inputs.
- **Out of scope**: claiming Syndicate wire-protocol compatibility; treating Syndicate `Cap`, trace, account, relay, or sturdyref evidence as Molten authority; replacing Iroh or node-control transport semantics.
- **Trusted assumptions**: Syndicate internals are a library implementation detail behind Molten admission gates; Molten canonical Preserves values and BLAKE3 refs remain the normative boundary.
- **Positive evidence**: parity fixtures show the existing Molten dataspace and Syndicate harness agree on adopted equality, pattern, assertion, retraction, and cleanup cases.
- **Negative evidence**: missing authority, overbroad cap/attenuation, stale facet owner, non-replayable trace, and exhausted account budget deny or remain diagnostic-only before side effects.
- **Canonical refs**: input step ref, pattern ref, actor/facet owner ref, assertion ref, observer ref, Syndicate trace ref, account observation ref, parity receipt ref, and Molten route receipt ref.
- **Regeneration command**: focused runtime dataspace, Syndicate harness, resource backpressure, and trace evidence tests.

## Functional core

Pure core functions translate canonical Molten runtime steps and values into adapter-neutral dataspace commands, compare normalized outcomes, classify parity differences, and build receipt values from explicit trace/account inputs. They do not read files, network state, clocks, process state, environment variables, or scheduler state.

## Imperative shell

The shell owns Syndicate actor/dataspace instantiation, task scheduling, fixture loading, and receipt writing. It must record enough trace and account observations for deterministic comparison, or mark the evidence diagnostic-only when replay data is incomplete.

## Staged adoption

1. Reference harness only: run Syndicate side-by-side with current Molten local dataspace for small fixtures.
2. Parity gate: compare normalized canonical events, assertion ownership, Observe delivery, and retractions.
3. Feature gates: enable Syndicate-backed routing for adopted local-only surfaces after parity evidence exists.
4. Production consideration: require separate change packages before replacing durable node, remote, or transport-facing routing.

## Evidence boundary

Syndicate trace, flow-control accounts, capabilities, and facets inform Molten evidence but do not grant authority by themselves. Molten policy, authority, resource, provenance, source-gate, retention, and delivery-idempotency checks remain mandatory.

## Non-goals

- No global dataspace or global consistency claim.
- No direct acceptance of Syndicate relay or sturdyref protocol as Molten transport authority.
- No unbounded use of host scheduler timing in replay decisions.
