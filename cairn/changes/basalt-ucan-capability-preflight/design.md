# Design: Basalt/UCAN capability preflight

## Context

The harness authority frontend remains the Preserves `<capabilities-v1 ...>` grant fixture. Mandatory fixtures removed implicit authority, but the gate still needed executable preflight evidence rather than marker strings.

## Basalt authority boundary

For each capability context, the runner constructs a Basalt `ContractEnvelope` with backend `nickel`, contract id `molten.harness.capability-context`, version `v1`, the canonical capability context ref as normalized source hash, the admission request schema, the capability authorization receipt schema, and the Basalt authority preflight receipt schema. The runner validates this envelope before report generation.

The capability gate embeds:

- the canonical capability context ref,
- an authority contract envelope and envelope ref,
- a `<basalt-authority-preflight ...>` receipt with decision, backend, contract id, envelope ref, capability ref, proofset ref, ordered grant refs, and Basalt reason,
- an explicit `<ucan-proofset-v1 ...>` value.

The current local harness proofset is explicitly empty. Non-empty UCAN proof refs are rejected until full Basalt/UCAN proof validation is implemented.

## Grant and admission binding

Each grant in the capability fixture has a canonical grant ref. Capability validation recomputes the ordered grant-ref set from the embedded suite and rejects stale or tampered gate evidence. Admission authority evidence is still recomputed per step, and every authorized grant ref must be present in the capability preflight receipt's grant-ref set.

## Gate receipts

Pass-evidence receipts include artifact refs for the capability context, capability gate, authority preflight receipt, and UCAN proofset. Checks include `basalt-authority-receipt`, `capability-proofset-binding`, and `grant-ref-binding` in addition to explicit fixture and deny-by-default checks.
