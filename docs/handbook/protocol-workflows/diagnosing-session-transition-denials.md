# Diagnosing session transition denials

Mode: Troubleshooting

Use this guide for choreography protocol operations, not transport-session admission or peer bootstrap. Begin with the [Handbook](../README.md) and keep the [transport session companion](../../technical/transport/session-admission-and-transitions.md) available when similarly named records appear. The procedures are source inspection and evidence triage; no runtime reproduction or test execution is claimed here.

Preserve the install receipt, prior state, attempted input, optional message, operation receipt, and any supplied next state. Keep their canonical references and source revision. Do not erase seen-message state, force a branch, or retry an uncertain external action merely because a local transition denied. A pure transition and an external effect have different failure boundaries.

## Symptom: no operation receipt was produced

**Discriminating evidence:** determine whether the call returned a parsing/validation error rather than a `ProtocolOperationRun` with decision `deny`. State and message parsers execute before several denial paths. Malformed record shapes, invalid references, or invalid names can therefore prevent a normal denial receipt. Startup also returns an error when installation denied or the role cannot be resolved.

**Safe next action:** retain the original bytes and identify the first failing parser using the [operation entry points](../../../src/protocol/parts/session/p002/body.rs). Confirm the artifact type and schema before interpreting it as state. Do not synthesize a passing install or replace an invalid reference with a plausible hash.

**Stop condition:** if the canonical input cannot be recovered from its owner, record an evidence gap. A summary string cannot reconstruct the missing state.

## Symptom: missing authority or resource evidence

**Discriminating evidence:** `admission_diagnostics` validates reference shapes, then checks empty authority and resource collections. It reports missing authority first; only after authority is present can missing resources become the returned diagnostic. These checks occur before send constructs a message.

**Safe next action:** compare the attempted operation's evidence collections with the actual admitted context. Obtain missing evidence from its legitimate producer. Do not treat a fixture-generated reference, transport identity, or receipt as a grant.

**Stop condition:** inability to establish the required current authority/resource context blocks an effectful continuation. The [admission helper](../../../src/protocol/parts/session/p011/body.rs) checks supplied references; its success does not prove every external authority policy has been evaluated.

## Symptom: send does not match the projected action

**Discriminating evidence:** inspect the first action in `prior.local_state.actions`. Send must match direction, peer, label, and payload tag. With no first action, the diagnostic is “endpoint does not expect send.” A supplied message is additionally checked for protocol, session, sender, recipient, sequence, label, and tag.

**Worked failure:** `wrong_label_and_missing_authority_deny_before_message` starts the request-response client's initial state, then attempts label/tag `response` to `server`. The [existing test](../../../src/protocol/parts/session/tests/m000/p000/body.rs) expects `deny` and no output message. The legal first edge is `request`; a valid authority collection cannot make `response` legal there.

**Safe next action:** recover the intended prior state and compare it with the recorded operation history. Correct a newly prepared input only when the workflow intent warrants it; do not alter historical state to make the request fit. Stop if two incompatible histories claim the same prior position.

## Symptom: receive mismatch or duplicate replay

**Discriminating evidence:** the [receive transition](../../../src/protocol/parts/session/p007/body.rs) checks whether the message reference is already in `seen_message_refs` before checking the next action. A duplicate diagnostic is therefore distinct from a wrong sequence. Otherwise matching requires protocol reference, session identifier, expected sender, local recipient role, label, payload tag, and sequence.

**Safe next action:** compare those fields independently and locate the earlier successful receive when duplicate identity is reported. Preserve both delivery observations if a shell supplied the same message twice. Never clear the seen set or increment sequence by hand.

**Stop condition:** if an earlier shell effect may have happened, reconcile its effect evidence before any retry. Duplicate detection in this retained local state is not an exactly-once external execution guarantee.

## Symptom: branch, offer, or next-state mismatch

**Discriminating evidence:** branch requires an internal-choice terminal; offer requires an offer terminal and a projected label. The low-level offer transition also checks a supplied peer against the offering role. A proposed next state is compared with protocol/session/role bindings, sequence advancement, derived local state, and seen-message references.

**Safe next action:** retain the projected endpoint and compare the selected label with its available branches. For next-state mismatches, obtain the actual transition output rather than constructing a guessed successor. The helper compares sequence with `saturating_add(1)`; do not infer an unlimited sequence space or a stronger overflow claim from diagnostic wording.

## Symptom: individual operations pass but lifecycle gating denies

**Discriminating evidence:** inspect missing prior/message/next references, install replay mismatch, and terminal trace diagnostics. The [gate](../../../src/protocol/parts/session/p004/body.rs) replays passing operations. The [trace walk](../../../src/protocol/parts/session/p005/body.rs) requires a unique passing successor until an empty-action `End` state, within its bound.

**Safe next action:** rebuild the evidence package from preserved canonical artifacts without changing their content. A missing artifact may be a packaging defect rather than a transition defect. Ambiguous successors require provenance review, not selecting whichever branch makes the gate pass. Denied receipts must have diagnostics and no next-state reference, but the denial branch is not the same replay path as passing operations.

Stop when evidence cannot establish a unique supplied trace. Report the narrow finding and unresolved scope; neither a repaired package nor a passing lifecycle gate establishes live transport, current authorization, or consensus membership.

## Sources

- [Handbook](../README.md)
- [Typed facade and effect boundary](../../choreography-typed-facade.md)
- [Transport sessions are a separate domain](../../technical/transport/session-admission-and-transitions.md)
- [Operation entry points](../../../src/protocol/parts/session/p002/body.rs)
- [Transition checks](../../../src/protocol/parts/session/p007/body.rs)
- [Admission and next-state checks](../../../src/protocol/parts/session/p011/body.rs)
- [Lifecycle operation checks](../../../src/protocol/parts/session/p004/body.rs)
- [Terminal trace diagnostics](../../../src/protocol/parts/session/p005/body.rs)
- [Existing wrong-label and missing-authority tests](../../../src/protocol/parts/session/tests/m000/p000/body.rs)
