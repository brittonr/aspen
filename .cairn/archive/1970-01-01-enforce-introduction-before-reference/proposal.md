# Proposal: Enforce introduction before reference

## Why

The manual states a hard rule for reference lifetimes: "messages MUST NOT embed any reference not previously known to
the peer (a 'transient reference')", a reference is introduced by an assertion, and a relay that receives such a
message must terminate the session with an error (`08-protocol.md → Membranes`).

Molten has the pieces but not the rule. The remote dataspace path validates declared content refs before delivery
(`molten.iroh_sam_dataspace.content_ref_validation`), peer sessions carry generation fences, and the accepted
requirement `molten.runtime_spine.reference_lifetimes` already requires session-scoped cleanup. No requirement states
that a message may not introduce a reference, no code derives the introduced set from live assertions, and no negative
test covers an unknown ref inside a message body. The tracey baseline lists the related runtime-spine requirements as
accepted and implementation-unestablished.

## What Changes

- Derive the introduced reference set for a session from that session's live assertions, plus the session's
  established bootstrap references. r[molten.runtime_spine.reference_introduction_rule]
- A message that carries a reference outside the introduced set MUST deny before delivery, and the denial MUST name
  the unknown reference.
- Removing the last assertion that introduced a reference MUST withdraw the introduction, so a later message that
  carries it denies again.
- A sender-side envelope build SHOULD refuse to emit a message with an unintroduced reference.
- Keep the peer-visible surface unchanged: Molten records denial evidence instead of inventing a protocol-level error
  reply, consistent with the FLP rationale in the manual (`07-syndicated-actor-model.md`, note 6).

## Impact

- **Files**: `src/remote/parts/dataspace/`, `src/runtime/dataspace/state.rs`, `docs/architecture.md` remote dataspace
  section, `docs/syndicate-reference-harness.md` boundary note.
- **Testing**: positive case where an assertion introduces a ref and a later message carries it; negative cases for an
  unknown ref, for a ref whose introducing assertion retracted, and for sender-side refusal; session close removes the
  introduction set.
- **Non-goals**: no Syndicate wire compatibility, no relay or membrane implementation, no delivery-completeness claim,
  and no authority change.
