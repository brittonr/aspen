# Proposal: Review the caveat attenuation boundary

## Why

The accepted requirement `molten.runtime_spine.capability_attenuation` admits scoped allow and deny authority only and
defers rewrite and filter transforms until "explicit future rule evidence" exists. The manual supplies the missing
design: a caveat chain evaluated in reverse order, where a caveat yields a rewritten value or no value, rejection is
silent, and an unknown caveat rejects every input (`08-protocol.md → Attenuation of authority`). Rewrite validity also
requires every template reference to be bound by the matching pattern (`08-protocol.md → Validity of Caveats`).

Molten has real attenuation needs and no filter language for them: plugin hostcall grants attenuate by resource refs and
evaluation turn, vat proxies are documented as narrowing or rewriting payloads, and remote tickets scope a session. UCAN
and Basalt already own authority in this stack, and UCAN carries its own caveat vocabulary. Adding a second authority
format without a recorded decision would duplicate the authority boundary that the review notes in
`docs/synit-incorporation-review.md` deliberately keep singular.

This package is a review and a boundary rule. It implements no filter language.

## What Changes

- Record the inventory of Molten attenuation needs: the caller, the resource, the narrowing that is required today, and
  whether the narrowing is a scope restriction or a payload filter.
  r[molten.runtime_spine.caveat_authority_boundary]
- Record whether the pinned UCAN model already expresses the required bounded filter over Preserves payloads. If it
  does, Molten consumes it. If it does not, the decision names the gap and the reason a Molten-owned form is or is not
  justified.
- Prohibit a Molten-owned caveat or filter format before that recorded decision exists.
- If any Molten-owned form is later admitted, fix its semantics in advance: unknown caveat rejects everything, caveat
  order is deterministic and documented, and an unbound template reference fails validation.

## Impact

- **Files**: `docs/synit-incorporation-review.md`, a new decision note under `docs/`, and the
  `.cairn/specs/runtime-spine/spec.md` requirement through this delta.
- **Testing**: document review only. No runtime behavior changes in this package, so no runtime tests are added here.
- **Non-goals**: no caveat implementation, no new token construction, no change to Basalt, UCAN, or plugin grant
  evaluation, and no authority widening.
