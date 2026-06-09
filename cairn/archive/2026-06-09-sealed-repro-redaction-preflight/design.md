# Design: sealed repro redaction preflight

## Context

A sealed repro bundle embeds a report, suite, effect log, policy/capability/budget evidence, and receipts. That is exactly the material needed to reproduce a run, but it can also contain secrets. The initial redaction rail is intentionally conservative.

## Policy

The static redaction policy is encoded as:

`<redaction-policy-v1 "molten.harness.redaction-policy.v1" ...>`

The policy mode is `deny-sensitive-markers`. The canonical forbidden marker labels are:

- `secret`
- `confidential`
- `credential`
- `private`
- `encrypted-ref`

`encrypted-ref` is denied for now because encrypted refs need their own validation and reveal receipts before they can be treated as safe in pass artifacts.

## Gate

Export derives a `<redaction-gate-v1 "molten.harness.redaction-gate.v1" ...>` receipt with:

- pass decision;
- redaction policy ref;
- report ref;
- suite ref;
- scan-root ref;
- checks for canonical scan and each marker class.

The scanner walks the canonical Preserves report value and rejects forbidden record labels. This catches sensitive markers in embedded suites, steps, observations, effect logs, and report evidence.

## Validation

Sealed bundle parsing recomputes the canonical redaction policy and redaction gate from the embedded report. Bundle gate checks require redaction policy/gate refs to be present, so unsealed report bundles no longer satisfy pass evidence gates.

## Future work

Future slices can replace fail-closed denial with explicit redaction transforms, encrypted-ref validation, reveal receipts, and policy-governed export profiles.
