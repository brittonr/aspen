# Design: harness generic replay gate

## Gate evidence

`gate_check_report` already runs validation and deterministic replay. This slice converts that replay result into a canonical `deterministic-replay-verify-v1` value and stores its ref on `GateCheck`.

`gate_receipt_value` embeds the generic replay receipt under the existing `replay` block and adds its content ref to `artifact-refs` with kind `deterministic-replay-verify`.

## Validation

`parse_gate_receipt` must parse the replay block, validate that the embedded generic replay receipt:

- has schema `molten.determinism.replay-verify.v1`;
- has pass decision;
- binds the same expected report, actual report, and final-state refs as the gate replay block;
- reports no divergence.

The existing harness replay comparison remains authoritative for constructing the gate receipt. The embedded generic receipt is a canonical evidence boundary for downstream tools.

## Boundaries

Generic replay evidence is evidence-only. It does not replace report validation, policy/capability/resource checks, chain evidence, turn journals, or source-gate evidence.
