## Context

Existing remote dataspace envelopes move canonical messages/assertions between peers. Trellis dependencies are present and already used in job DAG/evidence predicates. The missing slice is a protocol manifest and endpoint interpreter: users should install a finite choreography, project local endpoints, and ensure every send/receive/choice respects the projected state.

## Goals

- Define `protocol-manifest-v1`, `protocol-install-receipt-v1`, `protocol-endpoint-v1`, `protocol-session-state-v1`, `protocol-message-v1`, and `protocol-operation-receipt-v1`.
- Implement deterministic registries mapping manifest role/label/payload names to Trellis role ids, label ids, and payload tags.
- Compile the manifest to Trellis `GlobalChoreo`, run projectability checks, and project each role to `LocalChoreo`.
- Interpret local endpoint actions over Molten dataspace messages and remote dataspace envelopes.
- Enforce session sequence numbers, payload schema refs, expected local action, branch label admissibility, and replay windows.
- Emit receipts for install, send, receive, internal choice, offer, session close, denial, and full lifecycle gate replay.

## Non-Goals

- No ChoRus compatibility target.
- No Raft for ordinary protocol messages.
- No unbounded protocol recursion.
- No implicit payload schema from debug strings.
- No transport-specific semantics; Iroh carries envelopes only.

## Records

```preserves
<protocol-manifest-v1 "molten.protocol.manifest.v1"
  <protocol-id "proto:request-response">
  <roles ["client" "server"]>
  <labels ["request" "response"]>
  <payloads [<payload "request" <schema-ref>> ...]>
  <global <trellis-global-choreo-ref-or-record>>
  <policy [<policy-ref> ...]>
  <capability [<authority-context-ref> ...]>
  <resource [<resource-ref> ...]>
  <checks [<check "finite-protocol" "pass"> ...]>>
```

```preserves
<protocol-message-v1 "molten.protocol.message.v1"
  <protocol <protocol-ref>>
  <session <session-id>>
  <from-role "client">
  <to-role "server">
  <label "request">
  <payload-tag "request">
  <body-or-ref <preserves-value-or-content-ref>>
  <sequence 0>
  <evidence [<receipt-ref> ...]>
  <checks [<check "projected-action" "pass"> ...]>>
```

```preserves
<protocol-session-gate-receipt-v1 "molten.protocol.session-gate-receipt.v1"
  <decision "pass">
  <install <install-receipt-ref>>
  <protocol <manifest-ref>>
  <sessions ["session:request-response"]>
  <initial-states [<state-ref> ...]>
  <operations [<operation-receipt-ref> ...]>
  <messages [<message-ref> ...]>
  <final-states [<terminal-state-ref> ...]>
  <diagnostics []>
  <checks [<check "install-replay" "pass"> ...
           <check "protocol-session-gate-is-not-authority" "pass">]>>
```

## Interpreter Algorithm

1. Install manifest: canonicalize, build registries, lower to Trellis, check projectability, project endpoints, emit install receipt.
2. Start session: bind protocol ref, role endpoint ref, participants, policy/capability/resource refs, and initial local state.
3. Send: verify local endpoint expects a send/choice, validate payload schema, emit protocol message, advance local endpoint state, publish via dataspace.
4. Receive: verify message matches current endpoint offer/receive state, sequence/replay window, payload tag/schema, and participant mapping; then advance state.
5. Branch: require the selected label is one of the projected offers and bind selection evidence.
6. Close: emit session close receipt when all endpoints reach terminal local states.
7. Lifecycle gate: parse the install receipt, initial states, operation receipts, protocol messages, and next states; replay install and passing operations; deny if bindings diverge or roles fail to reach terminal state.

## Replay and Diagnostics

Replay stops at the first divergent protocol boundary: manifest ref, endpoint projection ref, local state ref, message ref, sequence number, selected label, payload ref, admission decision, resulting local state ref, or lifecycle gate terminal-state evidence. Gate receipts are operational evidence only and do not grant authority, resource rights, provenance, policy admission, or transport trust.
