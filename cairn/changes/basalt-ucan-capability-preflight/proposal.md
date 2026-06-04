# Change: basalt-ucan-capability-preflight

## Why

Capability fixtures are now mandatory and deny by default, but capability-gate evidence still relied on local marker checks for Basalt authority context and UCAN proof review. Pass evidence should prove that the explicit authority context was bound to a preflight receipt before admission decisions or effects.

## What

- Replace marker-only capability gates with Basalt authority contract envelopes and preflight receipts.
- Bind capability gates to the canonical capability context ref, explicit local UCAN proofset ref, and canonical grant refs.
- Keep non-empty UCAN proofsets fail-closed until full proof validation is implemented.
- Validate admission authority evidence against the capability preflight grant-ref set.
- Add pass-evidence gate receipt checks and artifact refs for authority preflight, proofset binding, and grant-ref binding.

## Impact

Reports gain richer `<capability-gate-v1 ...>` evidence. Older marker-only capability gates no longer validate as evidence-bearing pass reports.
