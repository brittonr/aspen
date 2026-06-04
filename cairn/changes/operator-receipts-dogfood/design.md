## Context

Molten already treats receipts as evidence, but operators need a coherent workflow. Aspen demonstrates a useful pattern: a full dogfood run writes operator-visible receipts and allows later readback. Molten should make this a core confidence rail aligned with Cairn, deterministic playback, and executable transcripts.

## Goals

- Provide a repeatable local dogfood workflow that exercises core runtime boundaries.
- Emit durable, inspectable receipts at every important trust boundary.
- Allow operators and CI to list/show/export/validate receipts.
- Publish a final receipt summarizing pass/fail, artifacts, dependency closure, initial/final state hashes, handler profile, and trace refs.
- Integrate with deterministic replay and transcript artifacts.

## Non-Goals

- Do not require a production cluster for the first dogfood workflow.
- Do not replace detailed traces with one final receipt.
- Do not make logs the primary evidence format.
- Do not use dogfood success as proof that all policies are correct.

## Dogfood workflow

A minimal workflow should include:

1. Load Nickel config and static policy.
2. Resolve persistent node identity.
3. Start a local deterministic runtime.
4. Install one or more content-addressed artifacts.
5. Bind a local handler profile.
6. Run two native actors through dataspace assertions/messages.
7. Exercise typed storage or receipt store.
8. Run an executable transcript.
9. Produce final state hash and success/failure receipt.
10. Clean up or leave state running according to command option.

## Receipt shape

Operator receipts should include:

- run id,
- command/profile,
- artifact and dependency closure refs,
- config/policy refs,
- node identity ref,
- initial/final state hashes,
- trace refs,
- child receipt refs,
- status and failure classification,
- first-divergence info if replay was involved,
- redaction/confidentiality metadata.

## CLI surface

Initial commands:

```text
molten dogfood local
molten dogfood local --leave-running
molten receipts list
molten receipts show <run-id>
molten receipts validate <run-id>
molten receipts export <run-id>
```

Later, cluster-backed readback can read receipts from Raft/control-plane storage.

## Policy and confidentiality

Receipt export is policy-gated. Secret fields are redacted by default. Dogfood may run in deterministic local mode or record mode; production effects require explicit admission.

## Open Questions

- What is the smallest vertical slice that is meaningful enough for first dogfood?
- Should final receipts be stored in Redb only first, or also as chunk-store objects?
- Which receipt schemas should be stable before CLI export is considered public?
